use chrono::Utc;
use dashmap::DashMap;
use serde::Serialize;
use tokio::sync::broadcast;
use vetty_common::{AgentHandshake, SandboxEvent, SandboxId, SandboxInfo, SandboxStatus};

#[derive(Debug, Clone, Serialize, serde::Deserialize)]
pub struct TaggedEvent {
    pub sandbox_id: SandboxId,
    pub event: SandboxEvent,
}

#[derive(Debug, Clone)]
struct SandboxSession {
    info: SandboxInfo,
    events: Vec<SandboxEvent>,
}

pub struct EventStore {
    sessions: DashMap<SandboxId, SandboxSession>,
    global_tx: broadcast::Sender<TaggedEvent>,
}

impl EventStore {
    pub fn new() -> Self {
        let (global_tx, _) = broadcast::channel(10_000);
        Self {
            sessions: DashMap::new(),
            global_tx,
        }
    }

    pub fn register(&self, handshake: &AgentHandshake) {
        let id = handshake.sandbox_id;
        let name = format!("{}@{}", id, handshake.hostname);
        let session = SandboxSession {
            info: SandboxInfo {
                id,
                name,
                status: SandboxStatus::Running,
                started_at: Utc::now(),
                event_count: 0,
            },
            events: Vec::new(),
        };
        self.sessions.insert(id, session);
    }

    pub fn push_event(&self, sandbox_id: &SandboxId, event: SandboxEvent) {
        if let Some(mut session) = self.sessions.get_mut(sandbox_id) {
            session.info.event_count += 1;
            session.events.push(event.clone());
        } else {
            let fallback = SandboxSession {
                info: SandboxInfo {
                    id: *sandbox_id,
                    name: sandbox_id.to_string(),
                    status: SandboxStatus::Running,
                    started_at: Utc::now(),
                    event_count: 1,
                },
                events: vec![event.clone()],
            };
            self.sessions.insert(*sandbox_id, fallback);
        }

        let tagged = TaggedEvent {
            sandbox_id: *sandbox_id,
            event,
        };
        let _ = self.global_tx.send(tagged);
    }

    pub fn mark_stopped(&self, sandbox_id: &SandboxId) {
        if let Some(mut session) = self.sessions.get_mut(sandbox_id) {
            session.info.status = SandboxStatus::Stopped;
        }
    }

    pub fn list_sandboxes(&self) -> Vec<SandboxInfo> {
        let mut sandboxes: Vec<_> = self
            .sessions
            .iter()
            .map(|entry| entry.info.clone())
            .collect();
        sandboxes.sort_by_key(|s| s.started_at);
        sandboxes
    }

    pub fn get_events(&self, sandbox_id: &SandboxId) -> Option<Vec<SandboxEvent>> {
        self.sessions.get(sandbox_id).map(|s| s.events.clone())
    }

    pub fn list_events(&self) -> Vec<TaggedEvent> {
        let mut all = Vec::new();
        for entry in self.sessions.iter() {
            let session = entry.value();
            for event in &session.events {
                all.push(TaggedEvent {
                    sandbox_id: *entry.key(),
                    event: event.clone(),
                });
            }
        }
        all.sort_by_key(|e| e.event.timestamp);
        all
    }

    pub fn subscribe(&self) -> broadcast::Receiver<TaggedEvent> {
        self.global_tx.subscribe()
    }
}
