# Step 3 — Guest Agent (`vetty-agent`)

## Goal
Build a statically-linked Rust binary that runs inside the Firecracker VM guest. It reads strace output from stdin (piped by `vetty-run`), parses each line into structured events, and sends them as newline-delimited JSON over a vsock connection to the host.

---

## 3.1 Create the Crate

### `crates/vetty-agent/Cargo.toml`

```toml
[package]
name = "vetty-agent"
version.workspace = true
edition.workspace = true

[dependencies]
vetty-common = { path = "../vetty-common" }
anyhow = { workspace = true }
serde_json = { workspace = true }
tracing = { workspace = true }
tracing-subscriber = { workspace = true }
chrono = { workspace = true }
uuid = { workspace = true }
regex = "1"
```

> **Important:** This binary must be statically linked for the guest. Build with:
> ```bash
> cargo build --target x86_64-unknown-linux-musl --release -p vetty-agent
> ```

---

## 3.2 File: `crates/vetty-agent/src/strace_parser.rs`

### What strace output looks like

```
12345 1715200000.123456 open("/etc/passwd", O_RDONLY) = 3
12345 1715200000.234567 read(3, "root:x:0:0:..."..., 4096) = 512
12345 1715200000.345678 connect(4, {sa_family=AF_INET, sin_port=htons(443), sin_addr=inet_addr("93.184.216.34")}, 16) = 0
12345 1715200000.456789 execve("/usr/bin/curl", ["curl", "https://evil.com"], ...) = 0
```

`vetty-run` will invoke strace with these flags:
```
strace -f -tt -T -e trace=file,network,process -o >(vetty-agent) -- <target>
```

### Parser requirements

1. **Regex-based line parser** — extract from each strace line:
   - `pid` (u32)
   - `timestamp` (parse the epoch seconds or HH:MM:SS.usec format)
   - `syscall_name` (string)
   - `args` (raw string of arguments)
   - `return_value` (i64)

2. **Classify into `EventType`:**
   - `open`, `openat`, `stat`, `lstat`, `access`, `readlink`, `unlink`, `rename`, `mkdir`, `rmdir`, `chmod`, `chown` → `FileAccess`
   - `connect`, `bind`, `accept`, `socket`, `sendto`, `recvfrom` → `NetworkConnect`
   - `execve`, `fork`, `clone`, `vfork` → `ProcessSpawn`
   - Everything else → `Syscall`

3. **Extract relevant fields based on type:**
   - For `FileAccess`: extract the `path` argument (first quoted string)
   - For `NetworkConnect`: extract hostname/IP and port from the `sa_family` struct
   - For `ProcessSpawn`: extract the binary path from execve's first arg

### Public API

```rust
use vetty_common::SandboxEvent;
use anyhow::Result;

pub struct StraceParser {
    // compiled regexes
}

impl StraceParser {
    pub fn new() -> Result<Self> { todo!() }

    /// Parse a single strace line into a SandboxEvent.
    /// Returns None if the line is not a valid/complete syscall line
    /// (e.g., unfinished calls, signals, etc.)
    pub fn parse_line(&self, line: &str) -> Option<SandboxEvent> { todo!() }
}
```

---

## 3.3 File: `crates/vetty-agent/src/vsock_client.rs`

### vsock connection

The agent connects to the host using virtio-vsock:
- CID: `2` (the host is always CID 2 from the guest's perspective)
- Port: `VSOCK_PORT` from `vetty-common` (5123)

### Implementation

```rust
use std::io::{BufWriter, Write};
use std::os::unix::net::UnixStream;  // vsock uses AF_VSOCK but we use the socket2 crate
use anyhow::Result;
use vetty_common::{WireMessage, AgentHandshake, SandboxEvent, VSOCK_PORT};

pub struct VsockClient {
    writer: BufWriter<socket2::Socket>,
}

impl VsockClient {
    /// Connect to the host over vsock
    pub fn connect() -> Result<Self> {
        // Use socket2 to create an AF_VSOCK socket
        // Connect to (CID=2, port=VSOCK_PORT)
        todo!()
    }

    /// Send the handshake message
    pub fn send_handshake(&mut self, handshake: &AgentHandshake) -> Result<()> {
        let msg = WireMessage::Handshake(handshake.clone());
        let line = serde_json::to_string(&msg)?;
        writeln!(self.writer, "{}", line)?;
        self.writer.flush()?;
        Ok(())
    }

    /// Send a single event
    pub fn send_event(&mut self, event: &SandboxEvent) -> Result<()> {
        let msg = WireMessage::Event(event.clone());
        let line = serde_json::to_string(&msg)?;
        writeln!(self.writer, "{}", line)?;
        // Don't flush every line — batch for performance
        Ok(())
    }

    /// Flush the buffer
    pub fn flush(&mut self) -> Result<()> {
        self.writer.flush()?;
        Ok(())
    }
}
```

> **Dependency:** Add `socket2 = { version = "0.5", features = ["all"] }` to Cargo.toml.

### vsock with socket2

```rust
use socket2::{Domain, Protocol, SockAddr, Socket, Type};

fn connect_vsock(cid: u32, port: u32) -> Result<Socket> {
    let socket = Socket::new(
        Domain::VSOCK,
        Type::STREAM,
        None,
    )?;

    let addr = SockAddr::vsock(cid, port);
    socket.connect(&addr)?;
    Ok(socket)
}
```

---

## 3.4 File: `crates/vetty-agent/src/main.rs`

### Main loop

```rust
use std::io::{self, BufRead};
use anyhow::Result;

mod strace_parser;
mod vsock_client;

fn main() -> Result<()> {
    tracing_subscriber::init();

    let parser = strace_parser::StraceParser::new()?;
    let mut client = vsock_client::VsockClient::connect()?;

    // Send handshake
    let handshake = AgentHandshake {
        sandbox_id: SandboxId::new(), // or read from env var set by init.sh
        agent_version: env!("CARGO_PKG_VERSION").to_string(),
        hostname: hostname::get()?.to_string_lossy().to_string(),
    };
    client.send_handshake(&handshake)?;

    // Read strace output from stdin, line by line
    let stdin = io::stdin();
    let mut line_count = 0u64;

    for line in stdin.lock().lines() {
        let line = line?;
        if let Some(event) = parser.parse_line(&line) {
            client.send_event(&event)?;
        }
        line_count += 1;

        // Flush every 100 lines for reasonable latency
        if line_count % 100 == 0 {
            client.flush()?;
        }
    }

    client.flush()?;
    Ok(())
}
```

> **Dependency:** Add `hostname = "0.4"` to Cargo.toml.

---

## 3.5 Cross-Compilation Note

The agent must be a **static musl binary** so it runs in the minimal guest without glibc:

```bash
rustup target add x86_64-unknown-linux-musl
cargo build --target x86_64-unknown-linux-musl --release -p vetty-agent
```

The resulting binary at `target/x86_64-unknown-linux-musl/release/vetty-agent` will be placed in the rootfs.

---

## Done Criteria

- [ ] `vetty-agent` compiles (at least `cargo check`)
- [ ] `StraceParser` can handle common strace line formats
- [ ] `VsockClient` creates an AF_VSOCK connection and sends newline-delimited JSON
- [ ] Main loop reads stdin → parses → sends events
- [ ] Handshake is sent on connect
