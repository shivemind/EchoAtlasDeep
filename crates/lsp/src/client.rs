#![allow(dead_code, unused_imports, unused_variables)]
//! Core LSP client — manages one language server process.
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use parking_lot::Mutex;
use serde_json::{json, Value};
use tokio::io::{BufReader, BufWriter};
use tokio::process::Command;
use tokio::sync::{mpsc, oneshot};
use tracing::{debug, error, info, warn};

use crate::jsonrpc::{read_message, write_message, RpcMessage};
use crate::types::*;

// ─── Internal types ──────────────────────────────────────────────────────────

struct Pending {
    tx: oneshot::Sender<anyhow::Result<Value>>,
}

struct Inner {
    write_tx: mpsc::Sender<RpcMessage>,
    pending: Mutex<HashMap<u64, Pending>>,
    next_id: AtomicU64,
}

// ─── Public notification type ────────────────────────────────────────────────

/// Notifications pushed from the language server to the editor.
#[derive(Debug, Clone)]
pub enum ServerNotification {
    PublishDiagnostics {
        uri: String,
        version: Option<i64>,
        diagnostics: Vec<Diagnostic>,
    },
    ShowMessage {
        level: u8,
        message: String,
    },
    LogMessage {
        level: u8,
        message: String,
    },
    ApplyEdit {
        label: Option<String>,
        edit: WorkspaceEdit,
    },
    Progress {
        token: String,
        value: Value,
    },
    Unknown {
        method: String,
        params: Value,
    },
}

// ─── LspClient ───────────────────────────────────────────────────────────────

/// Handle to a running language server. Cheaply cloneable (Arc-backed).
#[derive(Clone)]
pub struct LspClient {
    inner: Arc<Inner>,
    pub root_uri: String,
    pub server_name: String,
    pub capabilities: Arc<Mutex<Value>>,
}

impl LspClient {
    /// Spawn a language server process and perform the LSP initialize handshake.
    pub async fn spawn(
        server_name: impl Into<String>,
        command: &str,
        args: &[&str],
        root_path: &std::path::Path,
        notif_tx: mpsc::Sender<ServerNotification>,
    ) -> anyhow::Result<Self> {
        let server_name = server_name.into();
        let root_uri = path_to_uri(root_path);

        let mut child = Command::new(command)
            .args(args)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .map_err(|e| anyhow::anyhow!("Failed to spawn {server_name}: {e}"))?;

        let stdin = child.stdin.take().unwrap();
        let stdout = child.stdout.take().unwrap();

        let (write_tx, mut write_rx) = mpsc::channel::<RpcMessage>(256);

        let inner = Arc::new(Inner {
            write_tx: write_tx.clone(),
            pending: Mutex::new(HashMap::new()),
            next_id: AtomicU64::new(1),
        });
        let capabilities = Arc::new(Mutex::new(Value::Null));

        // Writer task — owns the child process (keep-alive via _child)
        tokio::spawn(async move {
            let _child = child;
            let mut writer = BufWriter::new(stdin);
            while let Some(msg) = write_rx.recv().await {
                if let Err(e) = write_message(&mut writer, &msg).await {
                    error!("LSP write error: {e}");
                    break;
                }
            }
        });

        // Reader task — dispatches responses to pending map and notifications to channel
        let inner2 = inner.clone();
        tokio::spawn(async move {
            let mut reader = BufReader::new(stdout);
            loop {
                match read_message(&mut reader).await {
                    Ok(Some(msg)) => dispatch_message(msg, &inner2, &notif_tx),
                    Ok(None) => {
                        debug!("LSP server EOF");
                        break;
                    }
                    Err(e) => {
                        error!("LSP read error: {e}");
                        break;
                    }
                }
            }
        });

        let client = Self {
            inner,
            root_uri: root_uri.clone(),
            server_name: server_name.clone(),
            capabilities,
        };

        // LSP initialize handshake
        let init_params = json!({
            "processId": std::process::id(),
            "rootUri": root_uri,
            "capabilities": build_client_capabilities(),
            "initializationOptions": {},
            "workspaceFolders": [{ "uri": root_uri, "name": "root" }],
        });

        let resp = client.request("initialize", init_params).await?;
        *client.capabilities.lock() = resp.get("capabilities").cloned().unwrap_or(Value::Null);

        client.notify("initialized", json!({})).await?;
        info!("LSP {server_name}: initialized");

        Ok(client)
    }

