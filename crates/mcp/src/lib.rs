#![allow(dead_code, unused_imports, unused_variables)]
pub mod protocol;
pub mod transport;
pub mod bridge;
pub mod resources;
pub mod prompts;
pub mod tools;
pub mod server;

use std::path::PathBuf;
use std::sync::Arc;
use parking_lot::RwLock;
use tokio::sync::mpsc;

pub use bridge::{McpBridge, McpEditorCommand, McpDiagnostic, McpSelection, McpBuffer};
pub use server::{spawn_tcp_server, spawn_stdio_server};

/// Create the MCP bridge + channel, returning the bridge (shared) and command receiver.
pub fn create_bridge(
    workspace_root: PathBuf,
) -> (Arc<RwLock<McpBridge>>, mpsc::UnboundedReceiver<McpEditorCommand>) {
    let (tx, rx) = mpsc::unbounded_channel();
    let bridge = Arc::new(RwLock::new(McpBridge::new(workspace_root, tx)));
    (bridge, rx)
}

/// Launch MCP server — TCP if port is Some, stdio if port is None.
pub fn launch(
    bind_addr: &str,
    port: Option<u16>,
    workspace_root: PathBuf,
    bridge: Arc<RwLock<McpBridge>>,
) {
    if let Some(p) = port {
        spawn_tcp_server(bind_addr.to_string(), p, workspace_root, bridge);
    } else {
        // Default TCP port if nothing configured: 7878
        spawn_tcp_server(bind_addr.to_string(), 7878, workspace_root, bridge);
    }
}
