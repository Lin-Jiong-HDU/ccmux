//! Unix socket daemon

use crate::protocol::{Request, Response, SessionStatus, SessionInfo};
use crate::state::State;
use crate::server::{Session, SessionEvent};
use crate::config::Config;
use anyhow::{Context, Result};
use tokio::net::UnixListener;
use tokio::sync::mpsc;
use std::path::PathBuf;
use std::collections::HashMap;
use tokio::fs;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tracing::{info, error, debug, warn};

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
        info!("Initializing ccmuxd daemon");
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        let state = State::load().unwrap_or_else(|e| {
            warn!("Failed to load state, using defaults: {}", e);
            State::default()
        });
        let config_loader = Config::load().unwrap_or_else(|e| {
            warn!("Failed to load config, using defaults: {}", e);
            Config::default()
        });

        debug!("Daemon initialized with socket: {}", config.socket_path.display());

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
        info!("Starting ccmuxd daemon");

        // Remove existing socket if present
        if self.config.socket_path.exists() {
            debug!("Removing existing socket: {}", self.config.socket_path.display());
            fs::remove_file(&self.config.socket_path).await
                .with_context(|| format!("Failed to remove existing socket at {}", self.config.socket_path.display()))?;
        }

        // Create socket directory if needed
        if let Some(parent) = self.config.socket_path.parent() {
            debug!("Creating socket directory: {}", parent.display());
            fs::create_dir_all(parent).await
                .with_context(|| format!("Failed to create socket directory at {}", parent.display()))?;
        }

        let listener = UnixListener::bind(&self.config.socket_path)
            .with_context(|| format!("Failed to bind socket at {}", self.config.socket_path.display()))?;

        info!("ccmuxd listening on {}", self.config.socket_path.display());

        loop {
            tokio::select! {
                // Accept new connections
                result = listener.accept() => {
                    match result {
                        Ok((stream, addr)) => {
                            debug!("Accepted connection from {:?}", addr);
                            let config = self.config.clone();
                            tokio::spawn(async move {
                                if let Err(e) = handle_connection(stream, config).await {
                                    error!("Connection handler error: {}", e);
                                }
                            });
                        }
                        Err(e) => {
                            error!("Failed to accept connection: {}", e);
                        }
                    }
                }

                // Handle session events
                Some(event) = self.event_rx.recv() => {
                    if let Err(e) = self.handle_session_event(event).await {
                        error!("Session event handler error: {}", e);
                    }
                }

                // Shutdown signal
                _ = tokio::signal::ctrl_c() => {
                    info!("Received shutdown signal");
                    break;
                }
            }
        }

        // Cleanup
        info!("Cleaning up socket and saving state");
        if let Err(e) = fs::remove_file(&self.config.socket_path).await {
            warn!("Failed to remove socket: {}", e);
        }
        self.state.save()
            .context("Failed to save state during shutdown")?;

        info!("Daemon shutdown complete");
        Ok(())
    }

    async fn handle_session_event(&mut self, event: SessionEvent) -> Result<()> {
        match event {
            SessionEvent::Output { session, output } => {
                debug!("[{}] Output: {} bytes", session, output.len());
            }
            SessionEvent::StatusChanged { session, status } => {
                info!("[{}] Status changed to: {}", session, status);
                let state_status = match status {
                    SessionStatus::Running => crate::state::SessionStatus::Running,
                    SessionStatus::Paused => crate::state::SessionStatus::Paused,
                    SessionStatus::Stopped => crate::state::SessionStatus::Stopped,
                };
                self.state.update_session_status(&session, state_status)
                    .with_context(|| format!("Failed to update status for session {}", session))?;
                self.state.save()
                    .context("Failed to save state after status change")?;
            }
            SessionEvent::Terminated { session } => {
                info!("[{}] Session terminated", session);
                self.sessions.remove(&session);
                self.state.remove_session(&session);
                self.state.save()
                    .context("Failed to save state after session termination")?;
            }
        }
        Ok(())
    }

    /// Handle a request synchronously (for simpler operations)
    pub fn handle_request(&mut self, request: Request) -> Result<Response> {
        debug!("Handling request: {:?}", std::mem::discriminant(&request));

        match request {
            Request::List => {
                debug!("Listing sessions");
                let sessions: Vec<_> = self.sessions.values()
                    .map(|s| s.info())
                    .collect();
                Ok(Response::success(serde_json::to_value(sessions)
                    .context("Failed to serialize session list")?))
            }

            Request::Status { session } => {
                if let Some(name) = session {
                    debug!("Getting status for session: {}", name);
                    if let Some(s) = self.sessions.get(&name) {
                        Ok(Response::success(serde_json::to_value(s.info())
                            .context("Failed to serialize session info")?))
                    } else {
                        warn!("Session not found: {}", name);
                        Ok(Response::error(format!("Session '{}' not found", name)))
                    }
                } else {
                    debug!("Getting status for all sessions");
                    let sessions: Vec<_> = self.sessions.values()
                        .map(|s| s.info())
                        .collect();
                    Ok(Response::success(serde_json::to_value(sessions)
                        .context("Failed to serialize session list")?))
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
