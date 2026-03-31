#![allow(dead_code, unused_imports, unused_variables)]
/// Windows ConPTY implementation.
use anyhow::{Context, Result};
use tokio::io::{AsyncReadExt, AsyncWriteExt, ReadHalf, WriteHalf};
use windows::Win32::Foundation::{HANDLE, CloseHandle};
use windows::Win32::System::Console::{
    CreatePseudoConsole, ClosePseudoConsole, ResizePseudoConsole,
    COORD, HPCON,
};
use windows::Win32::System::Threading::{
    CreateProcessW, PROCESS_INFORMATION, STARTUPINFOEXW,
    EXTENDED_STARTUPINFO_PRESENT, CREATE_UNICODE_ENVIRONMENT,
    InitializeProcThreadAttributeList, UpdateProcThreadAttribute,
    PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE, LPPROC_THREAD_ATTRIBUTE_LIST,
};
use tracing::info;

use super::{PtyConfig, PtyHandle, PtySize};

pub struct Pty {
    hpc: HPCON,
    process: HANDLE,
    reader: Option<ReadHalf<tokio::fs::File>>,
    writer: Option<WriteHalf<tokio::fs::File>>,
}

unsafe impl Send for Pty {}

impl Pty {
    pub async fn spawn(config: PtyConfig) -> Result<Self> {
        use std::os::windows::io::FromRawHandle;

        // Create pipe pairs for ConPTY I/O.
        let (pipe_read_in,  pipe_write_in)  = create_pipe()?;
        let (pipe_read_out, pipe_write_out) = create_pipe()?;

        let size = COORD {
            X: config.size.cols as i16,
            Y: config.size.rows as i16,
        };

        // Create the pseudo console — windows 0.56 returns Result<HPCON>.
        let hpc = unsafe {
            CreatePseudoConsole(size, pipe_read_in, pipe_write_out, 0)
                .context("CreatePseudoConsole failed")?
        };

        // Close pipe ends now owned by ConPTY.
        unsafe {
            CloseHandle(pipe_read_in).ok();
            CloseHandle(pipe_write_out).ok();
        }

        // Build STARTUPINFOEXW with PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE.
        let (si, _attr_buf) = build_startup_info(hpc)?;

        // Spawn the shell process.
        let mut cmd: Vec<u16> = config.shell.encode_utf16().collect();
        cmd.push(0);

        let mut pi = PROCESS_INFORMATION::default();
        unsafe {
            CreateProcessW(
                None,
                windows::core::PWSTR(cmd.as_mut_ptr()),
                None, None, false,
                EXTENDED_STARTUPINFO_PRESENT | CREATE_UNICODE_ENVIRONMENT,
                None, None,
                &si.StartupInfo as *const _ as *const _,
                &mut pi,
            ).context("CreateProcessW failed")?;
        }

        info!("ConPTY spawned shell (pid={})", pi.dwProcessId);

        // Wrap pipe handles in tokio async files.
        let read_file = unsafe {
            tokio::fs::File::from_raw_handle(pipe_read_out.0 as _)
        };
        let write_file = unsafe {
            tokio::fs::File::from_raw_handle(pipe_write_in.0 as _)
        };
        let (reader, _) = tokio::io::split(read_file);
        let (_, writer) = tokio::io::split(write_file);

        Ok(Self { hpc, process: pi.hProcess, reader: Some(reader), writer: Some(writer) })
    }

    pub async fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        Ok(self.reader.as_mut().ok_or_else(|| anyhow::anyhow!("reader taken"))?.read(buf).await?)
    }

    pub async fn write(&mut self, data: &[u8]) -> Result<()> {
        self.writer.as_mut().ok_or_else(|| anyhow::anyhow!("writer taken"))?.write_all(data).await?;
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
        let coord = COORD { X: size.cols as i16, Y: size.rows as i16 };
        unsafe {
            ResizePseudoConsole(self.hpc, coord)
                .context("ResizePseudoConsole failed")?;
        }
        Ok(())
    }

    fn process_id(&self) -> Option<u32> {
        None
    }
}

impl Drop for Pty {
    fn drop(&mut self) {
        unsafe {
            ClosePseudoConsole(self.hpc);
            CloseHandle(self.process).ok();
        }
    }
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn create_pipe() -> Result<(HANDLE, HANDLE)> {
    use windows::Win32::System::Pipes::CreatePipe;
    let mut read  = HANDLE::default();
    let mut write = HANDLE::default();
    unsafe {
        CreatePipe(&mut read, &mut write, None, 0)
            .context("CreatePipe failed")?;
    }
    Ok((read, write))
}

/// Build STARTUPINFOEXW with the ConPTY attribute attached.
/// Returns the struct AND the backing buffer (must stay alive while struct is used).
fn build_startup_info(hpc: HPCON) -> Result<(STARTUPINFOEXW, Vec<u8>)> {
    let mut si = STARTUPINFOEXW::default();
    si.StartupInfo.cb = std::mem::size_of::<STARTUPINFOEXW>() as u32;

    let mut attr_size: usize = 0;
    let buf = unsafe {
        // First call: get required size.
        let _ = InitializeProcThreadAttributeList(
            LPPROC_THREAD_ATTRIBUTE_LIST::default(), 1, 0, &mut attr_size,
        );
        let mut buf = vec![0u8; attr_size];
        let attr_list = LPPROC_THREAD_ATTRIBUTE_LIST(buf.as_mut_ptr() as _);

        InitializeProcThreadAttributeList(attr_list, 1, 0, &mut attr_size)
            .context("InitializeProcThreadAttributeList")?;

        UpdateProcThreadAttribute(
            attr_list,
            0,
            PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE as usize,
            Some(hpc.0 as *const _),
            std::mem::size_of::<HPCON>(),
            None, None,
        ).context("UpdateProcThreadAttribute")?;

        si.lpAttributeList = attr_list;
        buf
    };
    Ok((si, buf))
}
