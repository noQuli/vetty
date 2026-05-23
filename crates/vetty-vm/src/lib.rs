use std::fs::OpenOptions;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::Duration;

use anyhow::{bail, Context, Result};
use serde_json::json;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub mod config;
pub mod serial;

use config::VmConfig;

const FIRECRACKER_API_RETRIES: usize = 10;
const FIRECRACKER_API_RETRY_DELAY: Duration = Duration::from_millis(100);
const FIRECRACKER_STDERR_TAIL_LINES: usize = 40;

pub struct VmInstance {
    api_socket: PathBuf,
    vsock_uds: PathBuf,
    socket_dir: PathBuf,
    child: Child,
}

impl VmInstance {
    pub async fn create(config: &VmConfig) -> Result<Self> {
        validate_config(config)?;

        let run_id = uuid::Uuid::new_v4();
        let socket_dir = std::env::temp_dir().join(format!("vetty-{run_id}"));
        std::fs::create_dir_all(&socket_dir)?;

        let api_socket = socket_dir.join("firecracker-api.sock");
        let stderr_log = socket_dir.join("firecracker-stderr.log");
        let vsock_uds = config.vsock_uds_path.clone();
        if vsock_uds.exists() {
            let _ = std::fs::remove_file(&vsock_uds);
        }
        let mut child = spawn_firecracker(&config.firecracker_bin, &api_socket, &stderr_log)?;

        if let Err(err) = wait_for_socket(&api_socket).await {
            let diagnostics = firecracker_setup_diagnostics(&mut child, &stderr_log);
            let _ = cleanup_vm_process_and_files(&mut child, &api_socket, &vsock_uds, &socket_dir);
            let context = if diagnostics.is_empty() {
                "firecracker API socket did not become ready".to_string()
            } else {
                format!("firecracker API socket did not become ready{diagnostics}")
            };
            return Err(err.context(context));
        }

        let api = api_socket.display().to_string();
        if let Err(err) = configure_vm(&api, config, &vsock_uds).await {
            let diagnostics = firecracker_setup_diagnostics(&mut child, &stderr_log);
            let _ = cleanup_vm_process_and_files(&mut child, &api_socket, &vsock_uds, &socket_dir);
            let context = if diagnostics.is_empty() {
                "failed to configure firecracker VM via API".to_string()
            } else {
                format!("failed to configure firecracker VM via API{diagnostics}")
            };
            return Err(err.context(context));
        }

        Ok(Self {
            api_socket,
            vsock_uds,
            socket_dir,
            child,
        })
    }

    pub async fn start(&self) -> Result<()> {
        let api = self.api_socket.display().to_string();
        put_api(&api, "/actions", json!({ "action_type": "InstanceStart" })).await
    }

    pub fn vsock_uds_path(&self) -> &PathBuf {
        &self.vsock_uds
    }

    pub fn attach_serial(&mut self) -> Result<()> {
        let child_stdin = self
            .child
            .stdin
            .as_mut()
            .context("firecracker stdin is not piped")?;
        let child_stdout = self
            .child
            .stdout
            .as_mut()
            .context("firecracker stdout is not piped")?;
        serial::attach_serial(child_stdin, child_stdout)
    }

    pub fn kill(&mut self) -> Result<()> {
        cleanup_vm_process_and_files(
            &mut self.child,
            &self.api_socket,
            &self.vsock_uds,
            &self.socket_dir,
        )
    }
}

impl Drop for VmInstance {
    fn drop(&mut self) {
        let _ = self.kill();
    }
}

fn spawn_firecracker(bin: &Path, api_socket: &Path, stderr_log: &Path) -> Result<Child> {
    let stderr = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(stderr_log)
        .with_context(|| {
            format!(
                "failed to create firecracker stderr log at {}",
                stderr_log.display()
            )
        })?;

    let child = Command::new(bin)
        .arg("--api-sock")
        .arg(api_socket)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::from(stderr))
        .spawn()
        .with_context(|| format!("failed to spawn firecracker binary at {}", bin.display()))?;
    Ok(child)
}

