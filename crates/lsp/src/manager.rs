#![allow(dead_code, unused_imports, unused_variables)]
//! LSP manager — owns one client per language, routes file events.
use std::path::{Path, PathBuf};
use std::sync::Arc;

use dashmap::DashMap;
use tokio::sync::mpsc;
use tracing::{info, warn};

use crate::client::{LspClient, ServerNotification};
use crate::diagnostics::DiagnosticsStore;
use crate::types::*;

// ─── Language detection ──────────────────────────────────────────────────────

/// Map file extension to LSP language identifier.
pub fn language_id_for_ext(ext: &str) -> Option<&'static str> {
    match ext {
        "rs" => Some("rust"),
        "py" => Some("python"),
        "js" | "mjs" => Some("javascript"),
        "ts" | "mts" => Some("typescript"),
        "tsx" => Some("typescriptreact"),
        "jsx" => Some("javascriptreact"),
        "go" => Some("go"),
        "c" | "h" => Some("c"),
        "cpp" | "cc" | "cxx" | "hpp" => Some("cpp"),
        "json" => Some("json"),
        "toml" => Some("toml"),
        "yaml" | "yml" => Some("yaml"),
        "md" => Some("markdown"),
        "sh" | "bash" => Some("shellscript"),
        "lua" => Some("lua"),
        _ => None,
    }
}

/// Server binary candidates in preference order.
pub fn servers_for_language(lang: &str) -> Vec<ServerSpec> {
    match lang {
        "rust" => vec![ServerSpec {
            binary: "rust-analyzer",
            args: vec![],
        }],
        "python" => vec![
            ServerSpec {
                binary: "pyright-langserver",
                args: vec!["--stdio"],
            },
            ServerSpec {
                binary: "pylsp",
                args: vec![],
            },
        ],
        "javascript" | "typescript" | "typescriptreact" | "javascriptreact" => vec![ServerSpec {
            binary: "typescript-language-server",
            args: vec!["--stdio"],
        }],
        "go" => vec![ServerSpec {
            binary: "gopls",
            args: vec![],
        }],
        "c" | "cpp" => vec![ServerSpec {
            binary: "clangd",
            args: vec![],
        }],
        "lua" => vec![ServerSpec {
            binary: "lua-language-server",
            args: vec![],
        }],
        _ => vec![],
    }
}

#[derive(Debug, Clone)]
pub struct ServerSpec {
    pub binary: &'static str,
    pub args: Vec<&'static str>,
}

/// Check if a binary exists on PATH.
fn which(binary: &str) -> Option<PathBuf> {
    let path_var = std::env::var("PATH").unwrap_or_default();
    let extensions: &[&str] = if cfg!(windows) {
        &[".exe", ".cmd", ".bat"]
    } else {
        &[""]
    };
    for dir in std::env::split_paths(&path_var) {
        for ext in extensions {
            let candidate = dir.join(format!("{binary}{ext}"));
            if candidate.exists() {
                return Some(candidate);
            }
        }
    }
    None
}

// ─── DiagnosticEvent ─────────────────────────────────────────────────────────

/// A diagnostic update event routed to the application.
#[derive(Debug, Clone)]
pub struct DiagnosticEvent {
    pub uri: String,
    pub version: Option<i64>,
    pub diagnostics: Vec<Diagnostic>,
}

// ─── LspManager ──────────────────────────────────────────────────────────────

/// Manages multiple LSP server instances (one per language).
pub struct LspManager {
    /// language_id -> LspClient
    clients: Arc<DashMap<String, LspClient>>,
    pub diagnostics: Arc<DiagnosticsStore>,
    diag_tx: mpsc::Sender<DiagnosticEvent>,
    root_path: PathBuf,
}

impl LspManager {
    pub fn new(root_path: PathBuf) -> (Arc<Self>, mpsc::Receiver<DiagnosticEvent>) {
        let (diag_tx, diag_rx) = mpsc::channel(256);
        let mgr = Arc::new(Self {
            clients: Arc::new(DashMap::new()),
            diagnostics: Arc::new(DiagnosticsStore::new()),
            diag_tx,
            root_path,
        });
        (mgr, diag_rx)
    }

    /// Ensure a language server is running for `lang_id`. Spawns asynchronously.
    /// Returns `true` if a server is already running or one was found on PATH.
    pub fn ensure_server_for_language(&self, lang_id: &str) -> bool {
        if self.clients.contains_key(lang_id) {
            return true;
        }

        let specs = servers_for_language(lang_id);
        for spec in specs {
            if which(spec.binary).is_some() {
                let lang = lang_id.to_string();
                let binary = spec.binary.to_string();
                let args: Vec<String> = spec.args.iter().map(|s| s.to_string()).collect();
                let root = self.root_path.clone();
                let clients = self.clients.clone();
                let diag_store = self.diagnostics.clone();
                let diag_tx = self.diag_tx.clone();

                tokio::spawn(async move {
                    let (notif_tx, mut notif_rx) = mpsc::channel::<ServerNotification>(128);
                    let args_ref: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
                    match LspClient::spawn(lang.clone(), &binary, &args_ref, &root, notif_tx)
                        .await
                    {
                        Ok(client) => {
                            info!("LSP {lang}: server started");
                            clients.insert(lang.clone(), client);
                            // Process notifications
                            while let Some(notif) = notif_rx.recv().await {
                                match notif {
                                    ServerNotification::PublishDiagnostics {
                                        uri,
                                        version,
                                        diagnostics,
                                    } => {
                                        diag_store.update(&uri, version, diagnostics.clone());
                                        let _ = diag_tx
                                            .send(DiagnosticEvent {
                                                uri,
                                                version,
                                                diagnostics,
                                            })
                                            .await;
                                    }
                                    ServerNotification::ShowMessage { level, message } => {
                                        info!("LSP [{lang}] msg({level}): {message}");
                                    }
                                    _ => {}
                                }
                            }
                        }
                        Err(e) => warn!("LSP {lang}: failed to spawn {binary}: {e}"),
                    }
                });
                return true;
            }
        }
        false
    }

