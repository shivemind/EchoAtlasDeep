/// Windows ConPTY implementation.
use anyhow::{Context, Result};
use std::ptr;
use tokio::io::{AsyncReadExt, AsyncWriteExt, ReadHalf, WriteHalf};
use windows::Win32::Foundation::{HANDLE, CloseHandle};
use windows::Win32::System::Console::{
    CreatePseudoConsole, ClosePseudoConsole, ResizePseudoConsole,
    COORD, HPCON,
};
use windows::Win32::System::Threading::{
    CreateProcess, PROCESS_INFORMATION, STARTUPINFOEXW,
    EXTENDED_STARTUPINFO_PRESENT, CREATE_UNICODE_ENVIRONMENT,
    InitializeProcThreadAttributeList, UpdateProcThreadAttribute,
    PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE,
};
use tracing::info;

use super::{PtyConfig, PtyHandle, PtySize};

pub struct Pty {
    hpc: HPCON,
    process: HANDLE,
    reader: ReadHalf<tokio::fs::File>,
    writer: WriteHalf<tokio::fs::File>,
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

        // Create the pseudo console.
        let hpc = unsafe {
            let mut hpc = HPCON::default();
            CreatePseudoConsole(size, pipe_read_in, pipe_write_out, 0, &mut hpc)
                .context("CreatePseudoConsole failed")?;
            hpc
        };

        // Close pipe ends now owned by ConPTY.
        unsafe {
            CloseHandle(pipe_read_in).ok();
            CloseHandle(pipe_write_out).ok();
        }

        // Build STARTUPINFOEXW with PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE.
        let si = build_startup_info(hpc)?;

        // Spawn the shell process.
        let mut cmd: Vec<u16> = config.shell.encode_utf16().collect();
        cmd.push(0);

        let mut pi = PROCESS_INFORMATION::default();
        unsafe {
            CreateProcess(
                None,
                windows::core::PWSTR(cmd.as_mut_ptr()),
                None, None, false,
                EXTENDED_STARTUPINFO_PRESENT | CREATE_UNICODE_ENVIRONMENT,
                None, None,
                &si.StartupInfo as *const _ as *const _,
                &mut pi,
            ).context("CreateProcess failed")?;
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

        Ok(Self { hpc, process: pi.hProcess, reader, writer })
    }

    pub async fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        Ok(self.reader.read(buf).await?)
    }

    pub async fn write(&mut self, data: &[u8]) -> Result<()> {
        self.writer.write_all(data).await?;
        Ok(())
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
        None // not easily available post-spawn without storing dwProcessId
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
fn build_startup_info(hpc: HPCON) -> Result<STARTUPINFOEXW> {
    use windows::Win32::System::Threading::DeleteProcThreadAttributeList;
    let mut si = STARTUPINFOEXW::default();
    si.StartupInfo.cb = std::mem::size_of::<STARTUPINFOEXW>() as u32;

    let mut attr_size: usize = 0;
    unsafe {
        // First call to get required size.
        let _ = InitializeProcThreadAttributeList(None, 1, 0, &mut attr_size);
        let mut buf = vec![0u8; attr_size];
        let attr_list = buf.as_mut_ptr() as *mut _;
        InitializeProcThreadAttributeList(
            windows::Win32::System::Threading::LPPROC_THREAD_ATTRIBUTE_LIST(attr_list),
            1, 0, &mut attr_size,
        ).context("InitializeProcThreadAttributeList")?;

        UpdateProcThreadAttribute(
            windows::Win32::System::Threading::LPPROC_THREAD_ATTRIBUTE_LIST(attr_list),
            0,
            PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE as usize,
            Some(hpc.0 as *const _),
            std::mem::size_of::<HPCON>(),
            None, None,
        ).context("UpdateProcThreadAttribute")?;

        si.lpAttributeList =
            windows::Win32::System::Threading::LPPROC_THREAD_ATTRIBUTE_LIST(attr_list);
    }
    Ok(si)
}
