//! Background command execution for bypass sessions

use anyhow::{Context, Result};
use std::fs::OpenOptions;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// Execute a claude command in the background
///
/// This spawns the process and immediately returns the PID.
/// The process runs detached from the terminal (setsid).
/// Note: Child processes become zombies until the daemon exits;
/// this is acceptable for long-running daemons where init will reap them.
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
        .stderr(Stdio::from(output_file))
        .process_group(0);  // Create new process group

    // Spawn the process
    let mut child = cmd
        .spawn()
        .context("Failed to spawn claude command")?;

    let pid = child.id();

    // Spawn a background thread to reap the zombie when the process exits
    // This prevents zombie accumulation during daemon lifetime
    let _ = std::thread::spawn(move || {
        // Wait for the child process to exit and reap it
        // This prevents zombie accumulation
        let _ = child.wait();
    });

    tracing::info!("Started bypass session '{}' with PID {}", session_name, pid);

    Ok(pid)
}

/// Check if a process is still running
///
/// Returns true if the process exists, false otherwise.
/// Handles EPERM as "running" since the process exists but we lack permission.
pub fn is_process_running(pid: u32) -> bool {
    use nix::errno::Errno;

    match nix::sys::signal::kill(nix::unistd::Pid::from_raw(pid as i32), None) {
        Ok(()) => true,  // Process exists and we can signal it
        Err(Errno::ESRCH) => false,  // No such process
        Err(Errno::EPERM) => true,  // Process exists but we don't have permission
        Err(_) => false,  // Other errors, assume not running
    }
}
