# Step 5 — Host Daemon (`vetty-daemon`)

## Goal
Build the host-side daemon that listens for incoming vsock connections from guest agents, ingests JSON event streams, maintains a registry of active sandboxes, and exposes both a WebSocket (for the GUI) and REST endpoints.

---

## 5.1 Create the Crate

### `crates/vetty-daemon/Cargo.toml`

```toml
[package]
name = "vetty-daemon"
version.workspace = true
edition.workspace = true

[dependencies]
vetty-common = { path = "../vetty-common" }
anyhow = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
tokio = { workspace = true }
tracing = { workspace = true }
tracing-subscriber = { workspace = true }
uuid = { workspace = true }
chrono = { workspace = true }
axum = { version = "0.8", features = ["ws"] }
tower = "0.5"
tower-http = { version = "0.6", features = ["cors"] }
tokio-tungstenite = "0.24"
dashmap = "6"
```

---

## 5.2 File: `crates/vetty-daemon/src/events.rs`

### Event Store and Broadcasting

The daemon needs:

1. **In-memory event store** — a `DashMap<SandboxId, SandboxSession>` where each session contains:
   - `SandboxInfo` (id, name, status, started_at, event_count)
   - `Vec<SandboxEvent>` — all events for this session
   - A `tokio::sync::broadcast::Sender<TaggedEvent>` for live streaming

2. **`TaggedEvent`** — an event with its sandbox ID attached:
   ```rust
   #[derive(Debug, Clone, Serialize)]
   pub struct TaggedEvent {
       pub sandbox_id: SandboxId,
       pub event: SandboxEvent,
   }
   ```

3. **Global broadcast channel** — a single `broadcast::Sender<TaggedEvent>` that all WebSocket clients subscribe to.

### Public API

```rust
use vetty_common::*;
use dashmap::DashMap;
use tokio::sync::broadcast;

pub struct EventStore {
    sessions: DashMap<SandboxId, SandboxSession>,
    global_tx: broadcast::Sender<TaggedEvent>,
}

struct SandboxSession {
    info: SandboxInfo,
    events: Vec<SandboxEvent>,
}

impl EventStore {
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(10_000);
        Self {
            sessions: DashMap::new(),
            global_tx: tx,
        }
    }

    /// Register a new sandbox from a handshake
    pub fn register(&self, handshake: &AgentHandshake) { todo!() }

    /// Ingest an event for a sandbox
    pub fn push_event(&self, sandbox_id: &SandboxId, event: SandboxEvent) { todo!() }

    /// Mark a sandbox as stopped
    pub fn mark_stopped(&self, sandbox_id: &SandboxId) { todo!() }

    /// List all sandboxes
    pub fn list_sandboxes(&self) -> Vec<SandboxInfo> { todo!() }

    /// Get all events for a sandbox
    pub fn get_events(&self, sandbox_id: &SandboxId) -> Option<Vec<SandboxEvent>> { todo!() }

    /// Subscribe to the global event stream
    pub fn subscribe(&self) -> broadcast::Receiver<TaggedEvent> {
        self.global_tx.subscribe()
    }
}
```

---

## 5.3 File: `crates/vetty-daemon/src/vsock.rs`

### Vsock Listener

The daemon listens on the vsock Unix socket that Firecracker creates. When the guest agent connects:

1. Accept the connection
2. Read the first JSON line — it must be a `WireMessage::Handshake`
3. Register the sandbox in the `EventStore`
4. Read subsequent lines — each is a `WireMessage::Event`
5. Push each event into the `EventStore`
6. When the connection drops, mark the sandbox as `Stopped`

```rust
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::net::UnixListener;
use anyhow::Result;
use vetty_common::WireMessage;
use crate::events::EventStore;

pub async fn listen_vsock(uds_path: &str, store: Arc<EventStore>) -> Result<()> {
    // The vsock UDS path comes from Firecracker config
    // Firecracker appends "_<cid>" to the UDS path for incoming connections
    let listener = UnixListener::bind(format!("{}_3", uds_path))?;

    loop {
        let (stream, _) = listener.accept().await?;
        let store = store.clone();

        tokio::spawn(async move {
            let reader = BufReader::new(stream);
            let mut lines = reader.lines();

            // First line must be handshake
            let first_line = lines.next_line().await?;
            // Parse and register...

            // Remaining lines are events
            while let Some(line) = lines.next_line().await? {
                // Parse WireMessage::Event and push to store
            }

            // Connection closed — mark stopped
            Ok::<_, anyhow::Error>(())
        });
    }
}
```

