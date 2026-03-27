//! Status file management for bypass sessions

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Session status for bypass mode
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum BypassStatus {
    Idle,
    Running,
    Completed,
    Failed,
}

/// Status file content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusFile {
    pub name: String,
    pub status: BypassStatus,
    pub exit_code: Option<i32>,
    pub pid: Option<u32>,
    pub command: String,
    pub start_time: String,  // ISO 8601
    pub end_time: Option<String>,  // ISO 8601
}

impl StatusFile {
    /// Create a new status file entry
    pub fn new(name: String, command: String) -> Self {
        Self {
            name,
            status: BypassStatus::Idle,
            exit_code: None,
            pid: None,
            command,
            start_time: chrono::Utc::now().to_rfc3339(),
            end_time: None,
        }
    }

    /// Get the session directory path
    pub fn session_dir(base: &Path, name: &str) -> PathBuf {
        base.join("sessions").join(name)
    }

    /// Get the status file path
    pub fn status_path(base: &Path, name: &str) -> PathBuf {
        Self::session_dir(base, name).join("status.json")
    }

    /// Get the output log path
    pub fn output_path(base: &Path, name: &str) -> PathBuf {
        Self::session_dir(base, name).join("output.log")
    }

    /// Save status to file
    pub fn save(&self, base: &Path) -> Result<()> {
        let path = Self::status_path(base, &self.name);

        // Ensure directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory: {:?}", parent))?;
        }

        let content = serde_json::to_string_pretty(self)
            .context("Failed to serialize status")?;

        fs::write(&path, content)
            .with_context(|| format!("Failed to write status file: {:?}", path))?;

        Ok(())
    }

    /// Load status from file
    pub fn load(base: &Path, name: &str) -> Result<Self> {
        let path = Self::status_path(base, name);

        let content = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read status file: {:?}", path))?;

        let status: StatusFile = serde_json::from_str(&content)
            .with_context(|| format!("Failed to parse status file: {:?}", path))?;

        Ok(status)
    }

    /// Check if status file exists
    pub fn exists(base: &Path, name: &str) -> bool {
        Self::status_path(base, name).exists()
    }

    /// Mark as running with PID
    pub fn mark_running(&mut self, pid: u32) {
        self.status = BypassStatus::Running;
        self.pid = Some(pid);
    }

    /// Mark as completed
    pub fn mark_completed(&mut self, exit_code: i32) {
        self.status = if exit_code == 0 {
            BypassStatus::Completed
        } else {
            BypassStatus::Failed
        };
        self.exit_code = Some(exit_code);
        self.end_time = Some(chrono::Utc::now().to_rfc3339());
        self.pid = None;
    }

    /// Mark as failed
    pub fn mark_failed(&mut self, _reason: &str) {
        self.status = BypassStatus::Failed;
        self.exit_code = Some(1);
        self.end_time = Some(chrono::Utc::now().to_rfc3339());
        self.pid = None;
    }
}
