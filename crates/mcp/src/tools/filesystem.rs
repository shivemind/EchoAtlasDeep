#![allow(dead_code, unused_imports, unused_variables)]
use std::path::{Path, PathBuf};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::sync::Arc;
use parking_lot::RwLock;

use super::super::protocol::{ToolInfo, ToolCallResult, ContentBlock};
use super::super::bridge::McpBridge;
use super::Tool;

fn sandboxed(workspace_root: &Path, path: &str) -> Option<PathBuf> {
    // If absolute, check it's inside workspace. If relative, join with workspace.
    let p = if Path::new(path).is_absolute() {
        PathBuf::from(path)
    } else {
        workspace_root.join(path)
    };
    // Canonicalize to resolve .. etc — but don't fail if file doesn't exist yet
    let canonical = p.canonicalize().unwrap_or(p);
    let workspace_canonical = workspace_root.canonicalize().unwrap_or(workspace_root.to_path_buf());
    if canonical.starts_with(&workspace_canonical) {
        Some(canonical)
    } else {
        None
    }
}

fn ok_text(text: impl Into<String>) -> ToolCallResult {
    ToolCallResult { content: vec![ContentBlock::text(text)], is_error: false }
}
fn err_text(text: impl Into<String>) -> ToolCallResult {
    ToolCallResult { content: vec![ContentBlock::error(text)], is_error: true }
}

// ─── read_file ────────────────────────────────────────────────────────────────

pub struct ReadFileTool { pub workspace_root: PathBuf }

#[async_trait]
impl Tool for ReadFileTool {
    fn info(&self) -> ToolInfo {
        ToolInfo {
            name: "read_file".into(),
            description: "Read the contents of a file. Path must be within the workspace.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "File path (relative to workspace or absolute)" }
                },
                "required": ["path"]
            }),
        }
    }

    async fn call(&self, args: Value, _bridge: Arc<RwLock<McpBridge>>) -> ToolCallResult {
        let path_str = match args.get("path").and_then(|v| v.as_str()) {
            Some(p) => p,
            None => return err_text("Missing required argument: path"),
        };
        let full_path = match sandboxed(&self.workspace_root, path_str) {
            Some(p) => p,
            None => return err_text("Path is outside the workspace sandbox"),
        };
        match std::fs::read_to_string(&full_path) {
            Ok(content) => ok_text(content),
            Err(e) => err_text(format!("Failed to read file: {e}")),
        }
    }
}

// ─── write_file ───────────────────────────────────────────────────────────────

pub struct WriteFileTool { pub workspace_root: PathBuf }

#[async_trait]
impl Tool for WriteFileTool {
    fn info(&self) -> ToolInfo {
        ToolInfo {
            name: "write_file".into(),
            description: "Write content to a file. Path must be within the workspace. Creates parent directories as needed.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Destination file path" },
                    "content": { "type": "string", "description": "File content to write" }
                },
                "required": ["path", "content"]
            }),
        }
    }

    async fn call(&self, args: Value, _bridge: Arc<RwLock<McpBridge>>) -> ToolCallResult {
        let path_str = match args.get("path").and_then(|v| v.as_str()) {
            Some(p) => p,
            None => return err_text("Missing argument: path"),
        };
        let content = match args.get("content").and_then(|v| v.as_str()) {
            Some(c) => c,
            None => return err_text("Missing argument: content"),
        };
        let full_path = match sandboxed(&self.workspace_root, path_str) {
            Some(p) => p,
            None => return err_text("Path is outside the workspace sandbox"),
        };
        if let Some(parent) = full_path.parent() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                return err_text(format!("Failed to create directories: {e}"));
            }
        }
        match std::fs::write(&full_path, content) {
            Ok(_) => ok_text(format!("Written {} bytes to {}", content.len(), full_path.display())),
            Err(e) => err_text(format!("Failed to write file: {e}")),
        }
    }
}

// ─── list_directory ───────────────────────────────────────────────────────────

