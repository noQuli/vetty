use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, Result};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::net::{UnixListener, UnixStream};
use vetty_common::WireMessage;

use crate::events::EventStore;

pub async fn listen_vsock(uds_path: &str, store: Arc<EventStore>) -> Result<()> {
    let socket_path = format!("{}_{}", uds_path, vetty_common::VSOCK_PORT);
    let socket_path_buf = PathBuf::from(&socket_path);
    if socket_path_buf.exists() {
        std::fs::remove_file(&socket_path_buf)?;
    }

    let listener = UnixListener::bind(&socket_path_buf).with_context(|| {
        format!(
            "failed to bind vsock ingress unix socket at {}",
            socket_path_buf.display()
        )
    })?;

    loop {
        let (stream, _) = listener.accept().await?;
        let store = store.clone();
        tokio::spawn(async move {
            if let Err(err) = handle_connection(stream, store).await {
                tracing::error!("vsock connection error: {err}");
            }
        });
    }
}

async fn handle_connection(stream: UnixStream, store: Arc<EventStore>) -> Result<()> {
    let mut lines = BufReader::new(stream).lines();
    let first_line = lines
        .next_line()
        .await?
        .context("received empty vsock stream with no handshake")?;

    let handshake: WireMessage = serde_json::from_str(&first_line)
        .with_context(|| "failed to parse first wire line as JSON")?;
    let sandbox_id = match handshake {
        WireMessage::Handshake(handshake) => {
            let sandbox_id = handshake.sandbox_id;
            store.register(&handshake);
            sandbox_id
        }
        _ => {
            return Err(anyhow::anyhow!(
                "first wire message was not a handshake; dropping connection"
            ));
        }
    };

    while let Some(line) = lines.next_line().await? {
        let message: WireMessage = match serde_json::from_str(&line) {
            Ok(message) => message,
            Err(err) => {
                tracing::warn!("dropping malformed wire line: {err}");
                continue;
            }
        };

        if let WireMessage::Event(event) = message {
            store.push_event(&sandbox_id, event);
        }
    }

    store.mark_stopped(&sandbox_id);
    Ok(())
}
