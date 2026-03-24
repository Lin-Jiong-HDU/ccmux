//! Session lifecycle management

use crate::protocol::{SessionInfo, SessionStatus};
use crate::server::{Pty, PtySize};
use crate::state::SessionState;
use anyhow::Result;
use chrono::{DateTime, Utc};
use std::path::PathBuf;
use std::process::Command;
use tokio::sync::mpsc;

/// Session handle for external reference
#[derive(Debug, Clone)]
pub struct SessionHandle {
    pub id: String,
}

/// Session event for notifications
#[derive(Debug)]
pub enum SessionEvent {
    Output {
        session: String,
        output: String,
    },
    StatusChanged {
        session: String,
        status: SessionStatus,
    },
    Terminated {
        session: String,
    },
}

/// A managed Claude Code session
pub struct Session {
    pub id: String,
    pub name: String,
    pub cwd: String,
    pub strategy: String,
    pub status: SessionStatus,
    created_at: DateTime<Utc>,
    pty: Option<Pty>,
    event_tx: mpsc::UnboundedSender<SessionEvent>,
    log_path: PathBuf,
    last_output: String,
}

impl Session {
    /// Create a new session (not started yet)
    pub fn new(
        name: String,
        cwd: String,
        strategy: String,
        event_tx: mpsc::UnboundedSender<SessionEvent>,
        log_path: PathBuf,
    ) -> Result<Self> {
        let id = uuid::Uuid::new_v4().to_string();

        Ok(Self {
            id: id.clone(),
            name,
            cwd,
            strategy,
            status: SessionStatus::Stopped,
            created_at: Utc::now(),
            pty: None,
            event_tx,
            log_path,
            last_output: String::new(),
        })
    }

    /// Start the session with a command
    pub fn start(&mut self, cmd: Command) -> Result<()> {
        let pty = Pty::spawn(cmd)?;
        self.pty = Some(pty);
        self.status = SessionStatus::Running;
        let _ = self.event_tx.send(SessionEvent::StatusChanged {
            session: self.id.clone(),
            status: SessionStatus::Running,
        });
        Ok(())
    }

    /// Send input to the session
    pub fn send(&mut self, text: &str) -> Result<()> {
        if let Some(pty) = &mut self.pty {
            pty.write(text.as_bytes())?;
        }
        Ok(())
    }

    /// Read output from the session (non-blocking)
    pub fn read_output(&mut self) -> Result<String> {
        let mut buf = [0u8; 8192];
        if let Some(pty) = &mut self.pty {
            // Set non-blocking mode would be ideal, but for now we just try to read
            match pty.read(&mut buf) {
                Ok(n) if n > 0 => {
                    let output = String::from_utf8_lossy(&buf[..n]).to_string();
                    self.last_output = output.clone();

                    // Notify about output
                    let _ = self.event_tx.send(SessionEvent::Output {
                        session: self.id.clone(),
                        output: output.clone(),
                    });

                    Ok(output)
                }
                _ => Ok(String::new()),
            }
        } else {
            Ok(String::new())
        }
    }

    /// Pause the session
    pub fn pause(&mut self) -> Result<()> {
        if self.status == SessionStatus::Running {
            self.status = SessionStatus::Paused;
            let _ = self.event_tx.send(SessionEvent::StatusChanged {
                session: self.id.clone(),
                status: SessionStatus::Paused,
            });
        }
        Ok(())
    }

    /// Resume the session
    pub fn resume(&mut self) -> Result<()> {
        if self.status == SessionStatus::Paused {
            self.status = SessionStatus::Running;
            let _ = self.event_tx.send(SessionEvent::StatusChanged {
                session: self.id.clone(),
                status: SessionStatus::Running,
            });
        }
        Ok(())
    }

    /// Kill the session
    pub fn kill(&mut self) -> Result<()> {
        self.pty = None; // Drop PTY, which sends SIGHUP
        self.status = SessionStatus::Stopped;
        let _ = self.event_tx.send(SessionEvent::StatusChanged {
            session: self.id.clone(),
            status: SessionStatus::Stopped,
        });
        let _ = self.event_tx.send(SessionEvent::Terminated {
            session: self.id.clone(),
        });
        Ok(())
    }

    /// Resize the PTY
    pub fn resize(&mut self, cols: u16, rows: u16) -> Result<()> {
        if let Some(pty) = &self.pty {
            pty.resize(PtySize { cols, rows })?;
        }
        Ok(())
    }

    /// Get session info
    pub fn info(&self) -> SessionInfo {
        SessionInfo {
            id: self.id.clone(),
            status: self.status,
            pid: self.pty.as_ref().map(|p| p.child_pid().as_raw() as u32),
            cwd: self.cwd.clone(),
            strategy: self.strategy.clone(),
            created_at: self.created_at.to_rfc3339(),
            uptime_secs: Some((Utc::now() - self.created_at).num_seconds() as u64),
            last_output: if self.last_output.is_empty() {
                None
            } else {
                Some(self.last_output.clone())
            },
        }
    }

    /// Convert to state for persistence
    pub fn to_state(&self) -> SessionState {
        SessionState {
            id: self.id.clone(),
            status: match self.status {
                SessionStatus::Running => crate::state::SessionStatus::Running,
                SessionStatus::Paused => crate::state::SessionStatus::Paused,
                SessionStatus::Stopped => crate::state::SessionStatus::Stopped,
            },
            pid: self.pty.as_ref().map(|p| p.child_pid().as_raw() as u32),
            cwd: self.cwd.clone(),
            strategy: self.strategy.clone(),
            created_at: self.created_at.to_rfc3339(),
            log_file: self.log_path.to_string_lossy().to_string(),
        }
    }

    /// Get the session name
    pub fn name(&self) -> &str {
        &self.name
    }
}
