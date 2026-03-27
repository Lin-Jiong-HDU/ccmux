//! Background command execution for bypass sessions

use anyhow::{Context, Result};
use std::fs::OpenOptions;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// Execute a claude command in the background
pub fn execute_bypass_command(
    session_name: &str,
    prompt: &str,
    cwd: &Path,
    output_path: &Path,
) -> Result<u32> {
    // Find claude executable
    let claude = which::which("claude").unwrap_or_else(|_| PathBuf::from("claude"));

    // Ensure output directory exists
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create output directory: {:?}", parent))?;
    }

    // Open output file for appending
    let output_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(output_path)
        .context("Failed to open output file")?;

    // Build the command
    let mut cmd = Command::new(&claude);
    cmd.arg("--dangerously-skip-permissions")
        .arg(prompt)
        .current_dir(cwd)
        .stdout(Stdio::from(output_file.try_clone()?))
        .stderr(Stdio::from(output_file));

    // Spawn the process
    let child = cmd
        .spawn()
        .context("Failed to spawn claude command")?;

    let pid = child.id();
    tracing::info!("Started bypass session '{}' with PID {}", session_name, pid);

    // Don't wait - fire and forget
    // The child process will continue running
    drop(child);

    Ok(pid)
}

/// Check if a process is still running
pub fn is_process_running(pid: u32) -> bool {
    nix::sys::signal::kill(nix::unistd::Pid::from_raw(pid as i32), None).is_ok()
}
