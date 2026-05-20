# Step 1 — Workspace Setup & vetty-common

## Goal
Set up the Cargo workspace and create the shared types crate that all other crates depend on.

---

## 1.1 Create Cargo Workspace

Create `Cargo.toml` at the repo root:

```toml
[workspace]
resolver = "2"
members = [
    "crates/vetty-common",
    "crates/vetty-disk",
    "crates/vetty-agent",
    "crates/vetty-vm",
    "crates/vetty-daemon",
    "crates/vetty-cli",
]

[workspace.package]
version = "0.1.0"
edition = "2021"
license = "MIT"

[workspace.dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["full"] }
anyhow = "1"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
uuid = { version = "1", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }
```

---

## 1.2 Create vetty-common

This crate defines the shared event types and protocol structures used by both the guest agent and the host daemon.

### `crates/vetty-common/Cargo.toml`

```toml
[package]
name = "vetty-common"
version.workspace = true
edition.workspace = true

[dependencies]
serde = { workspace = true }
serde_json = { workspace = true }
chrono = { workspace = true }
uuid = { workspace = true }
```

### `crates/vetty-common/src/lib.rs`

Define these types:

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Unique identifier for a sandbox session
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct SandboxId(pub Uuid);

impl SandboxId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

/// The type of event captured
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    Syscall,
    FileAccess,
    NetworkConnect,
    ProcessSpawn,
    HttpRequest,
    HttpResponse,
}

/// A single captured event from the guest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxEvent {
    pub timestamp: DateTime<Utc>,
    pub pid: u32,
    pub event_type: EventType,
    pub syscall_name: Option<String>,
    pub path: Option<String>,
    pub hostname: Option<String>,
    pub port: Option<u16>,
    pub flags: Option<String>,
    pub return_value: Option<i64>,
    /// For HTTP events
    pub http_method: Option<String>,
    pub http_url: Option<String>,
    pub http_status: Option<u16>,
    pub http_headers: Option<serde_json::Value>,
    pub http_body: Option<String>,
    /// Raw strace line for debugging
    pub raw: Option<String>,
}

/// Handshake message sent by the agent when it connects over vsock
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentHandshake {
    pub sandbox_id: SandboxId,
    pub agent_version: String,
    pub hostname: String,
}

/// Status of a sandbox
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SandboxStatus {
    Starting,
    Running,
    Stopped,
    Error,
}

/// Summary of a sandbox, used in REST API responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxInfo {
    pub id: SandboxId,
    pub name: String,
    pub status: SandboxStatus,
    pub started_at: DateTime<Utc>,
    pub event_count: u64,
}

/// Wire protocol: each line over vsock is one of these
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WireMessage {
    #[serde(rename = "handshake")]
    Handshake(AgentHandshake),
    #[serde(rename = "event")]
    Event(SandboxEvent),
}

/// The vsock port the agent connects to on the host
pub const VSOCK_PORT: u32 = 5123;

/// The guest CID used by Firecracker (2 = host, 3+ = guest)
pub const GUEST_CID: u32 = 3;
```

---

## 1.3 Verify

After creating these files, ensure the workspace compiles:

```bash
cargo check -p vetty-common
```

---

## Done Criteria

- [ ] `Cargo.toml` workspace exists at repo root
- [ ] `crates/vetty-common` compiles with no errors
- [ ] All shared types (`SandboxEvent`, `WireMessage`, `AgentHandshake`, etc.) are defined
- [ ] Serde derive macros work for JSON serialization
