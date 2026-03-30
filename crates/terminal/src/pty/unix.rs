/// Unix PTY implementation using nix + tokio.
use anyhow::{Context, Result};
use nix::pty::{openpty, Winsize};
use nix::unistd::{close, dup2, execvpe, fork, setsid, ForkResult};
use std::ffi::CString;
use std::os::fd::{FromRawFd, IntoRawFd, OwnedFd};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::io::{ReadHalf, WriteHalf};
use tracing::{debug, info, warn};

use super::{PtyConfig, PtyHandle, PtySize};

pub struct Pty {
    /// Async write half to the master PTY fd.
    writer: WriteHalf<tokio::fs::File>,
    /// Async read half to the master PTY fd.
    reader: ReadHalf<tokio::fs::File>,
    /// Master fd kept alive for resize.
    master_fd: i32,
    /// Child PID.
    pid: nix::unistd::Pid,
}

impl Pty {
    pub async fn spawn(config: PtyConfig) -> Result<Self> {
        let winsize = Winsize {
            ws_row: config.size.rows,
            ws_col: config.size.cols,
            ws_xpixel: config.size.pixel_width,
            ws_ypixel: config.size.pixel_height,
        };

        let pty_res = openpty(Some(&winsize), None)
            .context("openpty failed")?;

        let master_fd = pty_res.master.into_raw_fd();
        let slave_fd  = pty_res.slave.into_raw_fd();

        // Build argv / envp for exec.
        let shell_c = CString::new(config.shell.as_bytes())?;
        let mut argv: Vec<CString> = vec![shell_c.clone()];
        for arg in &config.args {
            argv.push(CString::new(arg.as_bytes())?);
        }
        let envp: Vec<CString> = config.env
            .iter()
            .map(|(k, v)| CString::new(format!("{k}={v}").as_bytes()))
            .collect::<std::result::Result<_, _>>()?;

        let working_dir = config.working_dir.clone();

        match unsafe { fork()? } {
            ForkResult::Child => {
                // ── child process ─────────────────────────────────────
                // Detach from controlling terminal.
                setsid().ok();

                // Connect slave PTY to stdio.
                dup2(slave_fd, 0).ok(); // stdin
                dup2(slave_fd, 1).ok(); // stdout
                dup2(slave_fd, 2).ok(); // stderr

                // Close all other fds.
                for fd in 3..256 {
                    unsafe { libc::close(fd) };
                }

                if let Some(dir) = working_dir {
                    let _ = std::env::set_current_dir(&dir);
                }

                execvpe(&shell_c, &argv, &envp)
                    .expect("execvpe failed");
                unreachable!()
            }

            ForkResult::Parent { child } => {
                // ── parent process ────────────────────────────────────
                unsafe { libc::close(slave_fd) };

                info!("PTY spawned shell (pid={child})");

                // Wrap master fd in tokio async file.
                let master_file = unsafe {
                    tokio::fs::File::from_raw_fd(master_fd)
                };
                let (reader, writer) = tokio::io::split(master_file);

                Ok(Self { reader, writer, master_fd, pid: child })
            }
        }
    }

    pub async fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        let n = self.reader.read(buf).await?;
        Ok(n)
    }

    pub async fn write(&mut self, data: &[u8]) -> Result<()> {
        self.writer.write_all(data).await?;
        self.writer.flush().await?;
        Ok(())
    }
}

impl PtyHandle for Pty {
    fn resize(&self, size: PtySize) -> Result<()> {
        let winsize = Winsize {
            ws_row: size.rows,
            ws_col: size.cols,
            ws_xpixel: size.pixel_width,
            ws_ypixel: size.pixel_height,
        };
        unsafe {
            let ret = libc::ioctl(
                self.master_fd,
                libc::TIOCSWINSZ,
                &winsize as *const Winsize,
            );
            if ret != 0 {
                return Err(anyhow::anyhow!("TIOCSWINSZ failed: {}", std::io::Error::last_os_error()));
            }
        }
        Ok(())
    }

    fn process_id(&self) -> Option<u32> {
        Some(self.pid.as_raw() as u32)
    }
}
