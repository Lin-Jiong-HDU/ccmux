//! ccmux CLI

use clap::Parser;
use ccmux::cli::{Cli, Command};
use ccmux::client::Client;
use anyhow::Result;

fn main() -> Result<()> {
    let cli = Cli::parse();
    let client = Client::new()?;

    match cli.command {
        Command::New { name, cwd, strategy } => {
            let result = client.new_session(name, cwd, strategy)?;
            println!("Created session: {}", result.id);
            println!("  Status: {}", result.status);
            println!("  Strategy: {}", result.strategy);
        }

        Command::List { json } => {
            let sessions = client.list_sessions()?;
            if json {
                println!("{}", serde_json::to_string_pretty(&sessions)?);
            } else if sessions.is_empty() {
                println!("No sessions.");
            } else {
                print_sessions_table(&sessions);
            }
        }

        Command::Kill { session } => {
            client.kill_session(session)?;
            println!("Session killed.");
        }

        Command::Attach { session } => {
            println!("Attaching to session: {}", session);
            println!("(Attach not yet implemented - requires interactive terminal handling)");
        }

        Command::Send { session, text } => {
            client.send_input(session, text)?;
            println!("Sent.");
        }

        Command::Logs { session, follow, tail } => {
            println!("Logs for session: {} (follow: {}, tail: {})", session, follow, tail);
            let output = client.get_output(session, Some(tail))?;
            for line in output {
                println!("  {}", line);
            }
        }

        Command::Status { session, json, watch } => {
            if watch {
                println!("Watch mode not yet implemented");
            }
            let status = client.get_status(session)?;
            if json {
                println!("{}", serde_json::to_string_pretty(&status)?);
            } else {
                print_status_detail(&status);
            }
        }

        Command::Start => {
            println!("Starting daemon...");
            println!("Run 'ccmuxd' in a separate terminal to start the daemon.");
        }

        Command::Stop => {
            println!("Stopping daemon...");
            // TODO: Implement daemon stop via socket
            println!("(Stop not yet implemented)");
        }
    }

    Ok(())
}

fn print_sessions_table(sessions: &[ccmux::protocol::SessionInfo]) {
    println!("{:<15} {:<10} {:<12} {:<10} {}",
        "SESSION", "STATUS", "STRATEGY", "UPTIME", "LAST OUTPUT");
    println!("{}", "-".repeat(80));

    for s in sessions {
        let status_symbol = match s.status {
            ccmux::protocol::SessionStatus::Running => "✓",
            ccmux::protocol::SessionStatus::Paused => "⏸",
            ccmux::protocol::SessionStatus::Stopped => "■",
        };
        let uptime = s.uptime_secs
            .map(|u| format!("{}h {}m", u / 3600, (u % 3600) / 60))
            .unwrap_or_else(|| "-".to_string());
        let last_output = s.last_output.as_deref().unwrap_or("-");

        println!("{:<15} {:<10} {:<12} {:<10} {}",
            s.id,
            format!("{} {}", status_symbol, s.status),
            s.strategy,
            uptime,
            last_output.chars().take(40).collect::<String>()
        );
    }
}

fn print_status_detail(status: &ccmux::protocol::SessionStatusDetail) {
    println!("Session:    {}", status.session);
    println!("Status:     {}", status.status);
    println!("Strategy:   {}", status.strategy);
    println!("Uptime:     {}", status.uptime);
    println!("Working Dir: {}", status.cwd);
    if let Some(pid) = status.pid {
        println!("PID:        {}", pid);
    }
    if !status.last_lines.is_empty() {
        println!("\nLast {} lines:", status.last_lines.len());
        for line in &status.last_lines {
            println!("  {}", line);
        }
    }
}
