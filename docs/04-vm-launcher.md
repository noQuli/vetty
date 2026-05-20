# Step 4 — Firecracker VM Launcher (`vetty-vm`)

## Goal
Create a library crate that spawns a Firecracker process, configures it via its HTTP API, attaches drives, sets up vsock, starts the VM, and connects the host terminal to the serial console.

---

## 4.1 Create the Crate

### `crates/vetty-vm/Cargo.toml`

```toml
[package]
name = "vetty-vm"
version.workspace = true
edition.workspace = true

[dependencies]
vetty-common = { path = "../vetty-common" }
anyhow = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
tracing = { workspace = true }
tokio = { workspace = true }
uuid = { workspace = true }
reqwest = { version = "0.12", default-features = false, features = ["json"] }
hyper = { version = "1", features = ["client", "http1"] }
hyper-util = "0.1"
hyperlocal = "0.9"  # For Unix socket HTTP requests to Firecracker API
tempfile = "3"
nix = { version = "0.29", features = ["term", "signal"] }
```

---

## 4.2 File: `crates/vetty-vm/src/config.rs`

### VM configuration

```rust
use std::path::PathBuf;

/// Configuration for a Firecracker VM instance
pub struct VmConfig {
    /// Path to the Firecracker binary (default: "firecracker" on PATH)
    pub firecracker_bin: PathBuf,
    /// Path to the kernel image (vmlinux)
    pub kernel_path: PathBuf,
    /// Path to the root filesystem image
    pub rootfs_path: PathBuf,
    /// Path to the code disk image (mounted as /sandbox in guest)
    pub code_disk_path: PathBuf,
    /// Number of vCPUs (default: 1)
    pub vcpu_count: u8,
    /// Memory size in MB (default: 128)
    pub mem_size_mb: u32,
    /// Guest CID for vsock
    pub guest_cid: u32,
    /// Kernel boot arguments
    pub boot_args: String,
}

impl Default for VmConfig {
    fn default() -> Self {
        Self {
            firecracker_bin: PathBuf::from("firecracker"),
            kernel_path: PathBuf::new(),
            rootfs_path: PathBuf::new(),
            code_disk_path: PathBuf::new(),
            vcpu_count: 1,
            mem_size_mb: 128,
            guest_cid: vetty_common::GUEST_CID,
            boot_args: "console=ttyS0 reboot=k panic=1 pci=off".to_string(),
        }
    }
}
```

---

## 4.3 File: `crates/vetty-vm/src/lib.rs`

### Firecracker API interaction

The Firecracker API is an HTTP/REST API listening on a Unix socket. All configuration must happen before calling `PUT /actions` with `InstanceStart`.

#### Sequence of API calls:

1. **`PUT /boot-source`** — set kernel path and boot args
   ```json
   {
     "kernel_image_path": "/path/to/vmlinux",
     "boot_args": "console=ttyS0 reboot=k panic=1 pci=off"
   }
   ```

2. **`PUT /drives/rootfs`** — attach root drive
   ```json
   {
     "drive_id": "rootfs",
     "path_on_host": "/path/to/rootfs.ext4",
     "is_root_device": true,
     "is_read_only": false
   }
   ```

3. **`PUT /drives/codedisk`** — attach code disk
   ```json
   {
     "drive_id": "codedisk",
     "path_on_host": "/path/to/code.img",
     "is_root_device": false,
     "is_read_only": false
   }
   ```

4. **`PUT /machine-config`** — set vCPUs and memory
   ```json
   {
     "vcpu_count": 1,
     "mem_size_mib": 128
   }
   ```

5. **`PUT /vsock`** — configure vsock
   ```json
   {
     "guest_cid": 3,
     "uds_path": "/tmp/vetty-<uuid>_v.sock"
   }
   ```

6. **`PUT /actions`** — start the VM
   ```json
   { "action_type": "InstanceStart" }
   ```

### Public API