pub struct ListDirectoryTool { pub workspace_root: PathBuf }

#[async_trait]
impl Tool for ListDirectoryTool {
    fn info(&self) -> ToolInfo {
        ToolInfo {
            name: "list_directory".into(),
            description: "List the contents of a directory with file metadata.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Directory path (default: workspace root)" }
                }
            }),
        }
    }

    async fn call(&self, args: Value, _bridge: Arc<RwLock<McpBridge>>) -> ToolCallResult {
        let path_str = args.get("path").and_then(|v| v.as_str()).unwrap_or(".");
        let full_path = match sandboxed(&self.workspace_root, path_str) {
            Some(p) => p,
            None => return err_text("Path is outside the workspace sandbox"),
        };
        let entries = match std::fs::read_dir(&full_path) {
            Ok(e) => e,
            Err(e) => return err_text(format!("Cannot read directory: {e}")),
        };
        let mut lines: Vec<String> = Vec::new();
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            let meta = entry.metadata();
            let kind = if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) { "dir " } else { "file" };
            let size = meta.as_ref().map(|m| m.len()).unwrap_or(0);
            lines.push(format!("{kind}  {:>10}  {name}", size));
        }
        lines.sort();
        ok_text(lines.join("\n"))
    }
}

// ─── search_files ─────────────────────────────────────────────────────────────

pub struct SearchFilesTool { pub workspace_root: PathBuf }

#[async_trait]
impl Tool for SearchFilesTool {
    fn info(&self) -> ToolInfo {
        ToolInfo {
            name: "search_files".into(),
            description: "Search for files by name pattern (glob) or search file contents by regex.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "pattern": { "type": "string", "description": "Pattern to search for (regex for content, glob-style for filenames)" },
                    "root": { "type": "string", "description": "Search root (default: workspace root)" },
                    "search_content": { "type": "boolean", "description": "If true, search file contents; otherwise search filenames" }
                },
                "required": ["pattern"]
            }),
        }
    }

    async fn call(&self, args: Value, _bridge: Arc<RwLock<McpBridge>>) -> ToolCallResult {
        let pattern = match args.get("pattern").and_then(|v| v.as_str()) {
            Some(p) => p,
            None => return err_text("Missing argument: pattern"),
        };
        let root_str = args.get("root").and_then(|v| v.as_str()).unwrap_or(".");
        let search_content = args.get("search_content").and_then(|v| v.as_bool()).unwrap_or(false);

        let root = match sandboxed(&self.workspace_root, root_str) {
            Some(p) => p,
            None => return err_text("Root is outside the workspace sandbox"),
        };

        let mut results: Vec<String> = Vec::new();

        if search_content {
            // Search file contents with regex
            let re = match regex::Regex::new(pattern) {
                Ok(r) => r,
                Err(e) => return err_text(format!("Invalid regex: {e}")),
            };
            for entry in walkdir::WalkDir::new(&root).max_depth(10).into_iter().flatten() {
                if !entry.file_type().is_file() { continue; }
                if let Ok(content) = std::fs::read_to_string(entry.path()) {
                    for (line_no, line) in content.lines().enumerate() {
                        if re.is_match(line) {
                            let rel = entry.path().strip_prefix(&self.workspace_root).unwrap_or(entry.path());
                            results.push(format!("{}:{}:{}", rel.display(), line_no + 1, line.trim()));
                            if results.len() >= 100 { break; }
                        }
                    }
                }
                if results.len() >= 100 { break; }
            }
        } else {
            // Search filenames
            let re = match regex::Regex::new(pattern) {
                Ok(r) => r,
                Err(e) => return err_text(format!("Invalid regex: {e}")),
            };
            for entry in walkdir::WalkDir::new(&root).max_depth(10).into_iter().flatten() {
                if entry.file_type().is_file() {
                    let name = entry.file_name().to_string_lossy();
                    if re.is_match(&name) {
                        let rel = entry.path().strip_prefix(&self.workspace_root).unwrap_or(entry.path());
                        results.push(rel.display().to_string());
                        if results.len() >= 100 { break; }
                    }
                }
            }
        }

        if results.is_empty() {
            ok_text("No matches found")
        } else {
            ok_text(results.join("\n"))
        }
    }
}

