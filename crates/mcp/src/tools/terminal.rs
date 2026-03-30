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

pub struct NewTerminalTool;

#[async_trait]
impl Tool for NewTerminalTool {
    fn info(&self) -> ToolInfo {
        ToolInfo {
            name: "new_terminal".into(),
            description: "Spawn a new terminal pane in the editor.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "cwd": { "type": "string", "description": "Working directory for the terminal" },
                    "shell": { "type": "string", "description": "Shell executable" }
                }
            }),
        }
    }
    async fn call(&self, args: Value, bridge: Arc<RwLock<McpBridge>>) -> ToolCallResult {
        let cwd = args.get("cwd").and_then(|v| v.as_str()).map(|s| s.to_string());
        let shell = args.get("shell").and_then(|v| v.as_str()).map(|s| s.to_string());
        bridge.read().send_command(McpEditorCommand::NewTerminal { cwd, shell });
        ok_text("New terminal pane requested")
    }
}

pub struct SendInputTool;

#[async_trait]
impl Tool for SendInputTool {
    fn info(&self) -> ToolInfo {
        ToolInfo {
            name: "send_input".into(),
            description: "Send text input to a terminal pane.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "pane_id": { "type": "integer", "description": "Pane ID" },
                    "text": { "type": "string", "description": "Text to send" }
                },
                "required": ["pane_id", "text"]
            }),
        }
    }
    async fn call(&self, args: Value, bridge: Arc<RwLock<McpBridge>>) -> ToolCallResult {
        let pane_id = match args.get("pane_id").and_then(|v| v.as_u64()) {
            Some(n) => n as u32,
            None => return err_text("Missing argument: pane_id"),
        };
        let text = match args.get("text").and_then(|v| v.as_str()) {
            Some(t) => t.to_string(),
            None => return err_text("Missing argument: text"),
        };
        bridge.read().send_command(McpEditorCommand::SendTerminalInput { pane_id, text: text.clone() });
        ok_text(format!("Sent to pane {pane_id}: {text}"))
    }
}

pub struct GetOutputTool;

#[async_trait]
impl Tool for GetOutputTool {
    fn info(&self) -> ToolInfo {
        ToolInfo {
            name: "get_output".into(),
            description: "Get the last N lines of output from a terminal pane.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "pane_id": { "type": "integer", "description": "Pane ID (0 for active terminal)" },
                    "lines": { "type": "integer", "description": "Number of lines to return (default 50)" }
                }
            }),
        }
    }
    async fn call(&self, args: Value, bridge: Arc<RwLock<McpBridge>>) -> ToolCallResult {
        let pane_id = args.get("pane_id").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
        let lines = args.get("lines").and_then(|v| v.as_u64()).unwrap_or(50) as usize;
        let b = bridge.read();
        let output = b.terminal_output.get(&pane_id)
            .cloned()
            .unwrap_or_default();
        let start = output.len().saturating_sub(lines);
        ok_text(output[start..].join("\n"))
    }
}
