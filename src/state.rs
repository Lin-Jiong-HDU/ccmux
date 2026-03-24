//! State persistence

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::Path;

// Re-export SessionStatus from protocol for backward compatibility
pub use crate::protocol::SessionStatus;

/// Persisted session state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionState {
    pub id: String,
    pub status: SessionStatus,
    pub pid: Option<u32>,
    pub cwd: String,
    pub strategy: String,
    pub created_at: String,
    pub log_file: String,
}

/// Global state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct State {
    pub sessions: HashMap<String, SessionState>,
}

impl Default for State {
    fn default() -> Self {
        Self::new()
    }
}

impl State {
    pub fn new() -> Self {
        Self {
            sessions: HashMap::new(),
        }
    }

    pub fn add_session(&mut self, session: SessionState) {
        self.sessions.insert(session.id.clone(), session);
    }

    pub fn remove_session(&mut self, id: &str) -> Option<SessionState> {
        self.sessions.remove(id)
    }

    pub fn get_session(&self, id: &str) -> Option<&SessionState> {
        self.sessions.get(id)
    }

    pub fn update_session_status(&mut self, id: &str, status: SessionStatus) -> Result<()> {
        if let Some(session) = self.sessions.get_mut(id) {
            session.status = status;
            Ok(())
        } else {
            anyhow::bail!("Session not found: {}", id)
        }
    }

    pub fn save_to(&self, path: impl AsRef<Path>) -> Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        let mut file = fs::File::create(path.as_ref())?;
        file.write_all(json.as_bytes())?;
        Ok(())
    }

    pub fn load_from(path: impl AsRef<Path>) -> Result<Self> {
        let content = fs::read_to_string(path.as_ref())?;
        let state: State = serde_json::from_str(&content)?;
        Ok(state)
    }

    pub fn state_dir() -> anyhow::Result<std::path::PathBuf> {
        let base = dirs::data_local_dir()
            .ok_or_else(|| anyhow::anyhow!("Cannot determine data directory"))?;
        Ok(base.join("ccmux"))
    }

    pub fn state_path() -> anyhow::Result<std::path::PathBuf> {
        Ok(Self::state_dir()?.join("state.json"))
    }

    pub fn logs_dir() -> anyhow::Result<std::path::PathBuf> {
        Ok(Self::state_dir()?.join("logs"))
    }

    pub fn log_path(session_id: &str) -> anyhow::Result<std::path::PathBuf> {
        Ok(Self::logs_dir()?.join(format!("{}.log", session_id)))
    }

    pub fn load() -> Result<Self> {
        let path = Self::state_path()?;
        if path.exists() {
            Self::load_from(path)
        } else {
            Ok(Self::new())
        }
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::state_path()?;
        // Ensure directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        self.save_to(path)
    }
}
