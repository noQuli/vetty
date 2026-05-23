use std::fmt::{Display, Formatter};
use std::str::FromStr;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub mod http;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(transparent)]
pub struct SandboxId(pub Uuid);

impl SandboxId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for SandboxId {
    fn default() -> Self {
        Self::new()
    }
}

impl Display for SandboxId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for SandboxId {
    type Err = uuid::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(Uuid::parse_str(s)?))
    }
}

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
    pub http_method: Option<String>,
    pub http_url: Option<String>,
    pub http_status: Option<u16>,
    pub http_headers: Option<serde_json::Value>,
    pub http_body: Option<String>,
    pub http_message: Option<String>,
    pub raw: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentHandshake {
    pub sandbox_id: SandboxId,
    pub agent_version: String,
    pub hostname: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SandboxStatus {
    Starting,
    Running,
    Stopped,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxInfo {
    pub id: SandboxId,
    pub name: String,
    pub status: SandboxStatus,
    pub started_at: DateTime<Utc>,
    pub event_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WireMessage {
    #[serde(rename = "handshake")]
    Handshake(AgentHandshake),
    #[serde(rename = "event")]
    Event(Box<SandboxEvent>),
}

pub const VSOCK_PORT: u32 = 5123;
pub const HOST_CID: u32 = 2;
pub const GUEST_CID: u32 = 3;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sandbox_id_roundtrip() {
        let id = SandboxId::new();
        let encoded = serde_json::to_string(&id).expect("serialize");
        let decoded: SandboxId = serde_json::from_str(&encoded).expect("deserialize");
        assert_eq!(id, decoded);
    }
}