    // ── Core transport ───────────────────────────────────────────────────────

    /// Send a JSON-RPC request and await the response value.
    pub async fn request(&self, method: &str, params: Value) -> anyhow::Result<Value> {
        let id = self.inner.next_id.fetch_add(1, Ordering::Relaxed);
        let (tx, rx) = oneshot::channel();
        self.inner.pending.lock().insert(id, Pending { tx });
        let msg = RpcMessage::request(id, method, params);
        self.inner
            .write_tx
            .send(msg)
            .await
            .map_err(|_| anyhow::anyhow!("LSP write channel closed"))?;
        rx.await
            .map_err(|_| anyhow::anyhow!("LSP response channel dropped"))?
    }

    /// Send a JSON-RPC notification (no response expected).
    pub async fn notify(&self, method: &str, params: Value) -> anyhow::Result<()> {
        let msg = RpcMessage::notification(method, params);
        self.inner
            .write_tx
            .send(msg)
            .await
            .map_err(|_| anyhow::anyhow!("LSP write channel closed"))?;
        Ok(())
    }

    // ── High-level LSP methods ───────────────────────────────────────────────

    pub async fn did_open(
        &self,
        uri: &str,
        language_id: &str,
        version: i64,
        text: &str,
    ) -> anyhow::Result<()> {
        self.notify(
            "textDocument/didOpen",
            json!({
                "textDocument": {
                    "uri": uri,
                    "languageId": language_id,
                    "version": version,
                    "text": text,
                }
            }),
        )
        .await
    }

    pub async fn did_change(&self, uri: &str, version: i64, text: &str) -> anyhow::Result<()> {
        self.notify(
            "textDocument/didChange",
            json!({
                "textDocument": { "uri": uri, "version": version },
                "contentChanges": [{ "text": text }],
            }),
        )
        .await
    }

    pub async fn did_save(&self, uri: &str, text: Option<&str>) -> anyhow::Result<()> {
        let mut params = json!({ "textDocument": { "uri": uri } });
        if let Some(t) = text {
            params["text"] = json!(t);
        }
        self.notify("textDocument/didSave", params).await
    }

    pub async fn did_close(&self, uri: &str) -> anyhow::Result<()> {
        self.notify(
            "textDocument/didClose",
            json!({ "textDocument": { "uri": uri } }),
        )
        .await
    }

    pub async fn completion(
        &self,
        uri: &str,
        line: u32,
        character: u32,
    ) -> anyhow::Result<Vec<CompletionItem>> {
        let resp = self
            .request(
                "textDocument/completion",
                json!({
                    "textDocument": { "uri": uri },
                    "position": { "line": line, "character": character },
                    "context": { "triggerKind": 1 },
                }),
            )
            .await?;

        // Response can be CompletionList or CompletionItem[]
        if let Some(items) = resp.get("items") {
            Ok(serde_json::from_value(items.clone()).unwrap_or_default())
        } else if resp.is_array() {
            Ok(serde_json::from_value(resp).unwrap_or_default())
        } else {
            Ok(vec![])
        }
    }

