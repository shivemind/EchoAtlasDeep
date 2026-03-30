#![allow(dead_code, unused_imports, unused_variables)]
//! Breakpoint manager for the DAP crate.

use parking_lot::RwLock;

/// A single breakpoint.
#[derive(Debug, Clone)]
pub struct Breakpoint {
    pub file: String,
    pub line: usize,
    pub condition: Option<String>,
    pub enabled: bool,
}

/// Manages the set of breakpoints for the current debug session.
pub struct BreakpointManager {
    breakpoints: RwLock<Vec<Breakpoint>>,
}

impl BreakpointManager {
    pub fn new() -> Self {
        Self {
            breakpoints: RwLock::new(Vec::new()),
        }
    }

    /// Toggle a breakpoint at the given file/line. If one already exists it is
    /// removed; otherwise a new enabled breakpoint is added.
    pub fn toggle(&self, file: &str, line: usize) {
        let mut bps = self.breakpoints.write();
        if let Some(pos) = bps.iter().position(|b| b.file == file && b.line == line) {
            bps.remove(pos);
        } else {
            bps.push(Breakpoint {
                file: file.to_string(),
                line,
                condition: None,
                enabled: true,
            });
        }
    }

    /// Returns a cloned list of all breakpoints.
    pub fn list(&self) -> Vec<Breakpoint> {
        self.breakpoints.read().clone()
    }

    /// Returns cloned breakpoints for a specific file.
    pub fn for_file(&self, file: &str) -> Vec<Breakpoint> {
        self.breakpoints
            .read()
            .iter()
            .filter(|b| b.file == file)
            .cloned()
            .collect()
    }

    /// Remove all breakpoints.
    pub fn clear_all(&self) {
        self.breakpoints.write().clear();
    }

    /// Enable or disable a breakpoint at the given file/line.
    pub fn set_enabled(&self, file: &str, line: usize, enabled: bool) {
        let mut bps = self.breakpoints.write();
        if let Some(bp) = bps.iter_mut().find(|b| b.file == file && b.line == line) {
            bp.enabled = enabled;
        }
    }

    /// Set a condition on a breakpoint.
    pub fn set_condition(&self, file: &str, line: usize, condition: Option<String>) {
        let mut bps = self.breakpoints.write();
        if let Some(bp) = bps.iter_mut().find(|b| b.file == file && b.line == line) {
            bp.condition = condition;
        }
    }

    /// Returns true if there is an enabled breakpoint at the given location.
    pub fn has_breakpoint(&self, file: &str, line: usize) -> bool {
        self.breakpoints
            .read()
            .iter()
            .any(|b| b.file == file && b.line == line && b.enabled)
    }
}

impl Default for BreakpointManager {
    fn default() -> Self {
        Self::new()
    }
}