    /// Get the client for a language if it's already running.
    pub fn client_for(&self, lang_id: &str) -> Option<LspClient> {
        self.clients.get(lang_id).map(|c| c.clone())
    }

    // ── File lifecycle ───────────────────────────────────────────────────────

    pub async fn did_open(&self, path: &Path, text: &str, version: i64) {
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if let Some(lang) = language_id_for_ext(ext) {
            self.ensure_server_for_language(lang);
            if let Some(client) = self.client_for(lang) {
                let uri = path_to_uri(path);
                let _ = client.did_open(&uri, lang, version, text).await;
            }
        }
    }

    pub async fn did_change(&self, path: &Path, text: &str, version: i64) {
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if let Some(lang) = language_id_for_ext(ext) {
            if let Some(client) = self.client_for(lang) {
                let uri = path_to_uri(path);
                let _ = client.did_change(&uri, version, text).await;
            }
        }
    }

    pub async fn did_save(&self, path: &Path, text: &str) {
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if let Some(lang) = language_id_for_ext(ext) {
            if let Some(client) = self.client_for(lang) {
                let uri = path_to_uri(path);
                let _ = client.did_save(&uri, Some(text)).await;
            }
        }
    }

    // ── LSP feature requests ─────────────────────────────────────────────────

    pub async fn completions(&self, path: &Path, line: u32, col: u32) -> Vec<CompletionItem> {
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if let Some(lang) = language_id_for_ext(ext) {
            if let Some(client) = self.client_for(lang) {
                let uri = path_to_uri(path);
                return client.completion(&uri, line, col).await.unwrap_or_default();
            }
        }
        vec![]
    }

    pub async fn hover(&self, path: &Path, line: u32, col: u32) -> Option<String> {
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if let Some(lang) = language_id_for_ext(ext) {
            if let Some(client) = self.client_for(lang) {
                let uri = path_to_uri(path);
                return client.hover(&uri, line, col).await.unwrap_or(None);
            }
        }
        None
    }

    pub async fn definition(&self, path: &Path, line: u32, col: u32) -> Vec<Location> {
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if let Some(lang) = language_id_for_ext(ext) {
            if let Some(client) = self.client_for(lang) {
                let uri = path_to_uri(path);
                return client
                    .definition(&uri, line, col)
                    .await
                    .unwrap_or_default();
            }
        }
        vec![]
    }

    pub async fn references(&self, path: &Path, line: u32, col: u32) -> Vec<Location> {
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if let Some(lang) = language_id_for_ext(ext) {
            if let Some(client) = self.client_for(lang) {
                let uri = path_to_uri(path);
                return client
                    .references(&uri, line, col, false)
                    .await
                    .unwrap_or_default();
            }
        }
        vec![]
    }

    pub async fn format(&self, path: &Path) -> Vec<TextEdit> {
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if let Some(lang) = language_id_for_ext(ext) {
            if let Some(client) = self.client_for(lang) {
                let uri = path_to_uri(path);
                return client.formatting(&uri, 4, true).await.unwrap_or_default();
            }
        }
        vec![]
    }

    pub async fn rename(&self, path: &Path, line: u32, col: u32, new_name: &str) -> Option<WorkspaceEdit> {
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if let Some(lang) = language_id_for_ext(ext) {
            if let Some(client) = self.client_for(lang) {
                let uri = path_to_uri(path);
                return client.rename(&uri, line, col, new_name).await.ok().flatten();
            }
        }
        None
    }

    pub async fn code_actions(&self, path: &Path, line: u32, col: u32) -> Vec<CodeAction> {
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if let Some(lang) = language_id_for_ext(ext) {
            if let Some(client) = self.client_for(lang) {
                let uri = path_to_uri(path);
                return client.code_actions(&uri, line, col).await.unwrap_or_default();
            }
        }
        vec![]
    }

    pub fn diagnostics_near_cursor(&self, path: &Path, line: u32) -> Vec<Diagnostic> {
        let uri = path_to_uri(path);
        let all = self.diagnostics.get(&uri).map(|f| f.items).unwrap_or_default();
        // Return diagnostics on the current line or adjacent lines
        all.into_iter().filter(|d| {
            let dline = d.range.start.line;
            dline == line || dline == line.saturating_sub(1) || dline == line + 1
        }).collect()
    }

    pub fn all_diagnostics_sorted(&self, path: &Path) -> Vec<Diagnostic> {
        let uri = path_to_uri(path);
        let mut all = self.diagnostics.get(&uri).map(|f| f.items).unwrap_or_default();
        all.sort_by_key(|d| (d.range.start.line, d.range.start.character));
        all
    }

    pub fn get_diagnostics(&self, path: &Path) -> Vec<Diagnostic> {
        let uri = path_to_uri(path);
        self.diagnostics
            .get(&uri)
            .map(|f| f.items)
            .unwrap_or_default()
    }
}
