//! Session lifecycle management

use crate::protocol::{Key, ScreenContent, SessionInfo, SessionStatus, SessionStatusDetail};
use crate::server::{InteractionDetector, Pty, PtySize, ScreenBuffer};
use crate::state::SessionState;
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use std::collections::VecDeque;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::mpsc;

/// Timestamped output chunk (stores PTY read() result, not individual lines)
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct TimestampedOutput {
    pub(crate) ts: u64,  // Unix timestamp (milliseconds)
    pub(crate) text: String,
}

/// Output buffer storing the last N output chunks
#[derive(Debug, Clone)]
pub(crate) struct OutputBuffer {
    buffer: VecDeque<TimestampedOutput>,
    max_lines: usize,
}

impl OutputBuffer {
    pub(crate) fn new(max_lines: usize) -> Self {
        Self {
            buffer: VecDeque::with_capacity(max_lines),
            max_lines,
        }
    }

    pub(crate) fn push(&mut self, text: String) {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        if self.buffer.len() >= self.max_lines {
            self.buffer.pop_front();
        }
        self.buffer.push_back(TimestampedOutput { ts, text });
    }

    pub(crate) fn since(&self, ts: u64) -> Vec<&TimestampedOutput> {
        self.buffer.iter().filter(|o| o.ts > ts).collect()
    }

    pub(crate) fn find_pattern(&self, pattern: &str) -> Option<&TimestampedOutput> {
        let re = regex::Regex::new(pattern).ok()?;
        self.buffer.iter().rev().find(|o| re.is_match(&o.text))
    }
}

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
    output_buffer: OutputBuffer,
    screen_buffer: ScreenBuffer,
    mode_detector: InteractionDetector,
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

        // Initialize screen buffer with default size
        let screen_buffer = ScreenBuffer::new(80, 24);
        let mode_detector = InteractionDetector::new();

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
            output_buffer: OutputBuffer::new(1000),  // Store last 1000 output chunks
            screen_buffer,
            mode_detector,
        })
    }

    /// Start the session with a command
    pub fn start(&mut self, cmd: Command) -> Result<()> {
        let pty = Pty::spawn(cmd)?;
        self.pty = Some(pty);
        self.status = SessionStatus::Running;
        let _ = self.event_tx.send(SessionEvent::StatusChanged {
            session: self.name.clone(),  // Use name for daemon lookup
            status: SessionStatus::Running,
        });
        Ok(())
    }

    /// Send input to the session (appends carriage return to submit)
    pub fn send(&mut self, text: &str) -> Result<()> {
        if let Some(pty) = &mut self.pty {
            pty.write(text.as_bytes())?;
            pty.write(b"\r")?; // Send carriage return (Enter key) to submit input
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

                    // Update screen buffer with new output
                    if let Err(e) = self.screen_buffer.process_output(&buf[..n]) {
                        tracing::warn!("Failed to update screen buffer: {}", e);
                    }

                    // Update interaction mode
                    let new_mode = self.mode_detector.detect(&output, self.screen_buffer.detect_mode());
                    self.screen_buffer.set_mode(new_mode);

                    self.last_output = output.clone();

                    // Add to output buffer
                    if !output.is_empty() {
                        self.output_buffer.push(output.clone());
                    }

                    // Write to log file
                    if let Err(e) = self.append_log(&output) {
                        tracing::warn!("Failed to write to log file: {}", e);
                    }

                    // Notify about output
                    let _ = self.event_tx.send(SessionEvent::Output {
                        session: self.name.clone(),  // Use name for daemon lookup
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

    /// Append output to log file
    fn append_log(&self, output: &str) -> Result<()> {
        use std::fs::OpenOptions;
        use std::io::Write;

        // Ensure log directory exists
        if let Some(parent) = self.log_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_path)?;

        write!(file, "{}", output)?;
        file.flush()?;
        Ok(())
    }

    /// Pause the session
    pub fn pause(&mut self) -> Result<()> {
        if self.status == SessionStatus::Running {
            self.status = SessionStatus::Paused;
            let _ = self.event_tx.send(SessionEvent::StatusChanged {
                session: self.name.clone(),  // Use name for daemon lookup
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
                session: self.name.clone(),  // Use name for daemon lookup
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
            session: self.name.clone(),  // Use name for daemon lookup
            status: SessionStatus::Stopped,
        });
        let _ = self.event_tx.send(SessionEvent::Terminated {
            session: self.name.clone(),  // Use name for daemon lookup
        });
        Ok(())
    }

    /// Resize the PTY
    pub fn resize(&mut self, cols: u16, rows: u16) -> Result<()> {
        if let Some(pty) = &mut self.pty {
            pty.resize(PtySize { cols, rows })?;
        }
        // Update screen buffer size
        self.screen_buffer = ScreenBuffer::new(cols, rows);
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

    /// Get session status detail (for status command)
    pub fn status_detail(&self) -> SessionStatusDetail {
        let uptime_secs = (Utc::now() - self.created_at).num_seconds() as u64;
        let uptime = format!("{}h {}m", uptime_secs / 3600, (uptime_secs % 3600) / 60);

        // Get last lines from log file if it exists
        let last_lines = if self.log_path.exists() {
            std::fs::read_to_string(&self.log_path)
                .unwrap_or_default()
                .lines()
                .rev()
                .take(10)
                .map(|s| s.to_string())
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect()
        } else {
            vec![]
        };

        SessionStatusDetail {
            session: self.name.clone(),
            status: self.status,
            strategy: self.strategy.clone(),
            uptime,
            cwd: self.cwd.clone(),
            pid: self.pty.as_ref().map(|p| p.child_pid().as_raw() as u32),
            last_lines,
        }
    }

    /// Convert to state for persistence
    pub fn to_state(&self) -> SessionState {
        SessionState {
            id: self.id.clone(),
            status: self.status, // Now using same SessionStatus from protocol
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

    /// Get the output buffer (pub for daemon access within crate)
    pub(crate) fn output_buffer(&self) -> &OutputBuffer {
        &self.output_buffer
    }

    /// Send control key to the session
    pub fn send_key(&mut self, key: &Key) -> Result<()> {
        if let Some(pty) = &mut self.pty {
            let bytes = key.to_bytes();
            pty.write_raw(&bytes)
                .with_context(|| format!("Failed to send key: {:?}", key))?;
        }
        Ok(())
    }

    /// Get current screen content
    pub fn get_screen(&self) -> ScreenContent {
        self.screen_buffer.get_content()
    }
}
