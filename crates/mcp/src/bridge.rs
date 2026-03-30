#![allow(dead_code, unused_imports, unused_variables)]
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::sync::mpsc;

/// Diagnostic entry exposed to MCP tools.
#[derive(Debug, Clone)]
pub struct McpDiagnostic {
    pub file: String,
    pub line: usize,
    pub col: usize,
    pub severity: String,
    pub message: String,
}

/// Current editor selection.
#[derive(Debug, Clone)]
pub struct McpSelection {
    pub text: String,
    pub start_line: usize,
    pub start_col: usize,
    pub end_line: usize,
    pub end_col: usize,
}

/// Open buffer info.
#[derive(Debug, Clone)]
pub struct McpBuffer {
    pub path: String,
    pub content: String,
    pub is_dirty: bool,
}

/// Commands the MCP server can send to the main event loop.
#[derive(Debug, Clone)]
pub enum McpEditorCommand {
    OpenFile { path: String, line: Option<usize> },
    ApplyEdit {
        path: String,
        start_line: usize,
        start_col: usize,
        end_line: usize,
        end_col: usize,
        new_text: String,
    },
    RunCommand { command: String },
    NewTerminal { cwd: Option<String>, shell: Option<String> },
    SendTerminalInput { pane_id: u32, text: String },
}

/// Shared read-only snapshot of app state for MCP tools.
/// Updated by the main loop each frame.
pub struct McpBridge {
    pub workspace_root: PathBuf,
    pub git_branch: String,
    pub git_status_summary: String,
    pub diagnostics: Vec<McpDiagnostic>,
    pub selection: Option<McpSelection>,
    pub open_buffers: Vec<McpBuffer>,
    pub active_file: Option<String>,
    pub terminal_output: HashMap<u32, Vec<String>>,
    cmd_tx: mpsc::UnboundedSender<McpEditorCommand>,
}

impl McpBridge {
    pub fn new(
        workspace_root: PathBuf,
        cmd_tx: mpsc::UnboundedSender<McpEditorCommand>,
    ) -> Self {
        Self {
            workspace_root,
            git_branch: String::new(),
            git_status_summary: String::new(),
            diagnostics: Vec::new(),
            selection: None,
            open_buffers: Vec::new(),
            active_file: None,
            terminal_output: HashMap::new(),
            cmd_tx,
        }
    }

    pub fn send_command(&self, cmd: McpEditorCommand) {
        let _ = self.cmd_tx.send(cmd);
    }

    /// Build a workspace file tree listing.
    pub fn file_tree_text(&self) -> String {
        let mut lines = Vec::new();
        if let Ok(entries) = std::fs::read_dir(&self.workspace_root) {
            for entry in entries.flatten() {
                let kind = if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) { "/" } else { "" };
                let name = entry.file_name().to_string_lossy().into_owned();
                lines.push(format!("{name}{kind}"));
            }
        }
        lines.sort();
        lines.join("\n")
    }
}
