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

pub mod state;
pub mod protocol;
pub mod config;
pub mod server;
pub mod client;
pub mod cli;

pub use protocol::{Request, Response, SessionStatus, SessionInfo};
pub use config::{Config, StrategyConfig, ActionMode};
pub use state::{State, SessionState};
pub use client::Client;
pub use cli::{Cli, Command};