// ─── create_directory ─────────────────────────────────────────────────────────

pub struct CreateDirectoryTool { pub workspace_root: PathBuf }

#[async_trait]
impl Tool for CreateDirectoryTool {
    fn info(&self) -> ToolInfo {
        ToolInfo {
            name: "create_directory".into(),
            description: "Create a directory (and any missing parents).".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Directory path to create" }
                },
                "required": ["path"]
            }),
        }
    }
    async fn call(&self, args: Value, _bridge: Arc<RwLock<McpBridge>>) -> ToolCallResult {
        let path_str = match args.get("path").and_then(|v| v.as_str()) {
            Some(p) => p,
            None => return err_text("Missing argument: path"),
        };
        let full_path = match sandboxed(&self.workspace_root, path_str) {
            Some(p) => p,
            None => return err_text("Path is outside the workspace sandbox"),
        };
        match std::fs::create_dir_all(&full_path) {
            Ok(_) => ok_text(format!("Created directory: {}", full_path.display())),
            Err(e) => err_text(format!("Failed to create directory: {e}")),
        }
    }
}

// ─── delete_file ──────────────────────────────────────────────────────────────

pub struct DeleteFileTool { pub workspace_root: PathBuf }

#[async_trait]
impl Tool for DeleteFileTool {
    fn info(&self) -> ToolInfo {
        ToolInfo {
            name: "delete_file".into(),
            description: "Delete a file (not a directory). Path must be within workspace.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "File path to delete" }
                },
                "required": ["path"]
            }),
        }
    }
    async fn call(&self, args: Value, _bridge: Arc<RwLock<McpBridge>>) -> ToolCallResult {
        let path_str = match args.get("path").and_then(|v| v.as_str()) {
            Some(p) => p,
            None => return err_text("Missing argument: path"),
        };
        let full_path = match sandboxed(&self.workspace_root, path_str) {
            Some(p) => p,
            None => return err_text("Path is outside the workspace sandbox"),
        };
        match std::fs::remove_file(&full_path) {
            Ok(_) => ok_text(format!("Deleted: {}", full_path.display())),
            Err(e) => err_text(format!("Failed to delete: {e}")),
        }
    }
}

// ─── move_file ────────────────────────────────────────────────────────────────

pub struct MoveFileTool { pub workspace_root: PathBuf }

#[async_trait]
impl Tool for MoveFileTool {
    fn info(&self) -> ToolInfo {
        ToolInfo {
            name: "move_file".into(),
            description: "Move or rename a file within the workspace.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "source": { "type": "string", "description": "Source file path" },
                    "destination": { "type": "string", "description": "Destination path" }
                },
                "required": ["source", "destination"]
            }),
        }
    }
    async fn call(&self, args: Value, _bridge: Arc<RwLock<McpBridge>>) -> ToolCallResult {
        let src_str = match args.get("source").and_then(|v| v.as_str()) {
            Some(p) => p,
            None => return err_text("Missing argument: source"),
        };
        let dst_str = match args.get("destination").and_then(|v| v.as_str()) {
            Some(p) => p,
            None => return err_text("Missing argument: destination"),
        };
        let src = match sandboxed(&self.workspace_root, src_str) {
            Some(p) => p,
            None => return err_text("Source is outside the workspace sandbox"),
        };
        let dst = match sandboxed(&self.workspace_root, dst_str) {
            Some(p) => p,
            None => return err_text("Destination is outside the workspace sandbox"),
        };
        match std::fs::rename(&src, &dst) {
            Ok(_) => ok_text(format!("Moved {} -> {}", src.display(), dst.display())),
            Err(e) => err_text(format!("Failed to move: {e}")),
        }
    }
}
