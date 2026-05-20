use std::io::{BufWriter, Write};

use anyhow::Result;
use socket2::{Domain, SockAddr, Socket, Type};
use vetty_common::{AgentHandshake, SandboxEvent, WireMessage, HOST_CID, VSOCK_PORT};

pub struct VsockClient {
    writer: BufWriter<Socket>,
}

impl VsockClient {
    pub fn connect() -> Result<Self> {
        let socket = Socket::new(Domain::VSOCK, Type::STREAM, None)?;
        let addr = SockAddr::vsock(HOST_CID, VSOCK_PORT);
        socket.connect(&addr)?;
        Ok(Self {
            writer: BufWriter::new(socket),
        })
    }

    pub fn send_handshake(&mut self, handshake: &AgentHandshake) -> Result<()> {
        let line = serde_json::to_string(&WireMessage::Handshake(handshake.clone()))?;
        writeln!(self.writer, "{line}")?;
        self.writer.flush()?;
        Ok(())
    }

    pub fn send_event(&mut self, event: &SandboxEvent) -> Result<()> {
        let line = serde_json::to_string(&WireMessage::Event(event.clone()))?;
        writeln!(self.writer, "{line}")?;
        Ok(())
    }

    pub fn flush(&mut self) -> Result<()> {
        self.writer.flush()?;
        Ok(())
    }
}