> **Important vsock UDS convention:** Firecracker creates the Unix socket at `{uds_path}_{cid}` for incoming connections from the guest. So if you configured vsock with `uds_path: "/tmp/vetty_v.sock"` and `guest_cid: 3`, listen on `/tmp/vetty_v.sock_3`.

---

## 5.4 File: `crates/vetty-daemon/src/rest.rs`

### REST API

Use axum to expose:

| Method | Path                 | Response                  |
|--------|----------------------|---------------------------|
| GET    | `/api/sandboxes`     | `Vec<SandboxInfo>` (JSON) |
| GET    | `/api/sandboxes/:id/events` | `Vec<SandboxEvent>` (JSON) |

```rust
use axum::{Router, Json, extract::{State, Path}};
use std::sync::Arc;
use crate::events::EventStore;

pub fn rest_router(store: Arc<EventStore>) -> Router {
    Router::new()
        .route("/api/sandboxes", axum::routing::get(list_sandboxes))
        .route("/api/sandboxes/:id/events", axum::routing::get(get_events))
        .with_state(store)
}

async fn list_sandboxes(State(store): State<Arc<EventStore>>) -> Json<Vec<SandboxInfo>> {
    Json(store.list_sandboxes())
}

async fn get_events(
    State(store): State<Arc<EventStore>>,
    Path(id): Path<String>,
) -> Json<Vec<SandboxEvent>> {
    // Parse UUID from id, look up events
    todo!()
}
```

---

## 5.5 File: `crates/vetty-daemon/src/ws.rs`

### WebSocket Endpoint

The GUI connects to `ws://localhost:<port>/ws/events` and receives a real-time stream of `TaggedEvent` JSON messages.

```rust
use axum::{
    extract::{State, WebSocketUpgrade, ws::Message},
    response::Response,
};
use std::sync::Arc;
use crate::events::EventStore;

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(store): State<Arc<EventStore>>,
) -> Response {
    ws.on_upgrade(move |socket| handle_ws(socket, store))
}

async fn handle_ws(mut socket: axum::extract::ws::WebSocket, store: Arc<EventStore>) {
    let mut rx = store.subscribe();

    while let Ok(event) = rx.recv().await {
        let json = serde_json::to_string(&event).unwrap();
        if socket.send(Message::Text(json)).await.is_err() {
            break; // Client disconnected
        }
    }
}
```

---

## 5.6 File: `crates/vetty-daemon/src/main.rs`

### Daemon Entry Point

```rust
use std::sync::Arc;
use anyhow::Result;
use tokio::net::TcpListener;
use tower_http::cors::CorsLayer;

mod events;
mod vsock;
mod rest;
mod ws;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::init();

    let store = Arc::new(events::EventStore::new());

    // Start vsock listener in background
    let vsock_store = store.clone();
    let vsock_path = std::env::var("VETTY_VSOCK_PATH")
        .unwrap_or_else(|_| "/tmp/vetty_v.sock".to_string());
    tokio::spawn(async move {
        if let Err(e) = vsock::listen_vsock(&vsock_path, vsock_store).await {
            tracing::error!("vsock listener error: {}", e);
        }
    });

    // Build HTTP + WS router
    let app = rest::rest_router(store.clone())
        .route("/ws/events", axum::routing::get(ws::ws_handler))
        .layer(CorsLayer::permissive())
        .with_state(store);

    let port = std::env::var("VETTY_DAEMON_PORT")
        .unwrap_or_else(|_| "9876".to_string())
        .parse::<u16>()?;

    let listener = TcpListener::bind(format!("127.0.0.1:{}", port)).await?;
    tracing::info!("vetty-daemon listening on port {}", port);

    axum::serve(listener, app).await?;
    Ok(())
}
```

---

## Done Criteria

- [ ] `vetty-daemon` compiles
- [ ] `EventStore` handles registration, ingestion, and broadcasting
- [ ] vsock listener accepts connections, parses wire protocol
- [ ] REST endpoints return sandbox list and events
- [ ] WebSocket endpoint streams events in real time
- [ ] CORS is enabled for local GUI access
