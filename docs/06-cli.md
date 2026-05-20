# Step 6 — CLI (`vetty-cli`)

## Goal
Build the main user-facing CLI binary that ties everything together: takes a `--dir` argument, builds the code disk image, launches the Firecracker VM, and manages the lifecycle.

---

## 6.1 Create the Crate

### `crates/vetty-cli/Cargo.toml`

```toml
[package]
name = "vetty-cli"
version.workspace = true
edition.workspace = true

[[bin]]
name = "vetty"
path = "src/main.rs"

[dependencies]
vetty-common = { path = "../vetty-common" }
vetty-disk = { path = "../vetty-disk" }
vetty-vm = { path = "../vetty-vm" }
anyhow = { workspace = true }
tracing = { workspace = true }
tracing-subscriber = { workspace = true }
tokio = { workspace = true }
clap = { version = "4", features = ["derive"] }
ctrlc = "3"
```

---

## 6.2 File: `crates/vetty-cli/src/main.rs`

### CLI Arguments

```rust
use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "vetty", about = "Run untrusted code in a sandboxed Firecracker VM")]
struct Args {
    /// Path to the directory containing the untrusted code
    #[arg(long)]
    dir: PathBuf,

    /// Path to the rootfs image
    #[arg(long, default_value = "rootfs.ext4")]
    rootfs: PathBuf,

    /// Path to the kernel (vmlinux)
    #[arg(long, default_value = "vmlinux")]
    kernel: PathBuf,

    /// Memory in MB
    #[arg(long, default_value_t = 128)]
    memory: u32,

    /// Number of vCPUs
    #[arg(long, default_value_t = 1)]
    cpus: u8,
}
```

### Main Flow

```rust
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::init();
    let args = Args::parse();

    // 1. Validate inputs
    if !args.dir.is_dir() {
        anyhow::bail!("--dir {:?} is not a directory", args.dir);
    }
    if !args.rootfs.exists() {
        anyhow::bail!("rootfs not found at {:?}", args.rootfs);
    }
    if !args.kernel.exists() {
        anyhow::bail!("kernel not found at {:?}", args.kernel);
    }

    // 2. Build the code disk image
    tracing::info!("Building code disk image from {:?}...", args.dir);
    let disk_builder = vetty_disk::DiskBuilder::default();
    let code_img_path = args.dir.with_extension("img"); // e.g., ./mycode.img
    let code_disk = disk_builder.build(&args.dir, &code_img_path)?;
    tracing::info!("Code disk image created at {:?}", code_disk);

    // 3. Configure and launch the VM
    let vm_config = vetty_vm::config::VmConfig {
        kernel_path: args.kernel,
        rootfs_path: args.rootfs,
        code_disk_path: code_disk.clone(),
        vcpu_count: args.cpus,
        mem_size_mb: args.memory,
        ..Default::default()
    };

    tracing::info!("Launching Firecracker VM...");
    let mut vm = vetty_vm::VmInstance::create(&vm_config).await?;
    vm.start().await?;
    tracing::info!("VM started. Attaching serial console...");

    // 4. Set up Ctrl+C handler for clean shutdown
    let (shutdown_tx, mut shutdown_rx) = tokio::sync::oneshot::channel();
    let shutdown_tx = std::sync::Mutex::new(Some(shutdown_tx));
    ctrlc::set_handler(move || {
        if let Some(tx) = shutdown_tx.lock().unwrap().take() {
            let _ = tx.send(());
        }
    })?;

    // 5. Attach to serial console (blocks until VM exits or Ctrl+C)
    tokio::select! {
        result = tokio::task::spawn_blocking(move || vm.attach_serial()) => {
            result??;
        }
        _ = &mut shutdown_rx => {
            tracing::info!("Shutting down...");
        }
    }

    // 6. Cleanup
    tracing::info!("Cleaning up...");
    // vm is dropped here, which kills the Firecracker process
    // Remove the temporary code disk image
    let _ = std::fs::remove_file(&code_disk);
    tracing::info!("Done.");

    Ok(())
}
```

---

## 6.3 Usage

```bash
# Basic usage
vetty --dir ./suspicious-npm-package

# With custom resources
vetty --dir ./code --rootfs ./images/rootfs.ext4 --kernel ./images/vmlinux --memory 256 --cpus 2
```

---

## Done Criteria

- [ ] `vetty-cli` compiles as a binary named `vetty`
- [ ] CLI parses `--dir`, `--rootfs`, `--kernel`, `--memory`, `--cpus`
- [ ] Validates input paths before doing anything
- [ ] Calls disk builder, then VM launcher, in correct order
- [ ] Ctrl+C triggers clean shutdown (kills VM, removes temp images)
- [ ] Drop-based cleanup ensures no zombie Firecracker processes
