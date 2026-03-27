//! Bypass session - file-based, non-PTY session management

use crate::protocol::{SessionInfo, SessionStatus};
use crate::server::status_file::{StatusFile, BypassStatus};
use anyhow::{Context, Result};
use regex::Regex;
use std::path::PathBuf;

/// A bypass session (no PTY, file-based state)
pub struct BypassSession {
    pub name: String,
    pub cwd: String,
    pub strategy: String,
    pub base_dir: PathBuf,
    status_file: StatusFile,
}

impl BypassSession {
    /// Create a new bypass session
    pub fn new(
        name: String,
        cwd: String,
        strategy: String,
        base_dir: PathBuf,
    ) -> Result<Self> {
        let status_file = StatusFile::new(name.clone(), String::new());

        // Save initial status
        status_file.save(&base_dir)?;

        Ok(Self {
            name,
            cwd,
            strategy,
            base_dir,
            status_file,
        })
    }

    /// Load existing bypass session
    pub fn load(
        name: String,
        cwd: String,
        strategy: String,
        base_dir: PathBuf,
    ) -> Result<Self> {
        let status_file = StatusFile::load(&base_dir, &name)?;

        Ok(Self {
            name,
            cwd,
            strategy,
            base_dir,
            status_file,
        })
    }

    /// Send a task (execute command)
    pub fn send(&mut self, text: &str) -> Result<()> {
        use crate::server::bypass_exec;

        let output_path = StatusFile::output_path(&self.base_dir, &self.name);

        // Truncate output.log before each new task to avoid false-positive pattern matches
        // from previous runs
        if output_path.exists() {
            std::fs::write(&output_path, "")
                .with_context(|| format!("Failed to truncate output file: {:?}", output_path))?;
        }

        // Execute in background
        let pid = bypass_exec::execute_bypass_command(
            &self.name,
            text,
            std::path::Path::new(&self.cwd),
            &output_path,
        )?;

        // Update status
        self.status_file.command = format!(
            "claude --dangerously-skip-permissions {}",
            shell_escape::escape(text.into())
        );
        self.status_file.mark_running(pid);
        self.status_file.save(&self.base_dir)?;

        Ok(())
    }

    /// Get current status
    pub fn status(&self) -> SessionStatus {
        match self.status_file.status {
            BypassStatus::Idle => SessionStatus::Stopped,
            BypassStatus::Running => SessionStatus::Running,
            BypassStatus::Completed | BypassStatus::Failed => SessionStatus::Stopped,
        }
    }

    /// Get session info
    pub fn info(&self) -> SessionInfo {
        SessionInfo {
            id: self.name.clone(),  // Use name as ID for bypass
            status: self.status(),
            pid: self.status_file.pid,
            cwd: self.cwd.clone(),
            strategy: self.strategy.clone(),
            created_at: self.status_file.start_time.clone(),
            uptime_secs: None,  // Not tracked for bypass
            last_output: None,  // Not buffered for bypass
        }
    }

    /// Refresh status from file
    pub fn refresh(&mut self) -> Result<()> {
        self.status_file = StatusFile::load(&self.base_dir, &self.name)?;
        Ok(())
    }

    /// Kill the session (if running)
    pub fn kill(&mut self) -> Result<()> {
        if let Some(pid) = self.status_file.pid {
            if crate::server::bypass_exec::is_process_running(pid) {
                nix::sys::signal::kill(
                    nix::unistd::Pid::from_raw(pid as i32),
                    nix::sys::signal::Signal::SIGTERM,
                )?;
            }
        }

        self.status_file.mark_failed("Killed by user");
        self.status_file.save(&self.base_dir)?;

        Ok(())
    }

    /// Check if completed
    pub fn is_completed(&self) -> bool {
        matches!(
            self.status_file.status,
            BypassStatus::Completed | BypassStatus::Failed
        )
    }

    /// Get last lines from output
    pub fn get_last_lines(&self, count: usize) -> Vec<String> {
        let output_path = StatusFile::output_path(&self.base_dir, &self.name);

        if !output_path.exists() {
            return vec![];
        }

        std::fs::read_to_string(output_path)
            .unwrap_or_default()
            .lines()
            .rev()
            .take(count)
            .map(|s| s.to_string())
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect()
    }

    /// Find pattern in output
    pub fn find_pattern_in_output(&self, pattern: &str) -> bool {
        let re = match Regex::new(pattern) {
            Ok(r) => r,
            Err(_) => return false,
        };

        let output_path = StatusFile::output_path(&self.base_dir, &self.name);

        if !output_path.exists() {
            return false;
        }

        let content = match std::fs::read_to_string(output_path) {
            Ok(c) => c,
            Err(_) => return false,
        };

        re.is_match(&content)
    }

    /// Get the status file (for daemon access)
    pub fn status_file(&self) -> &StatusFile {
        &self.status_file
    }
}
