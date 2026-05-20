use std::io::{self, BufRead};
use std::str::FromStr;

use anyhow::Result;
use vetty_common::{AgentHandshake, SandboxId};

mod strace_parser;
mod vsock_client;

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_target(false)
        .init();

    let parser = strace_parser::StraceParser::new()?;
    let mut client = vsock_client::VsockClient::connect()?;

    let sandbox_id = std::env::var("VETTY_SANDBOX_ID")
        .ok()
        .and_then(|value| SandboxId::from_str(&value).ok())
        .unwrap_or_default();

    let hostname = hostname::get()?.to_string_lossy().to_string();
    let handshake = AgentHandshake {
        sandbox_id,
        agent_version: env!("CARGO_PKG_VERSION").to_string(),
        hostname,
    };
    client.send_handshake(&handshake)?;

    let stdin = io::stdin();

    for line in stdin.lock().lines() {
        let line = line?;
        if let Some(event) = parser.parse_line(&line) {
            client.send_event(&event)?;
            client.flush()?;
        }
    }

    client.flush()?;
    Ok(())
}
