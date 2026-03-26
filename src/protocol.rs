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
    #[serde(rename = "subscribe")]
    Subscribe {
        session: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        since: Option<u64>,
    },
    #[serde(rename = "wait")]
    Wait { session: String, pattern: String },
    #[serde(rename = "send_key")]
    SendKey { session: String, key: Key },
    #[serde(rename = "get_screen")]
    GetScreen { session: String },
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

/// Stream event for subscribe command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ts: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<SessionStatus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// Wait command result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaitResult {
    pub matched: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pattern: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<u64>,
}

/// Control key for interactive input
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Key {
    #[serde(rename = "up")]
    Up,
    #[serde(rename = "down")]
    Down,
    #[serde(rename = "left")]
    Left,
    #[serde(rename = "right")]
    Right,
    #[serde(rename = "enter")]
    Enter,
    #[serde(rename = "esc")]
    Esc,
    #[serde(rename = "tab")]
    Tab,
    #[serde(rename = "backspace")]
    Backspace,
    #[serde(rename = "ctrl_c")]
    CtrlC,
    #[serde(rename = "ctrl_d")]
    CtrlD,
    #[serde(rename = "ctrl_l")]
    CtrlL,
    #[serde(rename = "char")]
    Char(char),
}

impl Key {
    /// Convert key to raw bytes for PTY input
    pub fn to_bytes(&self) -> Vec<u8> {
        match self {
            Key::Up => b"\x1b[A".to_vec(),
            Key::Down => b"\x1b[B".to_vec(),
            Key::Left => b"\x1b[D".to_vec(),
            Key::Right => b"\x1b[C".to_vec(),
            Key::Enter => b"\r".to_vec(),
            Key::Esc => b"\x1b".to_vec(),
            Key::Tab => b"\t".to_vec(),
            Key::Backspace => b"\x7f".to_vec(),
            Key::CtrlC => b"\x03".to_vec(),
            Key::CtrlD => b"\x04".to_vec(),
            Key::CtrlL => b"\x0c".to_vec(),
            Key::Char(c) => c.to_string().into_bytes(),
        }
    }
}

impl std::fmt::Display for Key {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Key::Up => write!(f, "up"),
            Key::Down => write!(f, "down"),
            Key::Left => write!(f, "left"),
            Key::Right => write!(f, "right"),
            Key::Enter => write!(f, "enter"),
            Key::Esc => write!(f, "esc"),
            Key::Tab => write!(f, "tab"),
            Key::Backspace => write!(f, "backspace"),
            Key::CtrlC => write!(f, "ctrl_c"),
            Key::CtrlD => write!(f, "ctrl_d"),
            Key::CtrlL => write!(f, "ctrl_l"),
            Key::Char(c) => write!(f, "char_{}", c),
        }
    }
}

/// Interaction mode detected from PTY output
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InteractionMode {
    #[serde(rename = "normal")]
    Normal,
    #[serde(rename = "menu")]
    Menu,
    #[serde(rename = "editor")]
    Editor,
    #[serde(rename = "repl")]
    Repl,
}

impl std::fmt::Display for InteractionMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Normal => write!(f, "normal"),
            Self::Menu => write!(f, "menu"),
            Self::Editor => write!(f, "editor"),
            Self::Repl => write!(f, "repl"),
        }
    }
}

/// Screen content response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenContent {
    pub lines: Vec<String>,
    pub cursor_row: u16,
    pub cursor_col: u16,
    pub mode: InteractionMode,
}
