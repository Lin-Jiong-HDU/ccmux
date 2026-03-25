//! Client for communicating with daemon

use crate::protocol::{Request, Response, SessionInfo, SessionStatusDetail, StreamEvent, WaitResult};
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
        use std::net::Shutdown;
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

        // Shutdown write side to signal EOF to server
        stream
            .shutdown(Shutdown::Write)
            .context("Failed to shutdown write side")?;

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

    /// Subscribe to session output stream (returns events since timestamp)
    pub fn subscribe(&self, session: &str, since: Option<u64>) -> Result<Vec<StreamEvent>> {
        let response = self.send_request(Request::Subscribe {
            session: session.to_string(),
            since,
        })?;

        if response.success {
            Ok(serde_json::from_value(response.data.unwrap_or_default())?)
        } else {
            anyhow::bail!("{}", response.error.unwrap_or_default())
        }
    }

    /// Wait for a pattern in session output
    pub fn wait(&self, session: &str, pattern: &str, timeout: Option<u64>) -> Result<WaitResult> {
        let response = self.send_request(Request::Wait {
            session: session.to_string(),
            pattern: pattern.to_string(),
            timeout,
        })?;

        if response.success {
            Ok(serde_json::from_value(response.data.unwrap_or_default())?)
        } else {
            anyhow::bail!("{}", response.error.unwrap_or_default())
        }
    }

    /// Wait with polling (for patterns not yet in buffer)
    pub fn wait_with_poll(&self, session: &str, pattern: &str, timeout_ms: u64) -> Result<WaitResult> {
        let start = std::time::Instant::now();
        let poll_interval = 100; // ms

        loop {
            let result = self.wait(session, pattern, None)?;

            if result.matched {
                return Ok(result);
            }

            if start.elapsed().as_millis() as u64 >= timeout_ms {
                return Ok(WaitResult {
                    matched: false,
                    pattern: Some(pattern.to_string()),
                    output: None,
                    timestamp: None,
                });
            }

            std::thread::sleep(std::time::Duration::from_millis(poll_interval));
        }
    }
}

impl Default for Client {
    fn default() -> Self {
        // Default to /tmp if environment variables are not set
        let runtime_dir = std::env::var("XDG_RUNTIME_DIR")
            .or_else(|_| std::env::var("TMPDIR"))
            .unwrap_or_else(|_| "/tmp".to_string());

        Self {
            socket: PathBuf::from(runtime_dir).join("ccmux.sock"),
        }
    }
}
