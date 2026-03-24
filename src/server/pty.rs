//! PTY spawning and I/O handling

use anyhow::Result;
use nix::pty::{forkpty, Winsize};
use nix::unistd::Pid;
use std::ffi::CString;
use std::fs::File;
use std::io::{Read, Write};
use std::os::fd::{AsRawFd, RawFd};
use std::process::Command;

#[derive(Debug, Clone, Copy)]
pub struct PtySize {
    pub cols: u16,
    pub rows: u16,
}

impl From<Winsize> for PtySize {
    fn from(ws: Winsize) -> Self {
        Self {
            cols: ws.ws_col,
            rows: ws.ws_row,
        }
    }
}

impl From<PtySize> for Winsize {
    fn from(sz: PtySize) -> Self {
        Winsize {
            ws_row: sz.rows,
            ws_col: sz.cols,
            ws_xpixel: 0,
            ws_ypixel: 0,
        }
    }
}

/// PTY master handle
pub struct Pty {
    master: File,
    child_pid: Pid,
}

impl Pty {
    /// Spawn a new PTY with the given command
    pub fn spawn(cmd: Command) -> Result<Self> {
        let winsize = Some(Winsize {
            ws_row: 24,
            ws_col: 80,
            ws_xpixel: 0,
            ws_ypixel: 0,
        });

        let result = unsafe { forkpty(&winsize, None)? };

        match result {
            nix::pty::ForkptyResult::Parent { child, master } => {
                // Convert OwnedFd to File
                let master_file = File::from(master);
                Ok(Self {
                    master: master_file,
                    child_pid: child,
                })
            }
            nix::pty::ForkptyResult::Child => {
                // Child process - exec the command
                let program = CString::new(cmd.get_program().to_string_lossy().into_owned())?;
                let args: Vec<CString> = cmd
                    .get_args()
                    .map(|s| CString::new(s.to_string_lossy().into_owned()).unwrap())
                    .collect();
                nix::unistd::execvp(&program, &args)?;
                unreachable!("exec failed");
            }
        }
    }

    /// Write to the PTY master (sends to child stdin)
    pub fn write(&mut self, data: &[u8]) -> Result<usize> {
        let n = self.master.write(data)?;
        self.master.flush()?;
        Ok(n)
    }

    /// Read from the PTY master (reads from child stdout)
    pub fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        let n = self.master.read(buf)?;
        Ok(n)
    }

    /// Set PTY window size
    pub fn resize(&self, size: PtySize) -> Result<()> {
        let mut winsize = Winsize::from(size);
        nix::ioctl_readwrite_bad!(pty_set_winsize, nix::libc::TIOCSWINSZ, Winsize);
        unsafe { pty_set_winsize(self.master.as_raw_fd(), &mut winsize) }?;
        Ok(())
    }

    /// Get the raw file descriptor for async operations
    pub fn as_raw_fd(&self) -> RawFd {
        self.master.as_raw_fd()
    }

    /// Get the child PID
    pub fn child_pid(&self) -> Pid {
        self.child_pid
    }
}

impl Drop for Pty {
    fn drop(&mut self) {
        // Send SIGHUP to child process
        let _ = nix::sys::signal::kill(self.child_pid, nix::sys::signal::Signal::SIGHUP);
        // Wait for child to exit
        let _ = nix::sys::wait::waitpid(self.child_pid, None);
    }
}
