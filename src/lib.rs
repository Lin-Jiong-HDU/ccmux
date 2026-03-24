//! # ccmux
//!
//! Claude Code session manager - like tmux, but for Claude Code.
//!
//! ## Overview
//!
//! ccmux manages multiple Claude Code sessions, allowing you to:
//! - Run Claude Code in the background
//! - Switch between different projects/contexts
//! - Control auto/pause behavior with strategies
//!
//! ## Architecture
//!
//! The system consists of:
//! - **ccmuxd**: Background daemon that manages sessions
//! - **ccmux**: CLI client for controlling the daemon
//!
//! Communication happens via Unix socket using JSON protocol.
//!
//! ## Example
//!
//! ```no_run
//! use ccmux::Client;
//!
//! let client = Client::new()?;
//! let sessions = client.list_sessions()?;
//! # Ok::<(), anyhow::Error>(())
//! ```

pub mod cli;
pub mod client;
pub mod config;
pub mod protocol;
pub mod server;
pub mod state;

pub use cli::{Cli, Command};
pub use client::Client;
pub use config::{ActionMode, Config, StrategyConfig};
pub use protocol::{Request, Response, SessionInfo, SessionStatus};
pub use state::{SessionState, State};
