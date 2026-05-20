use std::path::PathBuf;
use std::process::Stdio;

use anyhow::{Context, Result};
use tokio::process::Command;

// ---------------------------------------------------------------------------
// Proxy backend selection
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProxyBackend {
    /// Start the built-in mitmproxy.
    Mitmproxy,
    /// Expect httptoolkit-server to be running separately; events arrive
    /// via the Vetty bridge at POST /api/proxy-events.
    Httptoolkit,
    /// Start mitmproxy + accept events from an external httptoolkit-server.
    Both,
    /// No HTTP interception.
    None,
}

impl ProxyBackend {
    /// Read the configured proxy backend from VETTY_PROXY_BACKEND.
    /// Defaults to mitmproxy when the variable is not set.
    pub fn from_env() -> Self {
        match std::env::var("VETTY_PROXY_BACKEND")
            .unwrap_or_default()
            .to_lowercase()
            .as_str()
        {
            "httptoolkit" | "htk" => Self::Httptoolkit,
            "both" => Self::Both,
            "none" | "off" | "0" | "false" => Self::None,
            "mitmproxy" => Self::Mitmproxy,
            _ => Self::Mitmproxy, // default
        }
    }

    pub fn wants_mitmproxy(self) -> bool {
        matches!(self, Self::Mitmproxy | Self::Both)
    }

    pub fn wants_httptoolkit(self) -> bool {
        matches!(self, Self::Httptoolkit | Self::Both)
    }
}

// ---------------------------------------------------------------------------
// Start selected backend(s)
// ---------------------------------------------------------------------------

/// Start any configured HTTP interception backend(s).
/// Returns Ok even if a backend fails to start (logs a warning).
pub async fn start_proxy_backend(daemon_port: u16) -> Result<()> {
    let backend = ProxyBackend::from_env();

    if backend.wants_mitmproxy() {
        match start_mitmproxy(daemon_port) {
            Ok(()) => tracing::info!("mitmproxy backend started"),
            Err(err) => tracing::warn!("mitmproxy failed to start: {err}"),
        }
    }

    if backend.wants_httptoolkit() {
        match start_httptoolkit_server(daemon_port).await {
            Ok(()) => tracing::info!("httptoolkit-server backend started"),
            Err(err) => tracing::warn!("httptoolkit-server failed to start: {err}"),
        }
    }

    if backend == ProxyBackend::None {
        tracing::info!("HTTP interception disabled (VETTY_PROXY_BACKEND=none)");
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// mitmproxy
// ---------------------------------------------------------------------------

fn start_mitmproxy(daemon_port: u16) -> Result<()> {
    let repo_root = default_repo_root();
    let proxy_port = std::env::var("VETTY_MITM_PORT")
        .unwrap_or_else(|_| "8899".to_string())
        .parse::<u16>()
        .context("invalid VETTY_MITM_PORT")?;
    let confdir = std::env::var("VETTY_MITM_CONFDIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| repo_root.join(".vetty-mitmproxy"));
    let addon = std::env::var("VETTY_MITM_ADDON")
        .map(PathBuf::from)
        .unwrap_or_else(|_| repo_root.join("scripts/vetty-mitmproxy-addon.py"));

    if !addon.is_file() {
        tracing::warn!(
            "mitmproxy addon not found at {}; HTTPS request capture disabled",
            addon.display()
        );
        return Ok(());
    }

    let mut child = Command::new("mitmdump")
        .arg("--listen-host")
        .arg("0.0.0.0")
        .arg("--listen-port")
        .arg(proxy_port.to_string())
        .arg("--set")
        .arg(format!("confdir={}", confdir.display()))
        .arg("-s")
        .arg(&addon)
        .env(
            "VETTY_DAEMON_URL",
            format!("http://127.0.0.1:{daemon_port}/api/proxy-events"),
        )
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::inherit())
        .spawn()
        .context("failed to start mitmdump")?;

    tracing::info!(
        "mitmproxy listening on 0.0.0.0:{proxy_port}; CA: {}/mitmproxy-ca-cert.pem",
        confdir.display()
    );

    tokio::spawn(async move {
        match child.wait().await {
            Ok(status) => tracing::warn!("mitmproxy exited with {status}"),
            Err(err) => tracing::warn!("failed to wait for mitmproxy: {err}"),
        }
    });

    Ok(())
}

// ---------------------------------------------------------------------------
// httptoolkit-server
// ---------------------------------------------------------------------------

async fn start_httptoolkit_server(daemon_port: u16) -> Result<()> {
    let repo_root = default_repo_root();
    let htk_dir = std::env::var("VETTY_HTK_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| repo_root.join("httptoolkit-server"));

    if !htk_dir.join("package.json").is_file() {
        tracing::warn!(
            "httptoolkit-server not found at {}; skipping startup",
            htk_dir.display()
        );
        return Ok(());
    }

    let htk_server_port =
        std::env::var("VETTY_HTK_SERVER_PORT").unwrap_or_else(|_| "45457".to_string());
    let htk_mockttp_port =
        std::env::var("VETTY_HTK_MOCKTTP_PORT").unwrap_or_else(|_| "45456".to_string());
    let htk_sandbox_id = std::env::var("VETTY_SANDBOX_ID").unwrap_or_else(|_| {
        std::env::var("VETTY_HTK_SANDBOX_ID")
            .unwrap_or_else(|_| "00000000-0000-4000-8000-000000000001".to_string())
    });
    let htk_proxy_port =
        std::env::var("VETTY_HTK_PROXY_PORT").unwrap_or_else(|_| "8000".to_string());

    tracing::info!(
        "Starting httptoolkit-server from {} (api: {htk_server_port}, mockttp: {htk_mockttp_port})",
        htk_dir.display()
    );

    let mut child = Command::new("npm")
        .arg("start")
        .arg("--")
        .arg("--server-port")
        .arg(&htk_server_port)
        .arg("--mockttp-port")
        .arg(&htk_mockttp_port)
        .current_dir(&htk_dir)
        .env("VETTY_HTK_AUTO_START", "1")
        .env("VETTY_HTK_PROXY_PORT", &htk_proxy_port)
        .env(
            "VETTY_DAEMON_URL",
            format!("http://127.0.0.1:{daemon_port}/api/proxy-events"),
        )
        .env("VETTY_SANDBOX_ID", htk_sandbox_id)
        .stdin(Stdio::null())
        // Keep stdout/stderr visible for debugging; proxy output is useful.
        .spawn()
        .context("failed to start httptoolkit-server (is Node.js + npm installed?)")?;

    tracing::info!(
        "httptoolkit-server will expose an HTTP(S) proxy on 127.0.0.1:{htk_proxy_port}; \
         Docker API proxy: unix:///tmp/httptoolkit-{htk_proxy_port}-docker.sock"
    );

    tokio::spawn(async move {
        match child.wait().await {
            Ok(status) => tracing::warn!("httptoolkit-server exited with {status}"),
            Err(err) => tracing::warn!("failed to wait for httptoolkit-server: {err}"),
        }
    });

    Ok(())
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn default_repo_root() -> PathBuf {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    if cwd.join("scripts/vetty-mitmproxy-addon.py").is_file() {
        return cwd;
    }

    if let Some(parent) = cwd.parent() {
        if parent.join("scripts/vetty-mitmproxy-addon.py").is_file() {
            return parent.to_path_buf();
        }
    }

    cwd
}
