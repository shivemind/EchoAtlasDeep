/// Unix PTY implementation using libc directly.
use anyhow::Result;
use std::ffi::CString;
use std::os::fd::FromRawFd;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::io::{ReadHalf, WriteHalf};
use tracing::info;

use super::{PtyConfig, PtyHandle, PtySize};

pub struct Pty {
    writer: Option<WriteHalf<tokio::fs::File>>,
    reader: Option<ReadHalf<tokio::fs::File>>,
    master_fd: i32,
    pid: libc::pid_t,
}

impl Pty {
    pub async fn spawn(config: PtyConfig) -> Result<Self> {
        let mut winsize = libc::winsize {
            ws_row: config.size.rows,
            ws_col: config.size.cols,
            ws_xpixel: config.size.pixel_width,
            ws_ypixel: config.size.pixel_height,
        };

        let mut master_fd: libc::c_int = -1;
        let mut slave_fd: libc::c_int = -1;

        let ret = unsafe {
            libc::openpty(
                &mut master_fd,
                &mut slave_fd,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                &mut winsize as *mut libc::winsize,
            )
        };
        if ret != 0 {
            return Err(anyhow::anyhow!(
                "openpty failed: {}",
                std::io::Error::last_os_error()
            ));
        }

        let shell_c = CString::new(config.shell.as_bytes())?;
        let mut argv: Vec<CString> = vec![shell_c.clone()];
        for arg in &config.args {
            argv.push(CString::new(arg.as_bytes())?);
        }
        let envp: Vec<CString> = config
            .env
            .iter()
            .map(|(k, v)| CString::new(format!("{k}={v}").as_bytes()))
            .collect::<std::result::Result<_, _>>()?;

        let working_dir = config.working_dir.clone();

        let pid = unsafe { libc::fork() };
        match pid {
            -1 => {
                return Err(anyhow::anyhow!(
                    "fork failed: {}",
                    std::io::Error::last_os_error()
                ));
            }
            0 => unsafe {
                // child process
                libc::setsid();
                libc::dup2(slave_fd, 0);
                libc::dup2(slave_fd, 1);
                libc::dup2(slave_fd, 2);
                for fd in 3..256 {
                    libc::close(fd);
                }
                if let Some(dir) = working_dir {
                    let _ = std::env::set_current_dir(&dir);
                }
                let mut argv_ptrs: Vec<*const libc::c_char> =
                    argv.iter().map(|s| s.as_ptr()).collect();
                argv_ptrs.push(std::ptr::null());
                let mut envp_ptrs: Vec<*const libc::c_char> =
                    envp.iter().map(|s| s.as_ptr()).collect();
                envp_ptrs.push(std::ptr::null());
                // execvpe searches PATH; not available on macOS libc — use execve there.
                #[cfg(target_os = "linux")]
                libc::execvpe(shell_c.as_ptr(), argv_ptrs.as_ptr(), envp_ptrs.as_ptr());
                #[cfg(not(target_os = "linux"))]
                libc::execve(shell_c.as_ptr(), argv_ptrs.as_ptr(), envp_ptrs.as_ptr());
                libc::_exit(1);
            },
            child_pid => {
                // parent process
                unsafe { libc::close(slave_fd) };
                info!("PTY spawned shell (pid={child_pid})");
                let master_file = unsafe { tokio::fs::File::from_raw_fd(master_fd) };
                let (reader, writer) = tokio::io::split(master_file);
                Ok(Self { reader: Some(reader), writer: Some(writer), master_fd, pid: child_pid })
            }
        }
    }

    pub async fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        Ok(self.reader.as_mut().ok_or_else(|| anyhow::anyhow!("reader taken"))?.read(buf).await?)
    }

    pub async fn write(&mut self, data: &[u8]) -> Result<()> {
        let w = self.writer.as_mut().ok_or_else(|| anyhow::anyhow!("writer taken"))?;
        w.write_all(data).await?;
        w.flush().await?;
        Ok(())
    }

    /// Take the reader half out of this Pty (for use in a reader task).
    pub fn take_reader(&mut self) -> Option<ReadHalf<tokio::fs::File>> {
        self.reader.take()
    }

    /// Take the writer half out of this Pty (for use in a writer task).
    pub fn take_writer(&mut self) -> Option<WriteHalf<tokio::fs::File>> {
        self.writer.take()
    }
}

impl PtyHandle for Pty {
    fn resize(&self, size: PtySize) -> Result<()> {
        let mut winsize = libc::winsize {
            ws_row: size.rows,
            ws_col: size.cols,
            ws_xpixel: size.pixel_width,
            ws_ypixel: size.pixel_height,
        };
        let ret = unsafe {
            libc::ioctl(
                self.master_fd,
                libc::TIOCSWINSZ,
                &mut winsize as *mut libc::winsize,
            )
        };
        if ret != 0 {
            return Err(anyhow::anyhow!(
                "TIOCSWINSZ failed: {}",
                std::io::Error::last_os_error()
            ));
        }
        Ok(())
    }

    fn process_id(&self) -> Option<u32> {
        Some(self.pid as u32)
    }
}
