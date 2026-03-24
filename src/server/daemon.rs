//! Unix socket daemon

use crate::config::Config;
use crate::protocol::{Request, Response};
use crate::server::{Session, SessionEvent};
use crate::state::State;
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::fs::OpenOptions;
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::path::PathBuf;
use tokio::fs;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixListener;
use tokio::sync::{mpsc, oneshot};
use tracing::{debug, error, info, warn};

/// Internal request with response channel
struct InternalRequest {
    request: Request,
    response_tx: oneshot::Sender<Result<Response>>,
}

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
            state_path: State::state_path()
                .unwrap_or_else(|_| PathBuf::from("/tmp/ccmux-state.json")),
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
    request_rx: mpsc::Receiver<InternalRequest>,
    request_tx: mpsc::Sender<InternalRequest>,
}

impl Daemon {
    /// Create a new daemon
    pub fn new(config: DaemonConfig) -> Result<Self> {
        info!("Initializing ccmuxd daemon");
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        let (request_tx, request_rx) = mpsc::channel(32);
        let state = State::load().unwrap_or_else(|e| {
            warn!("Failed to load state, using defaults: {}", e);
            State::default()
        });
        let config_loader = Config::load().unwrap_or_else(|e| {
            warn!("Failed to load config, using defaults: {}", e);
            Config::default()
        });

        debug!(
            "Daemon initialized with socket: {}",
            config.socket_path.display()
        );

        Ok(Self {
            config,
            state,
            config_loader,
            sessions: HashMap::new(),
            event_rx,
            event_tx,
            request_rx,
            request_tx,
        })
    }

    /// Run the daemon
    pub async fn run(mut self) -> Result<()> {
        info!("Starting ccmuxd daemon");

        // Create lockfile atomically to prevent race conditions and symlink attacks
        let lock_path = self.config.socket_path.with_extension("lock");

        // Try to create lockfile atomically
        let lock_file = OpenOptions::new()
            .write(true)
            .create_new(true)  // Atomic - fails if file exists
            .mode(0o600)       // Restrictive permissions
            .open(&lock_path);

        let mut lock_file = match lock_file {
            Ok(file) => file,
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                // Lockfile exists - check if it's stale
                if let Ok(pid_str) = std::fs::read_to_string(&lock_path) {
                    if let Ok(pid) = pid_str.trim().parse::<i32>() {
                        // Check if process is still running
                        if nix::sys::signal::kill(nix::unistd::Pid::from_raw(pid), None).is_ok() {
                            anyhow::bail!("Another daemon is already running (PID {})", pid);
                        }
                        // Process is dead - remove stale lockfile
                        warn!("Removing stale lockfile (PID {} is dead)", pid);
                        std::fs::remove_file(&lock_path)
                            .context("Failed to remove stale lockfile")?;
                        // Retry creating lockfile
                        OpenOptions::new()
                            .write(true)
                            .create_new(true)
                            .mode(0o600)
                            .open(&lock_path)
                            .context("Failed to create lockfile after removing stale one")?
                    } else {
                        anyhow::bail!("Invalid PID in lockfile: {}", pid_str);
                    }
                } else {
                    // Can't read lockfile - remove and retry
                    warn!("Removing unreadable lockfile");
                    std::fs::remove_file(&lock_path)?;
                    OpenOptions::new()
                        .write(true)
                        .create_new(true)
                        .mode(0o600)
                        .open(&lock_path)
                        .context("Failed to create lockfile")?
                }
            }
            Err(e) => return Err(e).context("Failed to create lockfile"),
        };

        // Write our PID to the lockfile
        write!(lock_file, "{}", std::process::id())
            .context("Failed to write PID to lockfile")?;
        lock_file.flush().context("Failed to flush lockfile")?;

        // Ensure lockfile is removed on drop
        let lock_path_clone = lock_path.clone();
        let _guard = scopeguard::guard(lock_path_clone, |path| {
            let _ = std::fs::remove_file(&path);
        });

        // Remove existing socket if present
        if self.config.socket_path.exists() {
            debug!(
                "Removing existing socket: {}",
                self.config.socket_path.display()
            );
            fs::remove_file(&self.config.socket_path)
                .await
                .with_context(|| {
                    format!(
                        "Failed to remove existing socket at {}",
                        self.config.socket_path.display()
                    )
                })?;
        }

