//! Unix socket daemon

use crate::protocol::{Request, Response, SessionStatus, SessionInfo};
use crate::state::State;
use crate::server::{Session, SessionEvent};
use crate::config::Config;
use anyhow::Result;
use tokio::net::UnixListener;
use tokio::sync::mpsc;
use std::path::PathBuf;
use std::collections::HashMap;
use tokio::fs;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tracing::{info, error, debug};

/// Daemon configuration
#[derive(Debug, Clone)]
pub struct DaemonConfig {
    pub socket_path: PathBuf,
    pub state_path: PathBuf,
}

impl Default for DaemonConfig {
    fn default() -> Self {
        let runtime_dir = std::env::var("XDG_RUNTIME_DIR")
            .or_else(|_| std::env::var("TMPDIR"))
            .unwrap_or_else(|_| "/tmp".to_string());

        Self {
            socket_path: PathBuf::from(runtime_dir).join("ccmux.sock"),
            state_path: State::state_path().unwrap_or_else(|_| PathBuf::from("/tmp/ccmux-state.json")),
        }
    }
}

/// Main daemon
pub struct Daemon {
    config: DaemonConfig,
    state: State,
    config_loader: Config,
    sessions: HashMap<String, Session>,
    event_rx: mpsc::UnboundedReceiver<SessionEvent>,
    event_tx: mpsc::UnboundedSender<SessionEvent>,
}

impl Daemon {
    /// Create a new daemon
    pub fn new(config: DaemonConfig) -> Result<Self> {
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        let state = State::load().unwrap_or_default();
        let config_loader = Config::load().unwrap_or_default();

        Ok(Self {
            config,
            state,
            config_loader,
            sessions: HashMap::new(),
            event_rx,
            event_tx,
        })
    }

    /// Run the daemon
    pub async fn run(mut self) -> Result<()> {
        // Remove existing socket if present
        if self.config.socket_path.exists() {
            fs::remove_file(&self.config.socket_path).await?;
        }

        // Create socket directory if needed
        if let Some(parent) = self.config.socket_path.parent() {
            fs::create_dir_all(parent).await?;
        }

        let listener = UnixListener::bind(&self.config.socket_path)?;

        info!("ccmuxd listening on {}", self.config.socket_path.display());

        loop {
            tokio::select! {
                // Accept new connections
                result = listener.accept() => {
                    match result {
                        Ok((stream, _addr)) => {
                            let config = self.config.clone();
                            tokio::spawn(async move {
                                if let Err(e) = handle_connection(stream, config).await {
                                    error!("Connection error: {}", e);
                                }
                            });
                        }
                        Err(e) => {
                            error!("Connection error: {}", e);
                        }
                    }
                }

                // Handle session events
                Some(event) = self.event_rx.recv() => {
                    if let Err(e) = self.handle_session_event(event).await {
                        error!("Session event error: {}", e);
                    }
                }

                // Shutdown signal
                _ = tokio::signal::ctrl_c() => {
                    info!("Shutting down...");
                    break;
                }
            }
        }

        // Cleanup
        fs::remove_file(&self.config.socket_path).await.ok();
        self.state.save()?;

        Ok(())
    }

    async fn handle_session_event(&mut self, event: SessionEvent) -> Result<()> {
        match event {
            SessionEvent::Output { session, output } => {
                debug!("[{}] Output: {} bytes", session, output.len());
            }
            SessionEvent::StatusChanged { session, status } => {
                info!("[{}] Status: {}", session, status);
                let state_status = match status {
                    SessionStatus::Running => crate::state::SessionStatus::Running,
                    SessionStatus::Paused => crate::state::SessionStatus::Paused,
                    SessionStatus::Stopped => crate::state::SessionStatus::Stopped,
                };
                self.state.update_session_status(&session, state_status)?;
                self.state.save()?;
            }
            SessionEvent::Terminated { session } => {
                info!("[{}] Terminated", session);
                self.sessions.remove(&session);
                self.state.remove_session(&session);
                self.state.save()?;
            }
        }
        Ok(())
    }

