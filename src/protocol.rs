//! Communication protocol between client and server

use serde::{Deserialize, Serialize};

/// Client request sent to server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Request {
    #[serde(rename = "new")]
    New {
        name: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        cwd: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        strategy: Option<String>,
    },
    #[serde(rename = "ls")]
    List,
    #[serde(rename = "kill")]
    Kill { session: String },
    #[serde(rename = "send")]
    Send { session: String, text: String },
    #[serde(rename = "output")]
    Output {
        session: String,
        lines: Option<usize>,
    },
    #[serde(rename = "resize")]
    Resize {
        session: String,
        cols: u16,
        rows: u16,
    },
    #[serde(rename = "status")]
    Status { session: Option<String> },
    #[serde(rename = "start")]
    StartDaemon,
    #[serde(rename = "stop")]
    StopDaemon,
}

/// Server response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl Response {
    pub fn success(data: impl Into<serde_json::Value>) -> Self {
        Self {
            success: true,
            data: Some(data.into()),
            error: None,
        }
    }

    pub fn error(error: impl Into<String>) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(error.into()),
        }
    }
}

/// Session status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionStatus {
    #[serde(rename = "running")]
    Running,
    #[serde(rename = "paused")]
    Paused,
    #[serde(rename = "stopped")]
    Stopped,
}

impl std::fmt::Display for SessionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Running => write!(f, "running"),
            Self::Paused => write!(f, "paused"),
            Self::Stopped => write!(f, "stopped"),
        }
    }
}

/// Session information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    pub id: String,
    pub status: SessionStatus,
    pub pid: Option<u32>,
    pub cwd: String,
    pub strategy: String,
    pub created_at: String,
    pub uptime_secs: Option<u64>,
    pub last_output: Option<String>,
}

/// List sessions response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionList {
    pub sessions: Vec<SessionInfo>,
}

/// Status response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionStatusDetail {
    pub session: String,
    pub status: SessionStatus,
    pub strategy: String,
    pub uptime: String,
    pub cwd: String,
    pub pid: Option<u32>,
    pub last_lines: Vec<String>,
}