async fn wait_for_socket(path: &Path) -> Result<()> {
    for _ in 0..120 {
        if path.exists() {
            return Ok(());
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    bail!(
        "firecracker API socket did not appear at {}",
        path.display()
    )
}

async fn put_api(socket_path: &str, endpoint: &str, body: serde_json::Value) -> Result<()> {
    api_request(socket_path, "PUT", endpoint, body).await
}

async fn patch_api(socket_path: &str, endpoint: &str, body: serde_json::Value) -> Result<()> {
    api_request(socket_path, "PATCH", endpoint, body).await
}

async fn api_request(
    socket_path: &str,
    method: &str,
    endpoint: &str,
    body: serde_json::Value,
) -> Result<()> {
    let payload = serde_json::to_vec(&body)?;
    let mut last_retryable_error = None;

    for attempt in 0..=FIRECRACKER_API_RETRIES {
        match api_request_once(socket_path, method, endpoint, &payload).await {
            Ok(()) => return Ok(()),
            Err(err) if attempt < FIRECRACKER_API_RETRIES && is_retryable_api_error(&err) => {
                last_retryable_error = Some(err);
                tokio::time::sleep(FIRECRACKER_API_RETRY_DELAY).await;
            }
            Err(err) => return Err(err),
        }
    }

    if let Some(err) = last_retryable_error {
        return Err(err.context(format!(
            "firecracker API request to {endpoint} kept failing after {} retries",
            FIRECRACKER_API_RETRIES
        )));
    }

    bail!("firecracker API request to {endpoint} failed without an error");
}

async fn api_request_once(
    socket_path: &str,
    method: &str,
    endpoint: &str,
    payload: &[u8],
) -> Result<()> {
    let mut stream = tokio::net::UnixStream::connect(socket_path)
        .await
        .with_context(|| format!("failed to connect to firecracker API socket at {socket_path}"))?;

    let request = format!(
        "{method} {endpoint} HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n",
        payload.len()
    );

    stream.write_all(request.as_bytes()).await?;
    stream.write_all(payload).await?;
    let text = read_http_response_headers(&mut stream).await?;
    let status = parse_http_status(&text)?;

    if (200..300).contains(&status) {
        return Ok(());
    }
    bail!("firecracker API {method} request to {endpoint} failed with status {status}: {text}");
}

async fn read_http_response_headers(stream: &mut tokio::net::UnixStream) -> Result<String> {
    let mut response = Vec::new();
    let mut buf = [0_u8; 1024];

    for _ in 0..64 {
        let read = tokio::time::timeout(Duration::from_secs(2), stream.read(&mut buf))
            .await
            .context("timed out waiting for firecracker API response")??;
        if read == 0 {
            break;
        }
        response.extend_from_slice(&buf[..read]);
        if response.windows(4).any(|window| window == b"\r\n\r\n") {
            break;
        }
    }

    if response.is_empty() {
        bail!("empty HTTP response from firecracker API");
    }

    Ok(String::from_utf8_lossy(&response).to_string())
}

fn parse_http_status(response: &str) -> Result<u16> {
    let first_line = response
        .lines()
        .next()
        .context("empty HTTP response from firecracker API")?;
    let status = first_line
        .split_whitespace()
        .nth(1)
        .with_context(|| format!("invalid HTTP status line from firecracker API: {first_line}"))?
        .parse::<u16>()?;
    Ok(status)
}

async fn configure_vm(api_socket: &str, config: &VmConfig, vsock_uds: &Path) -> Result<()> {
    put_api(
        api_socket,
        "/boot-source",
        json!({
            "kernel_image_path": config.kernel_path.display().to_string(),
            "boot_args": config.boot_args,
        }),
    )
    .await?;
    put_api(
        api_socket,
        "/drives/rootfs",
        json!({
            "drive_id": "rootfs",
            "path_on_host": config.rootfs_path.display().to_string(),
            "is_root_device": true,
            "is_read_only": false,
        }),
    )
    .await?;
    put_api(
        api_socket,
        "/drives/codedisk",
        json!({
            "drive_id": "codedisk",
            "path_on_host": config.code_disk_path.display().to_string(),
            "is_root_device": false,
            "is_read_only": false,
        }),
    )
    .await?;
    patch_api(
        api_socket,
        "/machine-config",
        json!({
            "vcpu_count": config.vcpu_count,
            "mem_size_mib": config.mem_size_mb,
        }),
    )
    .await?;
    put_api(
        api_socket,
        "/vsock",
        json!({
            "guest_cid": config.guest_cid,
            "uds_path": vsock_uds.display().to_string(),
        }),
    )
    .await?;
    put_api(
        api_socket,
        "/entropy",
        json!({
            "rate_limiter": {
                "bandwidth": {
                    "size": 1000,
                    "one_time_burst": 0,
                    "refill_time": 100
                }
            }
        }),
    )
    .await?;
    put_api(
        api_socket,
        "/network-interfaces/net1",
        json!({
            "iface_id": "net1",
            "guest_mac": config.guest_mac,
            "host_dev_name": config.tap_device,
        }),
    )
    .await?;

    Ok(())
}

fn is_retryable_api_error(err: &anyhow::Error) -> bool {
    if err
        .to_string()
        .contains("empty HTTP response from firecracker API")
    {
        return true;
    }

    err.chain().any(|cause| {
        if cause
            .downcast_ref::<tokio::time::error::Elapsed>()
            .is_some()
        {
            return true;
        }

        cause
            .downcast_ref::<std::io::Error>()
            .map(|io_err| {
                matches!(
                    io_err.kind(),
                    std::io::ErrorKind::ConnectionRefused
                        | std::io::ErrorKind::ConnectionReset
                        | std::io::ErrorKind::ConnectionAborted
                        | std::io::ErrorKind::NotConnected
                        | std::io::ErrorKind::TimedOut
                        | std::io::ErrorKind::UnexpectedEof
                        | std::io::ErrorKind::BrokenPipe
                        | std::io::ErrorKind::WouldBlock
                )
            })
            .unwrap_or(false)
    })
}

fn firecracker_setup_diagnostics(child: &mut Child, stderr_log: &Path) -> String {
    let mut sections = Vec::new();

    match child.try_wait() {
        Ok(Some(status)) => sections.push(format!("firecracker exited with status {status}")),
        Ok(None) => sections.push("firecracker process is still running".to_string()),
        Err(err) => sections.push(format!("failed to query firecracker process status: {err}")),
    }

    match std::fs::read_to_string(stderr_log) {
        Ok(stderr) => {
            let tail = last_lines(&stderr, FIRECRACKER_STDERR_TAIL_LINES);
            if !tail.trim().is_empty() {
                sections.push(format!(
                    "firecracker stderr (last {FIRECRACKER_STDERR_TAIL_LINES} lines):\n{tail}"
                ));
            }
        }
        Err(err) => sections.push(format!(
            "failed to read firecracker stderr log at {}: {err}",
            stderr_log.display()
        )),
    }

    if sections.is_empty() {
        String::new()
    } else {
        format!("\n\n{}", sections.join("\n\n"))
    }
}

fn last_lines(text: &str, line_count: usize) -> String {
    let lines: Vec<&str> = text.lines().collect();
    let start = lines.len().saturating_sub(line_count);
    lines[start..].join("\n")
}

fn cleanup_vm_process_and_files(
    child: &mut Child,
    api_socket: &Path,
    vsock_uds: &Path,
    socket_dir: &Path,
) -> Result<()> {
    if child.try_wait()?.is_none() {
        child.kill()?;
        let _ = child.wait();
    }

    let _ = std::fs::remove_file(api_socket);
    let _ = std::fs::remove_file(vsock_uds);
    let _ = std::fs::remove_dir_all(socket_dir);
    Ok(())
}

fn validate_config(config: &VmConfig) -> Result<()> {
    if !config.kernel_path.is_file() {
        bail!("kernel image not found at {}", config.kernel_path.display());
    }
    if !config.rootfs_path.is_file() {
        bail!("rootfs image not found at {}", config.rootfs_path.display());
    }
    if !config.code_disk_path.is_file() {
        bail!(
            "code disk image not found at {}",
            config.code_disk_path.display()
        );
    }
    if config.vcpu_count == 0 {
        bail!("vcpu_count must be greater than 0");
    }
    if config.mem_size_mb < 64 {
        bail!("mem_size_mb must be at least 64 MiB");
    }
    Ok(())
}