```rust
use std::path::PathBuf;
use anyhow::Result;

pub mod config;
pub mod serial;

use config::VmConfig;

pub struct VmInstance {
    /// Path to the Firecracker API socket
    api_socket: PathBuf,
    /// Path to the vsock UDS on the host
    vsock_uds: PathBuf,
    /// Handle to the Firecracker child process
    child: std::process::Child,
}

impl VmInstance {
    /// Spawn Firecracker and configure the VM, but don't start it yet.
    pub async fn create(config: &VmConfig) -> Result<Self> {
        // 1. Create a temp directory for sockets
        // 2. Spawn firecracker with --api-sock <socket_path>
        // 3. Wait briefly for the socket to appear
        // 4. Make PUT requests to configure the VM
        todo!()
    }

    /// Start the VM (PUT /actions InstanceStart)
    pub async fn start(&self) -> Result<()> { todo!() }

    /// Get the path to the vsock UDS (for the daemon to listen on)
    pub fn vsock_uds_path(&self) -> &PathBuf { &self.vsock_uds }

    /// Connect to the serial console and relay I/O to the current terminal
    pub fn attach_serial(&self) -> Result<()> { todo!() }

    /// Stop the VM and clean up
    pub fn kill(&mut self) -> Result<()> {
        self.child.kill()?;
        self.child.wait()?;
        // Clean up socket files
        let _ = std::fs::remove_file(&self.api_socket);
        let _ = std::fs::remove_file(&self.vsock_uds);
        Ok(())
    }
}

impl Drop for VmInstance {
    fn drop(&mut self) {
        let _ = self.kill();
    }
}
```

### HTTP-over-Unix-socket helper

```rust
use hyper_util::rt::TokioIo;
use tokio::net::UnixStream;

async fn put_api(socket_path: &str, endpoint: &str, body: serde_json::Value) -> Result<()> {
    let stream = UnixStream::connect(socket_path).await?;
    // Build HTTP PUT request to the endpoint
    // Send the JSON body
    // Check for 2xx response
    todo!()
}
```

---

## 4.4 File: `crates/vetty-vm/src/serial.rs`

### Serial console relay

Firecracker exposes the guest serial console through the process's stdin/stdout. To attach a terminal:

1. **Put the host terminal in raw mode** (disable echo, line buffering, etc.)
2. **Spawn two tasks:**
   - Read from host stdin → write to Firecracker's stdin
   - Read from Firecracker's stdout → write to host stdout
3. **Restore terminal settings on exit**

```rust
use nix::sys::termios;
use std::io::{Read, Write};
use anyhow::Result;

pub fn attach_serial(child_stdin: &mut dyn Write, child_stdout: &mut dyn Read) -> Result<()> {
    // Save original terminal settings
    let stdin_fd = std::io::stdin().as_raw_fd();
    let original = termios::tcgetattr(stdin_fd)?;

    // Set raw mode
    let mut raw = original.clone();
    termios::cfmakeraw(&mut raw);
    termios::tcsetattr(stdin_fd, termios::SetArg::TCSANOW, &raw)?;

    // Spawn relay threads
    // ... (read/write loops)

    // Restore terminal on exit
    termios::tcsetattr(stdin_fd, termios::SetArg::TCSANOW, &original)?;
    Ok(())
}
```

---

## 4.5 Spawning Firecracker

```rust
use std::process::{Command, Stdio};

fn spawn_firecracker(bin: &Path, api_socket: &Path) -> Result<std::process::Child> {
    let child = Command::new(bin)
        .arg("--api-sock")
        .arg(api_socket)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    Ok(child)
}
```

---

## Done Criteria

- [ ] `vetty-vm` compiles
- [ ] `VmConfig` covers all necessary fields
- [ ] `VmInstance::create()` spawns Firecracker and configures via API socket
- [ ] `VmInstance::start()` sends the InstanceStart action
- [ ] Serial console relay works in raw mode
- [ ] `VmInstance::kill()` and `Drop` clean up child process and socket files
