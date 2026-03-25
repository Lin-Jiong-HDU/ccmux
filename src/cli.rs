//! CLI argument parsing

use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "ccmux")]
#[command(about = "Claude Code session manager", long_about = None)]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Create a new session
    New {
        /// Session name
        #[arg(short, long)]
        name: String,
        /// Working directory
        #[arg(short, long)]
        cwd: Option<String>,
        /// Strategy (auto-safe, auto-all, manual)
        #[arg(short, long)]
        strategy: Option<String>,
    },

    /// List all sessions
    #[command(name = "ls")]
    List {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Kill a session
    Kill {
        /// Session name or ID
        session: String,
    },

    /// Attach to a session (interactive)
    Attach {
        /// Session name or ID
        session: String,
    },

    /// Send input to a session
    Send {
        /// Session name or ID
        session: String,
        /// Text to send
        text: String,
    },

    /// View session output
    Logs {
        /// Session name or ID
        session: String,
        /// Follow output
        #[arg(short, long)]
        follow: bool,
        /// Number of lines
        #[arg(short, long, default_value = "50")]
        tail: usize,
    },

    /// Get session status
    Status {
        /// Session name or ID (optional)
        session: Option<String>,
        /// Output as JSON
        #[arg(long)]
        json: bool,
        /// Watch mode
        #[arg(long)]
        watch: bool,
    },

    /// Start the daemon
    Start,

    /// Stop the daemon
    Stop,

    /// Wait for a pattern in session output
    Wait {
        /// Session name
        session: String,
        /// Pattern to wait for (regex)
        pattern: String,
        /// Timeout in seconds
        #[arg(short, long, default_value = "60")]
        timeout: u64,
    },

    /// Subscribe to session output stream
    Subscribe {
        /// Session name
        session: String,
        /// Only get output after this Unix epoch timestamp (milliseconds)
        #[arg(long, value_name = "MILLIS")]
        since: Option<u64>,
        /// Follow mode (poll continuously)
        #[arg(short, long)]
        follow: bool,
    },
}