    pub async fn hover(
        &self,
        uri: &str,
        line: u32,
        character: u32,
    ) -> anyhow::Result<Option<String>> {
        let resp = self
            .request(
                "textDocument/hover",
                json!({
                    "textDocument": { "uri": uri },
                    "position": { "line": line, "character": character },
                }),
            )
            .await?;

        if resp.is_null() {
            return Ok(None);
        }
        if let Some(contents) = resp.get("contents") {
            let text = match contents {
                Value::String(s) => s.clone(),
                Value::Object(o) => o
                    .get("value")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                Value::Array(arr) => arr
                    .iter()
                    .filter_map(|v| v.as_str().or_else(|| v.get("value")?.as_str()))
                    .collect::<Vec<_>>()
                    .join("\n"),
                _ => String::new(),
            };
            Ok(if text.is_empty() { None } else { Some(text) })
        } else {
            Ok(None)
        }
    }

    pub async fn definition(
        &self,
        uri: &str,
        line: u32,
        character: u32,
    ) -> anyhow::Result<Vec<Location>> {
        let resp = self
            .request(
                "textDocument/definition",
                json!({
                    "textDocument": { "uri": uri },
                    "position": { "line": line, "character": character },
                }),
            )
            .await?;
        parse_locations(resp)
    }

    pub async fn references(
        &self,
        uri: &str,
        line: u32,
        character: u32,
        include_declaration: bool,
    ) -> anyhow::Result<Vec<Location>> {
        let resp = self
            .request(
                "textDocument/references",
                json!({
                    "textDocument": { "uri": uri },
                    "position": { "line": line, "character": character },
                    "context": { "includeDeclaration": include_declaration },
                }),
            )
            .await?;
        parse_locations(resp)
    }

    pub async fn rename(
        &self,
        uri: &str,
        line: u32,
        character: u32,
        new_name: &str,
    ) -> anyhow::Result<Option<WorkspaceEdit>> {
        let resp = self
            .request(
                "textDocument/rename",
                json!({
                    "textDocument": { "uri": uri },
                    "position": { "line": line, "character": character },
                    "newName": new_name,
                }),
            )
            .await?;
        if resp.is_null() {
            return Ok(None);
        }
        Ok(serde_json::from_value(resp).ok())
    }

    pub async fn code_action(
        &self,
        uri: &str,
        range: Range,
        diagnostics: Vec<Diagnostic>,
    ) -> anyhow::Result<Vec<CodeAction>> {
        let resp = self
            .request(
                "textDocument/codeAction",
                json!({
                    "textDocument": { "uri": uri },
                    "range": range,
                    "context": {
                        "diagnostics": diagnostics,
                        "only": ["quickfix", "refactor", "source.fixAll"],
                    },
                }),
            )
            .await?;
        if resp.is_null() {
            return Ok(vec![]);
        }
        Ok(serde_json::from_value(resp).unwrap_or_default())
    }

    pub async fn formatting(
        &self,
        uri: &str,
        tab_size: u32,
        insert_spaces: bool,
    ) -> anyhow::Result<Vec<TextEdit>> {
        let resp = self
            .request(
                "textDocument/formatting",
                json!({
                    "textDocument": { "uri": uri },
                    "options": {
                        "tabSize": tab_size,
                        "insertSpaces": insert_spaces,
                        "trimTrailingWhitespace": true,
                        "insertFinalNewline": true,
                    },
                }),
            )
            .await?;
        if resp.is_null() {
            return Ok(vec![]);
        }
        Ok(serde_json::from_value(resp).unwrap_or_default())
    }

