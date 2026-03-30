#![allow(dead_code, unused_imports, unused_variables)]
use std::path::PathBuf;
use std::sync::Arc;
use parking_lot::RwLock;
use tokio::io::{AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tracing::{info, warn, debug};

use super::protocol::*;
use super::transport::{read_message, write_response, write_notification};
use super::bridge::McpBridge;
use super::resources::{list_resources, read_resource};
use super::prompts::{list_prompts, get_prompt};
use super::tools::{all_tools, BoxedTool};

/// Spawn an MCP server listening on the given address.
pub fn spawn_tcp_server(
    bind_addr: String,
    port: u16,
    workspace_root: PathBuf,
    bridge: Arc<RwLock<McpBridge>>,
) {
    let addr = format!("{bind_addr}:{port}");
    let tools: Arc<Vec<BoxedTool>> = Arc::new(all_tools(workspace_root));
    tokio::spawn(async move {
        match TcpListener::bind(&addr).await {
            Ok(listener) => {
                info!("MCP server listening on {addr}");
                loop {
                    match listener.accept().await {
                        Ok((stream, peer)) => {
                            info!("MCP client connected: {peer}");
                            let bridge_clone = bridge.clone();
                            let tools_clone = tools.clone();
                            tokio::spawn(async move {
                                if let Err(e) = handle_connection(stream, bridge_clone, tools_clone).await {
                                    warn!("MCP connection error: {e}");
                                }
                            });
                        }
                        Err(e) => warn!("MCP accept error: {e}"),
                    }
                }
            }
            Err(e) => warn!("MCP server bind error on {addr}: {e}"),
        }
    });
}

/// Spawn an MCP server on stdio (for use with Claude Desktop / MCP clients that pipe stdio).
pub fn spawn_stdio_server(
    workspace_root: PathBuf,
    bridge: Arc<RwLock<McpBridge>>,
) {
    let tools: Arc<Vec<BoxedTool>> = Arc::new(all_tools(workspace_root));
    tokio::spawn(async move {
        let stdin = tokio::io::stdin();
        let stdout = tokio::io::stdout();
        let mut reader = BufReader::new(stdin);
        let mut writer = tokio::io::BufWriter::new(stdout);
        let mut initialized = false;

        loop {
            let req = match read_message(&mut reader).await {
                Ok(r) => r,
                Err(e) => {
                    debug!("MCP stdio read error: {e}");
                    break;
                }
            };
            let resp = dispatch_request(req, &bridge, &tools, &mut initialized).await;
            if let Err(e) = write_response(&mut writer, &resp).await {
                warn!("MCP stdio write error: {e}");
                break;
            }
        }
    });
}

async fn handle_connection(
    stream: TcpStream,
    bridge: Arc<RwLock<McpBridge>>,
    tools: Arc<Vec<BoxedTool>>,
) -> anyhow::Result<()> {
    let (read_half, write_half) = stream.into_split();
    let mut reader = BufReader::new(read_half);
    let mut writer = tokio::io::BufWriter::new(write_half);
    let mut initialized = false;

    loop {
        let req = match read_message(&mut reader).await {
            Ok(r) => r,
            Err(e) => {
                debug!("MCP TCP read done: {e}");
                break;
            }
        };
        let resp = dispatch_request(req, &bridge, &tools, &mut initialized).await;
        write_response(&mut writer, &resp).await?;
    }
    Ok(())
}

async fn dispatch_request(
    req: RpcRequest,
    bridge: &Arc<RwLock<McpBridge>>,
    tools: &Arc<Vec<BoxedTool>>,
    initialized: &mut bool,
) -> RpcResponse {
    let id = req.id.clone();

    match req.method.as_str() {
        "initialize" => {
            *initialized = true;
            let result = InitializeResult {
                protocol_version: "2024-11-05".into(),
                capabilities: ServerCapabilities {
                    tools: Some(ToolsCapability { list_changed: false }),
                    resources: Some(ResourcesCapability { subscribe: false, list_changed: false }),
                    prompts: Some(PromptsCapability { list_changed: false }),
                },
                server_info: ServerInfo {
                    name: "rmtide".into(),
                    version: env!("CARGO_PKG_VERSION").into(),
                },
            };
            RpcResponse::ok(id, serde_json::to_value(result).unwrap())
        }

        "notifications/initialized" => {
            // Notification — no response needed but we handle gracefully
            RpcResponse::ok(id, serde_json::Value::Null)
        }

        "tools/list" => {
            let tool_list: Vec<ToolInfo> = tools.iter().map(|t| t.info()).collect();
            let result = serde_json::json!({ "tools": tool_list });
            RpcResponse::ok(id, result)
        }

        "tools/call" => {
            let params: ToolCallParams = match serde_json::from_value(req.params.clone()) {
                Ok(p) => p,
                Err(e) => return RpcResponse::err(id, ERR_INVALID_PARAMS, e.to_string()),
            };
            let tool = tools.iter().find(|t| t.info().name == params.name);
            match tool {
                Some(t) => {
                    let args = params.arguments.unwrap_or(serde_json::Value::Object(Default::default()));
                    let result = t.call(args, bridge.clone()).await;
                    RpcResponse::ok(id, serde_json::to_value(result).unwrap_or_default())
                }
                None => RpcResponse::err(id, ERR_TOOL_ERROR, format!("Tool not found: {}", params.name)),
            }
        }

        "resources/list" => {
            let resources = list_resources();
            let result = serde_json::json!({ "resources": resources });
            RpcResponse::ok(id, result)
        }

        "resources/read" => {
            let uri = req.params.get("uri").and_then(|v| v.as_str()).unwrap_or("");
            let b = bridge.read();
            match read_resource(uri, &b) {
                Some(contents) => {
                    let result = serde_json::json!({ "contents": [contents] });
                    RpcResponse::ok(id, result)
                }
                None => RpcResponse::err(id, ERR_RESOURCE_NOT_FOUND, format!("Resource not found: {uri}")),
            }
        }

        "prompts/list" => {
            let prompts = list_prompts();
            let result = serde_json::json!({ "prompts": prompts });
            RpcResponse::ok(id, result)
        }

        "prompts/get" => {
            let params: GetPromptParams = match serde_json::from_value(req.params.clone()) {
                Ok(p) => p,
                Err(e) => return RpcResponse::err(id, ERR_INVALID_PARAMS, e.to_string()),
            };
            match get_prompt(&params) {
                Some(result) => RpcResponse::ok(id, serde_json::to_value(result).unwrap_or_default()),
                None => RpcResponse::err(id, ERR_PROMPT_NOT_FOUND, format!("Prompt not found: {}", params.name)),
            }
        }

        "ping" => RpcResponse::ok(id, serde_json::json!({})),

        _ => RpcResponse::err(id, ERR_METHOD_NOT_FOUND, format!("Method not found: {}", req.method)),
    }
}
