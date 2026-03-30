#![allow(dead_code, unused_imports, unused_variables)]
pub mod filesystem;
pub mod editor;
pub mod terminal;

use std::path::PathBuf;
use std::sync::Arc;
use parking_lot::RwLock;
use async_trait::async_trait;
use serde_json::Value;

use super::protocol::{ToolInfo, ToolCallResult, ContentBlock};
use super::bridge::McpBridge;

#[async_trait]
pub trait Tool: Send + Sync {
    fn info(&self) -> ToolInfo;
    async fn call(&self, args: Value, bridge: Arc<RwLock<McpBridge>>) -> ToolCallResult;
}

pub type BoxedTool = Box<dyn Tool>;

/// All registered tools.
pub fn all_tools(workspace_root: PathBuf) -> Vec<BoxedTool> {
    use filesystem::*;
    use editor::*;
    use terminal::*;
    vec![
        Box::new(ReadFileTool { workspace_root: workspace_root.clone() }),
        Box::new(WriteFileTool { workspace_root: workspace_root.clone() }),
        Box::new(ListDirectoryTool { workspace_root: workspace_root.clone() }),
        Box::new(SearchFilesTool { workspace_root: workspace_root.clone() }),
        Box::new(CreateDirectoryTool { workspace_root: workspace_root.clone() }),
        Box::new(DeleteFileTool { workspace_root: workspace_root.clone() }),
        Box::new(MoveFileTool { workspace_root: workspace_root.clone() }),
        Box::new(OpenFileTool),
        Box::new(GetDiagnosticsTool),
        Box::new(GetSelectionTool),
        Box::new(ApplyEditTool),
        Box::new(RunCommandTool),
        Box::new(NewTerminalTool),
        Box::new(SendInputTool),
        Box::new(GetOutputTool),
    ]
}
