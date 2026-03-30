/// Platform-agnostic PTY abstraction.
/// Concrete implementations live in unix.rs / windows.rs.
use std::collections::HashMap;
use anyhow::Result;

#[derive(Debug, Clone, Copy)]
pub struct PtySize {
    pub rows: u16,
    pub cols: u16,
    pub pixel_width: u16,
    pub pixel_height: u16,
}

impl Default for PtySize {
    fn default() -> Self {
        Self { rows: 24, cols: 80, pixel_width: 0, pixel_height: 0 }
    }
}

// ─── Platform dispatch ───────────────────────────────────────────────────────

#[cfg(unix)]
mod unix;
#[cfg(windows)]
mod windows;

#[cfg(unix)]
pub use unix::Pty;
#[cfg(windows)]
pub use windows::Pty;

// ─── Shared config ───────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct PtyConfig {
    pub shell: String,
    pub args: Vec<String>,
    pub env: HashMap<String, String>,
    pub size: PtySize,
    pub working_dir: Option<std::path::PathBuf>,
}

impl Default for PtyConfig {
    fn default() -> Self {
        let shell = if cfg!(windows) {
            std::env::var("COMSPEC").unwrap_or_else(|_| "cmd.exe".into())
        } else {
            std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".into())
        };
        Self {
            shell,
            args: vec![],
            env: std::env::vars().collect(),
            size: PtySize::default(),
            working_dir: None,
        }
    }
}

/// Common trait implemented by both platform PTY types.
pub trait PtyHandle: Send + 'static {
    fn resize(&self, size: PtySize) -> Result<()>;
    fn process_id(&self) -> Option<u32>;
}
