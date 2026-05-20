use std::sync::Arc;

use axum::extract::ws::{Message, WebSocket};
use axum::extract::{State, WebSocketUpgrade};
use axum::response::Response;

use crate::events::EventStore;

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(store): State<Arc<EventStore>>,
) -> Response {
    ws.on_upgrade(move |socket| handle_ws(socket, store))
}

async fn handle_ws(mut socket: WebSocket, store: Arc<EventStore>) {
    let mut rx = store.subscribe();

    loop {
        let tagged = match rx.recv().await {
            Ok(event) => event,
            Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
            Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
        };

        let json = match serde_json::to_string(&tagged) {
            Ok(json) => json,
            Err(err) => {
                tracing::warn!("failed to serialize ws message: {err}");
                continue;
            }
        };

        if socket.send(Message::Text(json.into())).await.is_err() {
            break;
        }
    }
}
