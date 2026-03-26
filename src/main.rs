//! ccmux CLI

use anyhow::Result;
use ccmux::cli::{Cli, Command};
use ccmux::client::Client;
use ccmux::protocol::Key;
use clap::Parser;
use strip_ansi::strip_ansi as strip_ansi_escapes;

fn parse_key(s: &str) -> Result<Key> {
    match s.to_lowercase().as_str() {
        "up" => Ok(Key::Up),
        "down" => Ok(Key::Down),
        "left" => Ok(Key::Left),
        "right" => Ok(Key::Right),
        "enter" | "return" => Ok(Key::Enter),
        "esc" | "escape" => Ok(Key::Esc),
        "tab" => Ok(Key::Tab),
        "backspace" => Ok(Key::Backspace),
        "ctrl_c" | "ctrl-c" => Ok(Key::CtrlC),
        "ctrl_d" | "ctrl-d" => Ok(Key::CtrlD),
        "ctrl_l" | "ctrl-l" => Ok(Key::CtrlL),
        _ => anyhow::bail!("Unknown key: {}", s),
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let client = Client::new()?;

    match cli.command {
        Command::New {
            name,
            cwd,
            strategy,
        } => {
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

        Command::SendKey { session, key } => {
            let key = parse_key(&key)?;
            client.send_key(session, key)?;
            println!("Sent key: {}", key);
        }

        Command::Screen { session, json } => {
            let screen = client.get_screen(session)?;
            if json {
                println!("{}", serde_json::to_string_pretty(&screen)?);
            } else {
                println!("Mode: {}", screen.mode);
                println!("Cursor: {},{}", screen.cursor_row, screen.cursor_col);
                println!("--- Screen ---");
                for line in screen.lines {
                    println!("{}", line);
                }
            }
        }

        Command::Logs {
            session,
            follow,
            tail,
        } => {
            println!(
                "Logs for session: {} (follow: {}, tail: {})",
                session, follow, tail
            );
            let output = client.get_output(session, Some(tail))?;
            for line in output {
                println!("  {}", strip_ansi_escapes(&line));
            }
        }

        Command::Status {
            session,
            json,
            watch,
        } => {
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

        Command::Wait { session, pattern, timeout } => {
            println!("Waiting for pattern '{}' (timeout: {}s)...", pattern, timeout);

            let result = client.wait_with_poll(&session, &pattern, timeout * 1000)?;

            if result.matched {
                println!("✓ Matched!");
                if let Some(output) = &result.output {
                    println!("{}", strip_ansi_escapes(output));
                }
            } else {
                println!("✗ Timeout - pattern not found");
            }
        }

        Command::Subscribe {
            session,
            since,
            follow,
        } => {
            let mut last_ts = since.unwrap_or(0);

            loop {
                let events = client.subscribe(&session, Some(last_ts))?;

                for event in &events {
                    if let Some(ts) = event.ts {
                        last_ts = last_ts.max(ts);
                    }
                    if let Some(text) = &event.text {
                        println!("{}", strip_ansi_escapes(text));
                    }
                }

                if !follow {
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
        }
    }

    Ok(())
}

fn print_sessions_table(sessions: &[ccmux::protocol::SessionInfo]) {
    println!(
        "{:<15} {:<10} {:<12} {:<10} LAST OUTPUT",
        "SESSION", "STATUS", "STRATEGY", "UPTIME"
    );
    println!("{}", "-".repeat(80));

    for s in sessions {
        let status_symbol = match s.status {
            ccmux::protocol::SessionStatus::Running => "✓",
            ccmux::protocol::SessionStatus::Paused => "⏸",
            ccmux::protocol::SessionStatus::Stopped => "■",
        };
        let uptime = s
            .uptime_secs
            .map(|u| format!("{}h {}m", u / 3600, (u % 3600) / 60))
            .unwrap_or_else(|| "-".to_string());
        let last_output = s.last_output.as_deref().unwrap_or("-");

        println!(
            "{:<15} {:<10} {:<12} {:<10} {}",
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
            println!("  {}", strip_ansi_escapes(line));
        }
    }
}