    pub async fn shutdown(&self) -> anyhow::Result<()> {
        let _ = self.request("shutdown", json!(null)).await;
        let _ = self.notify("exit", json!(null)).await;
        Ok(())
    }
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn dispatch_message(
    msg: RpcMessage,
    inner: &Arc<Inner>,
    notif_tx: &mpsc::Sender<ServerNotification>,
) {
    if msg.is_response() {
        if let Some(id_val) = &msg.id {
            if let Some(id) = id_val.as_u64() {
                let mut pending = inner.pending.lock();
                if let Some(p) = pending.remove(&id) {
                    let result = if let Some(err) = msg.error {
                        Err(anyhow::anyhow!(
                            "LSP error {}: {}",
                            err.code,
                            err.message
                        ))
                    } else {
                        Ok(msg.result.unwrap_or(Value::Null))
                    };
                    let _ = p.tx.send(result);
                }
            }
        }
    } else if msg.is_notification() {
        let notif = parse_notification(&msg);
        let _ = notif_tx.try_send(notif);
    }
}

fn parse_notification(msg: &RpcMessage) -> ServerNotification {
    let method = msg.method.as_deref().unwrap_or("");
    let params = msg.params.clone().unwrap_or(Value::Null);
    match method {
        "textDocument/publishDiagnostics" => {
            let uri = params["uri"].as_str().unwrap_or("").to_string();
            let version = params["version"].as_i64();
            let diagnostics =
                serde_json::from_value(params["diagnostics"].clone()).unwrap_or_default();
            ServerNotification::PublishDiagnostics {
                uri,
                version,
                diagnostics,
            }
        }
        "window/showMessage" => ServerNotification::ShowMessage {
            level: params["type"].as_u64().unwrap_or(4) as u8,
            message: params["message"].as_str().unwrap_or("").to_string(),
        },
        "window/logMessage" => ServerNotification::LogMessage {
            level: params["type"].as_u64().unwrap_or(4) as u8,
            message: params["message"].as_str().unwrap_or("").to_string(),
        },
        "workspace/applyEdit" => {
            let label = params["label"].as_str().map(str::to_string);
            let edit = serde_json::from_value(params["edit"].clone())
                .unwrap_or(WorkspaceEdit { changes: None });
            ServerNotification::ApplyEdit { label, edit }
        }
        "$/progress" => {
            let token = params["token"].as_str().unwrap_or("").to_string();
            ServerNotification::Progress {
                token,
                value: params["value"].clone(),
            }
        }
        _ => ServerNotification::Unknown {
            method: method.to_string(),
            params,
        },
    }
}

fn parse_locations(resp: Value) -> anyhow::Result<Vec<Location>> {
    if resp.is_null() {
        return Ok(vec![]);
    }
    if resp.is_array() {
        // Could be Location[] or LocationLink[] — try Location[] first
        if let Ok(locs) = serde_json::from_value::<Vec<Location>>(resp.clone()) {
            return Ok(locs);
        }
        // LocationLink array — extract targetUri + targetSelectionRange
        if let Some(arr) = resp.as_array() {
            let locs: Vec<Location> = arr
                .iter()
                .filter_map(|v| {
                    let uri = v["targetUri"].as_str()?.to_string();
                    let range: Range =
                        serde_json::from_value(v["targetSelectionRange"].clone()).ok()?;
                    Some(Location { uri, range })
                })
                .collect();
            return Ok(locs);
        }
    }
    Ok(vec![])
}

fn build_client_capabilities() -> Value {
    json!({
        "textDocument": {
            "synchronization": {
                "dynamicRegistration": false,
                "willSave": false,
                "willSaveWaitUntil": false,
                "didSave": true,
            },
            "completion": {
                "dynamicRegistration": false,
                "completionItem": {
                    "snippetSupport": true,
                    "documentationFormat": ["plaintext", "markdown"],
                    "insertReplaceSupport": true,
                },
                "contextSupport": true,
            },
            "hover": {
                "dynamicRegistration": false,
                "contentFormat": ["plaintext", "markdown"],
            },
            "definition": { "dynamicRegistration": false, "linkSupport": false },
            "references": { "dynamicRegistration": false },
            "rename": { "dynamicRegistration": false, "prepareSupport": false },
            "codeAction": {
                "dynamicRegistration": false,
                "resolveSupport": { "properties": [] },
            },
            "publishDiagnostics": {
                "relatedInformation": true,
                "versionSupport": true,
            },
            "formatting": { "dynamicRegistration": false },
        },
        "workspace": {
            "applyEdit": true,
            "workspaceFolders": true,
        },
    })
}
