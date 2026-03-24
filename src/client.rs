//! Client for communicating with daemon

use crate::protocol::{Request, Response, SessionInfo, SessionStatusDetail};
use anyhow::{Context, Result};
use std::path::PathBuf;
use tracing::debug;

/// Client for ccmux daemon
pub struct Client {
    socket: PathBuf,
}

impl Client {
    /// Create a new client
    pub fn new() -> Result<Self> {
        let runtime_dir = std::env::var("XDG_RUNTIME_DIR")
            .or_else(|_| std::env::var("TMPDIR"))
            .unwrap_or_else(|_| "/tmp".to_string());

        let socket_path = PathBuf::from(runtime_dir).join("ccmux.sock");
        debug!("Creating client with socket: {}", socket_path.display());

        Ok(Self {
            socket: socket_path,
        })
    }

    /// Get the socket path
    pub fn socket_path(&self) -> &PathBuf {
        &self.socket
    }

    /// Check if daemon is running (socket exists)
    pub fn is_daemon_running(&self) -> bool {
        self.socket.exists()
    }

    /// Send a request and get response (blocking)
    fn send_request(&self, request: Request) -> Result<Response> {
        use std::io::{Read, Write};
        use std::os::unix::net::UnixStream;

        let mut stream = UnixStream::connect(&self.socket).with_context(|| {
            format!(
                "Failed to connect to daemon socket at {}",
                self.socket.display()
            )
        })?;

        let request_bytes = serde_json::to_vec(&request).context("Failed to serialize request")?;
        stream
            .write_all(&request_bytes)
            .context("Failed to write request to socket")?;
        stream.flush().context("Failed to flush socket")?;

        let mut buf = Vec::new();
        stream
            .read_to_end(&mut buf)
            .context("Failed to read response from socket")?;

        let response: Response =
            serde_json::from_slice(&buf).context("Failed to deserialize response")?;
        Ok(response)
    }

    /// List all sessions
    pub fn list_sessions(&self) -> Result<Vec<SessionInfo>> {
        let response = self.send_request(Request::List)?;
        if response.success {
            Ok(serde_json::from_value(response.data.unwrap_or_default())?)
        } else {
            anyhow::bail!("{}", response.error.unwrap_or_default())
        }
    }

    /// Get session status
    pub fn get_status(&self, session: Option<String>) -> Result<SessionStatusDetail> {
        let response = self.send_request(Request::Status { session })?;
        if response.success {
            Ok(serde_json::from_value(response.data.unwrap_or_default())?)
        } else {
            anyhow::bail!("{}", response.error.unwrap_or_default())
        }
    }

    /// Create a new session
    pub fn new_session(
        &self,
        name: String,
        cwd: Option<String>,
        strategy: Option<String>,
    ) -> Result<SessionInfo> {
        let response = self.send_request(Request::New {
            name,
            cwd,
            strategy,
        })?;
        if response.success {
            Ok(serde_json::from_value(response.data.unwrap_or_default())?)
        } else {
            anyhow::bail!("{}", response.error.unwrap_or_default())
        }
    }

    /// Kill a session
    pub fn kill_session(&self, session: String) -> Result<()> {
        let response = self.send_request(Request::Kill { session })?;
        if response.success {
            Ok(())
        } else {
            anyhow::bail!("{}", response.error.unwrap_or_default())
        }
    }

    /// Send input to a session
    pub fn send_input(&self, session: String, text: String) -> Result<()> {
        let response = self.send_request(Request::Send { session, text })?;
        if response.success {
            Ok(())
        } else {
            anyhow::bail!("{}", response.error.unwrap_or_default())
        }
    }

    /// Get session output
    pub fn get_output(&self, session: String, lines: Option<usize>) -> Result<Vec<String>> {
        let response = self.send_request(Request::Output { session, lines })?;
        if response.success {
            Ok(serde_json::from_value(response.data.unwrap_or_default())?)
        } else {
            anyhow::bail!("{}", response.error.unwrap_or_default())
        }
    }

    /// Resize session PTY
    pub fn resize_session(&self, session: String, cols: u16, rows: u16) -> Result<()> {
        let response = self.send_request(Request::Resize {
            session,
            cols,
            rows,
        })?;
        if response.success {
            Ok(())
        } else {
            anyhow::bail!("{}", response.error.unwrap_or_default())
        }
    }
}

impl Default for Client {
    fn default() -> Self {
        Self::new().expect("Failed to create client")
    }
}