    /// Handle a request synchronously (for simpler operations)
    pub fn handle_request(&mut self, request: Request) -> Result<Response> {
        match request {
            Request::List => {
                let sessions: Vec<_> = self.sessions.values()
                    .map(|s| s.info())
                    .collect();
                Ok(Response::success(serde_json::to_value(sessions)?))
            }

            Request::Status { session } => {
                if let Some(name) = session {
                    if let Some(s) = self.sessions.get(&name) {
                        Ok(Response::success(serde_json::to_value(s.info())?))
                    } else {
                        Ok(Response::error("Session not found"))
                    }
                } else {
                    let sessions: Vec<_> = self.sessions.values()
                        .map(|s| s.info())
                        .collect();
                    Ok(Response::success(serde_json::to_value(sessions)?))
                }
            }

            Request::New { name, cwd, strategy } => {
                let cwd = cwd.unwrap_or_else(|| std::env::var("PWD").unwrap_or_else(|_| ".".to_string()));
                let strategy = strategy.unwrap_or_else(|| self.config_loader.default_strategy().to_string());

                let log_path = State::log_path(&name)?;

                let session = Session::new(
                    name.clone(),
                    cwd,
                    strategy,
                    self.event_tx.clone(),
                    log_path,
                )?;

                let info = session.info();
                self.state.add_session(session.to_state());
                self.sessions.insert(name.clone(), session);
                self.state.save()?;

                Ok(Response::success(serde_json::to_value(info)?))
            }

            Request::Kill { session } => {
                if let Some(mut s) = self.sessions.remove(&session) {
                    s.kill()?;
                    self.state.remove_session(&session);
                    self.state.save()?;
                    Ok(Response::success(serde_json::json!({"killed": session})))
                } else {
                    Ok(Response::error("Session not found"))
                }
            }

            Request::Send { session, text } => {
                if let Some(s) = self.sessions.get_mut(&session) {
                    s.send(&text)?;
                    Ok(Response::success(serde_json::json!({"sent": true})))
                } else {
                    Ok(Response::error("Session not found"))
                }
            }

            Request::Output { session, lines } => {
                if let Some(s) = self.sessions.get_mut(&session) {
                    let output = s.read_output()?;
                    // For now, just return the current output
                    let lines_vec: Vec<String> = if output.is_empty() {
                        vec![]
                    } else {
                        output.lines().take(lines.unwrap_or(50)).map(|s| s.to_string()).collect()
                    };
                    Ok(Response::success(serde_json::to_value(lines_vec)?))
                } else {
                    Ok(Response::error("Session not found"))
                }
            }

            Request::Resize { session, cols, rows } => {
                if let Some(s) = self.sessions.get_mut(&session) {
                    s.resize(cols, rows)?;
                    Ok(Response::success(serde_json::json!({"resized": true})))
                } else {
                    Ok(Response::error("Session not found"))
                }
            }

            Request::StartDaemon => {
                Ok(Response::error("Daemon already running"))
            }

            Request::StopDaemon => {
                // Signal shutdown
                Ok(Response::success(serde_json::json!({"stopping": true})))
            }
        }
    }
}

async fn handle_connection(mut stream: tokio::net::UnixStream, _config: DaemonConfig) -> Result<()> {
    let mut buf = vec![0u8; 65536];
    let n = stream.read(&mut buf).await?;
    buf.truncate(n);

    let request: Request = serde_json::from_slice(&buf)?;

    // For now, handle simple requests that don't need session state
    // Full implementation would need to communicate with the main daemon loop
    let response = match request {
        Request::List => {
            // Return empty list for now - this would need proper implementation
            Response::success(serde_json::to_value(Vec::<SessionInfo>::new())?)
        }
        _ => {
            Response::error("This command requires daemon mode. Please use 'ccmuxd' directly.")
        }
    };

    let response_bytes = serde_json::to_vec(&response)?;
    stream.write_all(&response_bytes).await?;
    stream.flush().await?;

    Ok(())
}
