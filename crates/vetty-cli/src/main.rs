use std::io::IsTerminal;
use std::path::PathBuf;

use std::process::Command;

use anyhow::{bail, Context, Result};
use clap::Parser;
use serde_json::Value;
use vetty_disk::DiskBuilder;
use vetty_vm::config::VmConfig;
use vetty_vm::VmInstance;

#[derive(Debug, Parser)]
#[command(name = "vetty", about = "Run untrusted code in a Firecracker sandbox")]
struct Args {
    #[arg(long)]
    dir: PathBuf,

    #[arg(long, default_value = "image/rootfs.ext4")]
    rootfs: PathBuf,

    #[arg(long, default_value = "image/vmlinux")]
    kernel: PathBuf,

    #[arg(long, default_value_t = 128)]
    memory: u32,

    #[arg(long, default_value_t = 1)]
    cpus: u8,

    #[arg(long, default_value = "firecracker")]
    firecracker: PathBuf,

    #[arg(long, default_value_t = false)]
    no_serial: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_target(false)
        .init();

    let args = Args::parse();
    validate_inputs(&args)?;

    let run_dir = std::env::temp_dir().join(format!("vetty-run-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&run_dir)?;
    let code_img_path = run_dir.join("code.img");

    tracing::info!("building code disk image from {}", args.dir.display());
    let disk_builder = DiskBuilder::default();
    let code_disk = disk_builder
        .build(&args.dir, &code_img_path)
        .context("failed to build code disk image")?;
    tracing::info!("code disk image created at {}", code_disk.display());

    let config = VmConfig {
        firecracker_bin: args.firecracker,
        kernel_path: args.kernel,
        rootfs_path: args.rootfs,
        code_disk_path: code_disk.clone(),
        vcpu_count: args.cpus,
        mem_size_mb: args.memory,
        ..Default::default()
    };

    if let Err(err) = setup_host_network() {
        tracing::warn!("Failed to setup host network automatically: {err}");
        tracing::warn!("Container internet access might not work. Please refer to FIRECRACKER.md to setup tap device manually.");
    }

    let mut vm = VmInstance::create(&config).await?;
    vm.start().await?;
    tracing::info!("vm started");

    let stdin_is_terminal = std::io::stdin().is_terminal();
    let attach_serial = should_attach_serial(args.no_serial, stdin_is_terminal);

    if !args.no_serial && !stdin_is_terminal {
        tracing::warn!("stdin is not a terminal; skipping serial console. Use --no-serial to silence this warning.");
    }

    if attach_serial {
        tracing::info!("attaching serial console (Ctrl+] to return to host shell)");
        vm.attach_serial()?;
    } else {
        tracing::info!("serial disabled; waiting for Ctrl+C");
        tokio::signal::ctrl_c().await?;
    }

    vm.kill()?;
    if code_disk.exists() {
        std::fs::remove_file(&code_disk)?;
    }
    let _ = std::fs::remove_dir_all(run_dir);
    Ok(())
}

fn should_attach_serial(no_serial: bool, stdin_is_terminal: bool) -> bool {
    !no_serial && stdin_is_terminal
}

#[cfg(test)]
mod tests {
    use super::should_attach_serial;

    #[test]
    fn serial_is_disabled_when_flag_is_set() {
        assert!(!should_attach_serial(true, true));
        assert!(!should_attach_serial(true, false));
    }

    #[test]
    fn serial_requires_a_terminal() {
        assert!(should_attach_serial(false, true));
        assert!(!should_attach_serial(false, false));
    }
}

fn validate_inputs(args: &Args) -> Result<()> {
    if !args.dir.is_dir() {
        bail!("--dir is not a directory: {}", args.dir.display());
    }
    if !args.rootfs.is_file() {
        bail!("--rootfs file not found: {}", args.rootfs.display());
    }
    if !args.kernel.is_file() {
        bail!("--kernel file not found: {}", args.kernel.display());
    }
    if args.cpus == 0 {
        bail!("--cpus must be greater than 0");
    }
    if args.memory < 64 {
        bail!("--memory must be at least 64");
    }
    Ok(())
}

fn setup_host_network() -> Result<()> {
    let tap_dev = "tap0";
    let tap_ip = "172.16.0.1";
    let mask_short = "/30";

    // Ignore errors for link del, as the interface might not exist
    let _ = Command::new("sudo")
        .args(["ip", "link", "del", tap_dev])
        .output();

    let user = std::env::var("USER").unwrap_or_else(|_| "root".to_string());
    let output = Command::new("sudo")
        .args([
            "ip", "tuntap", "add", "dev", tap_dev, "mode", "tap", "user", &user,
        ])
        .output()?;
    if !output.status.success() {
        bail!(
            "Failed to create tap device: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let output = Command::new("sudo")
        .args([
            "ip",
            "addr",
            "add",
            &format!("{}{}", tap_ip, mask_short),
            "dev",
            tap_dev,
        ])
        .output()?;
    if !output.status.success() {
        bail!(
            "Failed to assign IP to tap device: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let output = Command::new("sudo")
        .args(["ip", "link", "set", "dev", tap_dev, "up"])
        .output()?;
    if !output.status.success() {
        bail!(
            "Failed to bring up tap device: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let output = Command::new("sudo")
        .args(["sh", "-c", "echo 1 > /proc/sys/net/ipv4/ip_forward"])
        .output()?;
    if !output.status.success() {
        bail!(
            "Failed to enable IP forwarding: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let output = Command::new("sudo")
        .args(["iptables", "-P", "FORWARD", "ACCEPT"])
        .output()?;
    if !output.status.success() {
        bail!(
            "Failed to accept FORWARD traffic: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let output = Command::new("ip")
        .args(["-j", "route", "list", "default"])
        .output()?;
    if !output.status.success() {
        bail!(
            "Failed to list default routes: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let json: Value =
        serde_json::from_slice(&output.stdout).context("Failed to parse ip route JSON")?;
    let host_iface = json
        .get(0)
        .and_then(|r| r.get("dev"))
        .and_then(|d| d.as_str())
        .context("Could not find default host interface")?;

    // Ignore error for deletion
    let _ = Command::new("sudo")
        .args([
            "iptables",
            "-t",
            "nat",
            "-D",
            "POSTROUTING",
            "-o",
            host_iface,
            "-j",
            "MASQUERADE",
        ])
        .output();
    let output = Command::new("sudo")
        .args([
            "iptables",
            "-t",
            "nat",
            "-I",
            "POSTROUTING",
            "1",
            "-o",
            host_iface,
            "-j",
            "MASQUERADE",
        ])
        .output()?;
    if !output.status.success() {
        bail!(
            "Failed to add iptables masquerade: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    // Explicitly allow forwarding for our interfaces
    let _ = Command::new("sudo")
        .args([
            "iptables", "-D", "FORWARD", "-i", tap_dev, "-o", host_iface, "-j", "ACCEPT",
        ])
        .output();
    let _ = Command::new("sudo")
        .args([
            "iptables", "-I", "FORWARD", "1", "-i", tap_dev, "-o", host_iface, "-j", "ACCEPT",
        ])
        .output()?;
    let _ = Command::new("sudo")
        .args([
            "iptables", "-D", "FORWARD", "-i", host_iface, "-o", tap_dev, "-j", "ACCEPT",
        ])
        .output();
    let _ = Command::new("sudo")
        .args([
            "iptables", "-I", "FORWARD", "1", "-i", host_iface, "-o", tap_dev, "-j", "ACCEPT",
        ])
        .output()?;

    // Fix MTU black holes - Insert at the TOP of the mangle FORWARD chain instead of appending
    let _ = Command::new("sudo")
        .args([
            "iptables",
            "-t",
            "mangle",
            "-D",
            "FORWARD",
            "-p",
            "tcp",
            "--tcp-flags",
            "SYN,RST",
            "SYN",
            "-j",
            "TCPMSS",
            "--clamp-mss-to-pmtu",
        ])
        .output();
    let output = Command::new("sudo")
        .args([
            "iptables",
            "-t",
            "mangle",
            "-I",
            "FORWARD",
            "1",
            "-p",
            "tcp",
            "--tcp-flags",
            "SYN,RST",
            "SYN",
            "-j",
            "TCPMSS",
            "--clamp-mss-to-pmtu",
        ])
        .output()?;
    if !output.status.success() {
        bail!(
            "Failed to add iptables mangle rule for MTU clamping: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    Ok(())
}
