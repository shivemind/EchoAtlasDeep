#![allow(dead_code, unused_imports, unused_variables)]
use std::sync::Arc;
use async_trait::async_trait;
use serde_json::{json, Value};
use parking_lot::RwLock;

use super::super::protocol::{ToolInfo, ToolCallResult, ContentBlock};
use super::super::bridge::{McpBridge, McpEditorCommand};
use super::Tool;

fn ok_text(text: impl Into<String>) -> ToolCallResult {
    ToolCallResult { content: vec![ContentBlock::text(text)], is_error: false }
}
fn err_text(text: impl Into<String>) -> ToolCallResult {
    ToolCallResult { content: vec![ContentBlock::error(text)], is_error: true }
}

// ─── open_file ────────────────────────────────────────────────────────────────

pub struct OpenFileTool;

#[async_trait]
impl Tool for OpenFileTool {
    fn info(&self) -> ToolInfo {
        ToolInfo {
            name: "open_file".into(),
            description: "Open a file in the editor pane, optionally scrolling to a specific line.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "File path to open" },
                    "line": { "type": "integer", "description": "Line number to scroll to (1-based)" }
                },
                "required": ["path"]
            }),
        }
    }
    async fn call(&self, args: Value, bridge: Arc<RwLock<McpBridge>>) -> ToolCallResult {
        let path = match args.get("path").and_then(|v| v.as_str()) {
            Some(p) => p.to_string(),
            None => return err_text("Missing argument: path"),
        };
        let line = args.get("line").and_then(|v| v.as_u64()).map(|n| n as usize);
        bridge.read().send_command(McpEditorCommand::OpenFile { path: path.clone(), line });
        ok_text(format!("Requested to open: {path}"))
    }
}

// ─── get_diagnostics ─────────────────────────────────────────────────────────

pub struct GetDiagnosticsTool;

#[async_trait]
impl Tool for GetDiagnosticsTool {
    fn info(&self) -> ToolInfo {
        ToolInfo {
            name: "get_diagnostics".into(),
            description: "Get LSP diagnostics for a file or the entire workspace.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Optional file path to scope diagnostics" }
                }
            }),
        }
    }
    async fn call(&self, args: Value, bridge: Arc<RwLock<McpBridge>>) -> ToolCallResult {
        let path_filter = args.get("path").and_then(|v| v.as_str()).map(|s| s.to_string());
        let b = bridge.read();
        let diags = &b.diagnostics;
        let filtered: Vec<_> = if let Some(ref f) = path_filter {
            diags.iter().filter(|d| d.file.contains(f.as_str())).collect()
        } else {
            diags.iter().collect()
        };
        if filtered.is_empty() {
            return ok_text("No diagnostics");
        }
        let text = filtered.iter().map(|d| {
            format!("{}:{}:{}: [{}] {}", d.file, d.line, d.col, d.severity, d.message)
        }).collect::<Vec<_>>().join("\n");
        ok_text(text)
    }
}

// ─── get_selection ────────────────────────────────────────────────────────────

pub struct GetSelectionTool;

#[async_trait]
impl Tool for GetSelectionTool {
    fn info(&self) -> ToolInfo {
        ToolInfo {
            name: "get_selection".into(),
            description: "Get the current visual selection text and its range in the editor.".into(),
            input_schema: json!({ "type": "object", "properties": {} }),
        }
    }
    async fn call(&self, _args: Value, bridge: Arc<RwLock<McpBridge>>) -> ToolCallResult {
        let b = bridge.read();
        match &b.selection {
            Some(sel) if !sel.text.is_empty() => {
                let info = format!("Selection [{},{}] to [{},{}]:\n{}",
                    sel.start_line, sel.start_col,
                    sel.end_line, sel.end_col,
                    sel.text);
                ok_text(info)
            }
            _ => ok_text("No active selection"),
        }
    }
}

// ─── apply_edit ───────────────────────────────────────────────────────────────

pub struct ApplyEditTool;

#[async_trait]
impl Tool for ApplyEditTool {
    fn info(&self) -> ToolInfo {
        ToolInfo {
            name: "apply_edit".into(),
            description: "Apply a text edit to a file in the editor.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "File path" },
                    "start_line": { "type": "integer", "description": "Start line (0-based)" },
                    "start_col": { "type": "integer", "description": "Start column (0-based)" },
                    "end_line": { "type": "integer", "description": "End line (0-based)" },
                    "end_col": { "type": "integer", "description": "End column (0-based)" },
                    "new_text": { "type": "string", "description": "Replacement text" }
                },
                "required": ["path", "start_line", "start_col", "end_line", "end_col", "new_text"]
            }),
        }
    }
    async fn call(&self, args: Value, bridge: Arc<RwLock<McpBridge>>) -> ToolCallResult {
        let get_usize = |key: &str| args.get(key).and_then(|v| v.as_u64()).map(|n| n as usize);
        let path = match args.get("path").and_then(|v| v.as_str()) {
            Some(p) => p.to_string(),
            None => return err_text("Missing argument: path"),
        };
        let (sl, sc, el, ec) = match (get_usize("start_line"), get_usize("start_col"), get_usize("end_line"), get_usize("end_col")) {
            (Some(sl), Some(sc), Some(el), Some(ec)) => (sl, sc, el, ec),
            _ => return err_text("Missing range arguments"),
        };
        let new_text = match args.get("new_text").and_then(|v| v.as_str()) {
            Some(t) => t.to_string(),
            None => return err_text("Missing argument: new_text"),
        };
        bridge.read().send_command(McpEditorCommand::ApplyEdit {
            path: path.clone(), start_line: sl, start_col: sc, end_line: el, end_col: ec, new_text,
        });
        ok_text(format!("Edit queued for {path}"))
    }
}

// ─── run_command ──────────────────────────────────────────────────────────────

pub struct RunCommandTool;

#[async_trait]
impl Tool for RunCommandTool {
    fn info(&self) -> ToolInfo {
        ToolInfo {
            name: "run_command".into(),
            description: "Execute an editor command (e.g. ':w', ':bd', ':vsplit').".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "command": { "type": "string", "description": "Editor command string" }
                },
                "required": ["command"]
            }),
        }
    }
    async fn call(&self, args: Value, bridge: Arc<RwLock<McpBridge>>) -> ToolCallResult {
        let command = match args.get("command").and_then(|v| v.as_str()) {
            Some(c) => c.to_string(),
            None => return err_text("Missing argument: command"),
        };
        bridge.read().send_command(McpEditorCommand::RunCommand { command: command.clone() });
        ok_text(format!("Command queued: {command}"))
    }
}