        // Create socket directory if needed
        if let Some(parent) = self.config.socket_path.parent() {
            debug!("Creating socket directory: {}", parent.display());
            fs::create_dir_all(parent).await.with_context(|| {
                format!("Failed to create socket directory at {}", parent.display())
            })?;
        }

        let listener = UnixListener::bind(&self.config.socket_path).with_context(|| {
            format!(
                "Failed to bind socket at {}",
                self.config.socket_path.display()
            )
        })?;

        info!("ccmuxd listening on {}", self.config.socket_path.display());

        // Clone request_tx for connection handlers
        let request_tx = self.request_tx.clone();

        loop {
            tokio::select! {
                // Accept new connections
                result = listener.accept() => {
                    match result {
                        Ok((stream, addr)) => {
                            debug!("Accepted connection from {:?}", addr);
                            let request_tx = request_tx.clone();
                            tokio::spawn(async move {
                                if let Err(e) = handle_connection(stream, request_tx).await {
                                    error!("Connection handler error: {}", e);
                                }
                            });
                        }
                        Err(e) => {
                            error!("Failed to accept connection: {}", e);
                        }
                    }
                }

                // Handle internal requests from connection handlers
                Some(internal_req) = self.request_rx.recv() => {
                    let response = self.handle_request(internal_req.request);
                    let _ = internal_req.response_tx.send(response);
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
        // Lockfile is removed automatically by the guard
        self.state
            .save()
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
                // Now using same SessionStatus from protocol
                self.state
                    .update_session_status(&session, status)
                    .with_context(|| format!("Failed to update status for session {}", session))?;
                self.state
                    .save()
                    .context("Failed to save state after status change")?;
            }
            SessionEvent::Terminated { session } => {
                info!("[{}] Session terminated", session);
                self.sessions.remove(&session);
                self.state.remove_session(&session);
                self.state
                    .save()
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
                let sessions: Vec<_> = self.sessions.values().map(|s| s.info()).collect();
                Ok(Response::success(
                    serde_json::to_value(sessions).context("Failed to serialize session list")?,
                ))
            }

            Request::Status { session } => {
                if let Some(name) = session {
                    debug!("Getting status for session: {}", name);
                    if let Some(s) = self.sessions.get(&name) {
                        Ok(Response::success(
                            serde_json::to_value(s.status_detail())
                                .context("Failed to serialize session status detail")?,
                        ))
                    } else {
                        warn!("Session not found: {}", name);
                        Ok(Response::error(format!("Session '{}' not found", name)))
                    }
                } else {
                    debug!("Getting status for all sessions");
                    // When no session specified, return list of status details
                    let details: Vec<_> = self.sessions.values().map(|s| s.status_detail()).collect();
                    Ok(Response::success(
                        serde_json::to_value(details)
                            .context("Failed to serialize status list")?,
                    ))
                }
            }

            Request::New {
                name,
                cwd,
                strategy,
            } => {
                info!("Creating new session: {}", name);
                let cwd =
                    cwd.unwrap_or_else(|| std::env::var("PWD").unwrap_or_else(|_| ".".to_string()));
                let strategy =
                    strategy.unwrap_or_else(|| self.config_loader.default_strategy().to_string());

                let log_path = State::log_path(&name)
                    .with_context(|| format!("Failed to get log path for session {}", name))?;

                let mut session =
                    Session::new(name.clone(), cwd.clone(), strategy, self.event_tx.clone(), log_path)
                        .with_context(|| format!("Failed to create session {}", name))?;

                // Start the session with claude CLI
                let claude_path = which::which("claude")
                    .unwrap_or_else(|_| PathBuf::from("claude"));

                let mut cmd = std::process::Command::new(&claude_path);
                cmd.current_dir(&cwd);

                if let Err(e) = session.start(cmd) {
                    warn!("Failed to start session {}: {}", name, e);
                    // Session is created but not started - user can retry
                }

                let info = session.info();
                self.state.add_session(session.to_state());
                self.sessions.insert(name.clone(), session);
                self.state
                    .save()
                    .context("Failed to save state after creating session")?;

                debug!("Session created successfully: {}", name);
                Ok(Response::success(
                    serde_json::to_value(info).context("Failed to serialize session info")?,
                ))
            }

            Request::Kill { session } => {
                info!("Killing session: {}", session);
                if let Some(mut s) = self.sessions.remove(&session) {
                    s.kill()
                        .with_context(|| format!("Failed to kill session {}", session))?;
                    self.state.remove_session(&session);
                    self.state
                        .save()
                        .context("Failed to save state after killing session")?;
                    Ok(Response::success(serde_json::json!({"killed": session})))
                } else {
                    warn!("Attempted to kill non-existent session: {}", session);
                    Ok(Response::error(format!("Session '{}' not found", session)))
                }
            }

            Request::Send { session, text } => {
                debug!("Sending to session {}: {} bytes", session, text.len());
                if let Some(s) = self.sessions.get_mut(&session) {
                    s.send(&text)
                        .with_context(|| format!("Failed to send to session {}", session))?;
                    Ok(Response::success(serde_json::json!({"sent": true})))
                } else {
                    warn!("Attempted to send to non-existent session: {}", session);
                    Ok(Response::error(format!("Session '{}' not found", session)))
                }
            }

            Request::Output { session, lines } => {
                debug!(
                    "Reading output from session: {} (lines: {:?})",
                    session, lines
                );
                if let Some(s) = self.sessions.get_mut(&session) {
                    let output = s.read_output().with_context(|| {
                        format!("Failed to read output from session {}", session)
                    })?;
                    // For now, just return the current output
                    let lines_vec: Vec<String> = if output.is_empty() {
                        vec![]
                    } else {
                        output
                            .lines()
                            .take(lines.unwrap_or(50))
                            .map(|s| s.to_string())
                            .collect()
                    };
                    Ok(Response::success(
                        serde_json::to_value(lines_vec)
                            .context("Failed to serialize output lines")?,
                    ))
                } else {
                    warn!(
                        "Attempted to read output from non-existent session: {}",
                        session
                    );
                    Ok(Response::error(format!("Session '{}' not found", session)))
                }
            }

            Request::Resize {
                session,
                cols,
                rows,
            } => {
                debug!("Resizing session {}: {}x{}", session, cols, rows);
                if let Some(s) = self.sessions.get_mut(&session) {
                    s.resize(cols, rows)
                        .with_context(|| format!("Failed to resize session {}", session))?;
                    Ok(Response::success(serde_json::json!({"resized": true})))
                } else {
                    warn!("Attempted to resize non-existent session: {}", session);
                    Ok(Response::error(format!("Session '{}' not found", session)))
                }
            }

            Request::StartDaemon => Ok(Response::error("Daemon already running")),

            Request::StopDaemon => {
                // Signal shutdown
                Ok(Response::success(serde_json::json!({"stopping": true})))
            }
        }
    }
}

async fn handle_connection(
    mut stream: tokio::net::UnixStream,
    request_tx: mpsc::Sender<InternalRequest>,
) -> Result<()> {
    debug!("Handling connection");

    // Read until EOF - client should close write side after sending request
    let mut buf = Vec::new();
    stream
        .read_to_end(&mut buf)
        .await
        .context("Failed to read from socket")?;

    let request: Request = serde_json::from_slice(&buf).context("Failed to deserialize request")?;

    debug!("Received request: {:?}", std::mem::discriminant(&request));

    // Send request to main daemon loop and wait for response
    let (response_tx, response_rx) = oneshot::channel();

    request_tx
        .send(InternalRequest {
            request,
            response_tx,
        })
        .await
        .context("Failed to send request to daemon")?;

    let response = response_rx
        .await
        .context("Failed to receive response from daemon")?
        .unwrap_or_else(|e| Response::error(format!("Internal error: {}", e)));

    let response_bytes = serde_json::to_vec(&response).context("Failed to serialize response")?;
    stream
        .write_all(&response_bytes)
        .await
        .context("Failed to write response to socket")?;
    stream.flush().await.context("Failed to flush socket")?;

    debug!("Connection handled successfully");
    Ok(())
}
