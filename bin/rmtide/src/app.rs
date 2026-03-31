#![allow(dead_code, unused_imports, unused_variables)]
//! Central application state owned by the main task.
use std::sync::Arc;

use rmcore::ids::{BufferId, IdGen};
use editor::cursor::MultiCursor;
use editor::fold::FoldState;
use editor::modal::{EditorCommand, ModalState, SearchDir, VisualKind};
use editor::registers::Registers;
use editor::registry::BufferRegistry;
use editor::search::SearchState;
use editor::undo::UndoTree;
use editor::view::{CursorPos, EditorView};

use lsp::manager::{DiagnosticEvent, LspManager};
use lsp::types::{CodeAction, CompletionItem, Location};

use ai::{BackendRegistry, EditorContext, GhostText, KeyVault, SpendTracker, ApprovalQueue, ApprovalModalState, ResponseCache, FallbackChain};
use ai::{AgentMemory, PromptLibrary};
use ai::agent::AgentUpdate;
use git::GitManager;
use git;
use ui;
use plugin::PluginRegistry;
use dap;
use ui::theme::Theme;

use crate::config::Config;
use ui::render::EditorDisplay;
use ui::widgets::model_picker::ModelEntry;
use runner;

pub struct ActiveEditor {
    pub buffer_id: BufferId,
    pub view: EditorView,
    pub modal: ModalState,
    pub undo: UndoTree,
    pub registers: Registers,
    pub search: SearchState,
    pub folds: FoldState,
    pub cursors: MultiCursor,
}

impl ActiveEditor {
    pub fn new(buffer_id: BufferId) -> Self {
        Self {
            buffer_id,
            view: EditorView::new(buffer_id),
            modal: ModalState::new(),
            undo: UndoTree::new(),
            registers: Registers::new(),
            search: SearchState::new(),
            folds: FoldState::new(),
            cursors: MultiCursor::new(),
        }
    }
}

pub struct AppState {
    pub config: Arc<Config>,
    pub buffers: BufferRegistry,
    pub ids: IdGen,
    pub active_editor: Option<ActiveEditor>,
    pub file_picker_open: bool,
    pub workspace_root: std::path::PathBuf,
    // Phase 3 — LSP
    pub lsp: Arc<LspManager>,
    pub lsp_diag_rx: Option<tokio::sync::mpsc::Receiver<DiagnosticEvent>>,
    pub hover_text: Option<String>,
    pub completion_items: Vec<CompletionItem>,
    pub completion_selected: usize,
    pub completion_visible: bool,
    pub code_actions: Vec<CodeAction>,
    pub goto_locations: Vec<Location>,
    /// Buffer version counter — incremented on each edit for LSP didChange.
    pub buffer_version: i64,
    // Phase 4 — AI
    pub ai: BackendRegistry,
    pub ghost: GhostText,
    pub model_picker_open: bool,
    pub model_picker_entries: Vec<ModelEntry>,
    pub model_picker_selected: usize,
    // Phase 5 (numbering in PLAN) — Git integration:
    pub git: GitManager,
    pub git_panel_open: bool,
    pub git_branch_panel_open: bool,
    pub git_branch_selected: usize,
    pub git_blame_active: bool,
    pub git_commit_msg: String,
    // Phase 7 — Plugins + Theme
    pub plugins: PluginRegistry,
    pub theme: Theme,
    // Phase 8 — BYOK + Spend + Approvals:
    pub key_vault: Arc<KeyVault>,
    pub spend: Arc<SpendTracker>,
    pub approval_queue: Arc<ApprovalQueue>,
    pub cache: Arc<ResponseCache>,
    pub offline_mode: bool,
    pub model_matrix_open: bool,
    // Phase 9 — Agent:
    pub agent_session: Option<Arc<tokio::sync::Mutex<ai::agent::AgentSession>>>,
    pub agent_memory: Arc<AgentMemory>,
    pub prompt_library: PromptLibrary,
    pub context_sources: ui::widgets::context_picker::ContextSources,
    pub agent_update_rx: Option<tokio::sync::mpsc::UnboundedReceiver<AgentUpdate>>,
    pub agent_panel_open: bool,
    pub tool_trace_open: bool,
    pub prompt_library_open: bool,
    pub context_picker_open: bool,
    pub agent_memory_open: bool,
    // Phase 10:
    pub file_tree: ui::widgets::file_tree::FileTreeState,
    pub file_tree_open: bool,
    pub file_tree_focused: bool,
    pub tab_bar: ui::widgets::tab_bar::TabBarState,
    pub find_replace: ui::widgets::find_replace::FindReplaceState,
    pub symbol_browser: ui::widgets::symbol_browser::SymbolBrowserState,
    pub dap_client: Arc<dap::DapClient>,
    pub breakpoints: Arc<dap::BreakpointManager>,
    pub dap_panel_open: bool,
    pub minimap_open: bool,
    pub minimap_state: ui::widgets::minimap::MinimapState,
    pub bookmarks: ui::widgets::bookmarks::BookmarkManager,
    pub bookmark_picker_open: bool,
    pub macros: ui::widgets::macro_panel::MacroManager,
    pub macro_panel_open: bool,
    pub clipboard_ring: ui::widgets::clipboard_ring::ClipboardRing,
    pub clipboard_picker_open: bool,
    pub session_manager: ui::widgets::session_manager::SessionManager,
    pub session_picker_open: bool,
    // Phase 11 — Task Runner, Live Server, Process Manager, Env, HTTP, DB:
    pub task_runner: Arc<runner::TaskRunner>,
    pub live_server: Arc<runner::LiveServer>,
    pub process_mgr: Arc<runner::ProcessManager>,
    pub env_mgr: Arc<runner::EnvManager>,
    pub http_client: Arc<runner::HttpClient>,
    pub db_client: Arc<runner::DbClient>,
    pub log_viewer_state: ui::widgets::log_viewer::LogViewerState,
    pub task_runner_open: bool,
    pub log_viewer_open: bool,
    pub diff_review_open: bool,
    pub process_panel_open: bool,
    pub port_panel_open: bool,
    pub deploy_panel_open: bool,
    pub env_panel_open: bool,
    pub http_panel_open: bool,
    pub db_panel_open: bool,
    // Phase 12 — Intelligence, Security & Polish:
    pub semantic_search_open: bool,
    pub commit_composer_open: bool,
    pub security_panel_open: bool,
    pub analytics_panel_open: bool,
    pub notebook_open: bool,
    pub keymap_editor_open: bool,
    pub plugin_marketplace_open: bool,
    pub collab_open: bool,
    pub pair_programmer_active: bool,
    pub command_palette_open: bool,
    // Terminal emulator
    pub terminal_session: Option<std::sync::Arc<parking_lot::Mutex<terminal::session::PtySession>>>,
    pub terminal_mode: bool,
}

impl AppState {
    pub fn new(config: Config) -> Self {
        let workspace_root =
            std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
        let (lsp, lsp_diag_rx) = LspManager::new(workspace_root.clone());
        let ai = ai::build_registry(
            config.ai.anthropic_api_key.clone(),
            config.ai.google_api_key.clone(),
            config.ai.openai_api_key.clone(),
            &config.ai.backend,
        );
        let git = GitManager::new(&workspace_root);
        let theme_name = config.theme.clone();
        // Capture key strings before moving config into Arc
        let anthropic_key = config.ai.anthropic_api_key.clone();
        let openai_key = config.ai.openai_api_key.clone();
        let gemini_key = config.ai.google_api_key.clone();
        // Phase 9 — pre-capture workspace_root clone for agent memory / prompt library
        let agent_memory = Arc::new(AgentMemory::new(&workspace_root));
        let prompt_library = PromptLibrary::load(Some(&workspace_root));
        // Phase 10 — pre-capture workspace_root clone for bookmarks / file tree
        let workspace_root_p10 = workspace_root.clone();
        Self {
            config: Arc::new(config),
            buffers: BufferRegistry::new(),
            ids: IdGen::default(),
            active_editor: None,
            file_picker_open: false,
            workspace_root,
            lsp,
            lsp_diag_rx: Some(lsp_diag_rx),
            hover_text: None,
            completion_items: Vec::new(),
            completion_selected: 0,
            completion_visible: false,
            code_actions: Vec::new(),
            goto_locations: Vec::new(),
            buffer_version: 0,
            ai,
            ghost: GhostText::new(),
            model_picker_open: false,
            model_picker_entries: Vec::new(),
            model_picker_selected: 0,
            git,
            git_panel_open: false,
            git_branch_panel_open: false,
            git_branch_selected: 0,
            git_blame_active: false,
            git_commit_msg: String::new(),
            plugins: {
                let mut reg = PluginRegistry::new();
                reg.load_all();
                reg
            },
            theme: ui::theme::load_theme(&theme_name),
            key_vault: {
                let vault = Arc::new(KeyVault::new());
                // Migrate existing config keys to vault
                vault.seed_from_config("claude", anthropic_key.as_deref());
                vault.seed_from_config("openai", openai_key.as_deref());
                vault.seed_from_config("gemini", gemini_key.as_deref());
                vault
            },
            spend: Arc::new(SpendTracker::new()),
            approval_queue: Arc::new(ApprovalQueue::new()),
            cache: Arc::new(ResponseCache::new()),
            offline_mode: false,
            model_matrix_open: false,
            // Phase 9 — Agent
            agent_session: None,
            agent_memory,
            prompt_library,
            context_sources: ui::widgets::context_picker::ContextSources::default(),
            agent_update_rx: None,
            agent_panel_open: false,
            tool_trace_open: false,
            prompt_library_open: false,
            context_picker_open: false,
            agent_memory_open: false,
            // Phase 10:
            file_tree: ui::widgets::file_tree::FileTreeState::new(&workspace_root_p10),
            file_tree_open: false,
            file_tree_focused: false,
            tab_bar: ui::widgets::tab_bar::TabBarState::new(),
            find_replace: ui::widgets::find_replace::FindReplaceState::new(),
            symbol_browser: ui::widgets::symbol_browser::SymbolBrowserState::new(),
            dap_client: Arc::new(dap::DapClient::new()),
            breakpoints: Arc::new(dap::BreakpointManager::new()),
            dap_panel_open: false,
            minimap_open: false,
            minimap_state: ui::widgets::minimap::MinimapState::new(),
            bookmarks: ui::widgets::bookmarks::BookmarkManager::new(&workspace_root_p10),
            bookmark_picker_open: false,
            macros: ui::widgets::macro_panel::MacroManager::new(),
            macro_panel_open: false,
            clipboard_ring: {
                let ring = ui::widgets::clipboard_ring::ClipboardRing::new();
                let _ = ring.load();
                ring
            },
            clipboard_picker_open: false,
            session_manager: ui::widgets::session_manager::SessionManager::new(),
            session_picker_open: false,
            // Phase 11 — Task Runner, Live Server, Process Manager, Env, HTTP, DB:
            task_runner: {
                let tr = Arc::new(runner::TaskRunner::new(&workspace_root_p10));
                let _ = tr.load_tasks(); // ignore error if tasks.toml absent
                tr
            },
            live_server: Arc::new(runner::LiveServer::new(workspace_root_p10.clone(), 3000)),
            process_mgr: Arc::new(runner::ProcessManager::new()),
            env_mgr: {
                let em = Arc::new(runner::EnvManager::new(&workspace_root_p10));
                em.load();
                em
            },
            http_client: {
                let hc = Arc::new(runner::HttpClient::new(&workspace_root_p10));
                let _ = hc.load_collections();
                hc
            },
            db_client: Arc::new(runner::DbClient::new()),
            log_viewer_state: ui::widgets::log_viewer::LogViewerState::new(),
            task_runner_open: false,
            log_viewer_open: false,
            diff_review_open: false,
            process_panel_open: false,
            port_panel_open: false,
            deploy_panel_open: false,
            env_panel_open: false,
            http_panel_open: false,
            db_panel_open: false,
            // Phase 12:
            semantic_search_open: false,
            commit_composer_open: false,
            security_panel_open: false,
            analytics_panel_open: false,
            notebook_open: false,
            keymap_editor_open: false,
            plugin_marketplace_open: false,
            collab_open: false,
            pair_programmer_active: false,
            command_palette_open: false,
            // Terminal emulator — populated after spawn in main.rs
            terminal_session: None,
            terminal_mode: false,
        }
    }

    // ── Phase 8 helpers ──────────────────────────────────────────────────────

    /// Toggle offline mode.
    pub fn toggle_offline(&mut self) -> bool {
        self.offline_mode = self.cache.toggle_offline();
        self.offline_mode
    }

    /// Get spend status string for status bar.
    pub fn spend_status(&self) -> String {
        self.spend.status_string()
    }

    /// Build keyring panel entries from vault.
    pub fn keyring_entries(&self) -> Vec<ui::widgets::keyring_panel::KeyEntry> {
        self.key_vault.list_keys().into_iter().map(|(id, meta)| {
            let raw_key = self.key_vault.get_key(&id).unwrap_or_default();
            ui::widgets::keyring_panel::KeyEntry {
                provider: id.provider.clone(),
                label: id.label.clone(),
                masked: ai::KeyMeta::mask(&raw_key),
                age_days: meta.age_days(),
                last_used: meta.last_used,
                needs_rotation: meta.needs_rotation(),
            }
        }).collect()
    }

    // ── AI helpers ────────────────────────────────────────────────────────────

    /// Build an EditorContext snapshot from the active buffer.
    pub fn build_editor_context(&self) -> EditorContext {
        let mut ctx = EditorContext::new(self.config.ai.max_context_tokens);
        if let Some(ae) = &self.active_editor {
            if let Some(buf_arc) = self.buffers.get(ae.buffer_id) {
                let buf = buf_arc.read();
                ctx.file_path = buf.path.as_ref().map(|p| p.display().to_string());
                ctx.language = buf
                    .path
                    .as_ref()
                    .and_then(|p| p.extension())
                    .and_then(|e| e.to_str())
                    .map(|e| e.to_string());
                ctx.file_content = Some(buf.text.to_string_lossy());
                ctx.cursor_line = Some(ae.view.cursor.line);
            }
        }
        ctx
    }

    /// Build model picker entries from all registered backends.
    pub async fn build_model_picker_entries(&self) -> Vec<ModelEntry> {
        let all_models = self.ai.list_all_models().await;
        let mut entries = Vec::new();
        for (backend_name, models) in all_models {
            for m in models {
                entries.push(ModelEntry {
                    backend: backend_name.clone(),
                    model_id: m.id,
                    model_name: m.name,
                    context_window: m.context_window,
                });
            }
        }
        entries
    }

    /// Open the model picker by loading entries.
    pub async fn open_model_picker(&mut self) {
        self.model_picker_entries = self.build_model_picker_entries().await;
        self.model_picker_selected = 0;
        self.model_picker_open = true;
    }

    /// Apply the currently selected model picker entry.
    pub fn model_picker_confirm(&mut self) {
        if let Some(entry) = self.model_picker_entries.get(self.model_picker_selected) {
            self.ai.set_active(&entry.backend);
            self.ai.set_active_model(&entry.model_id);
        }
        self.model_picker_open = false;
    }

    /// Format AI status string for status bar display.
    pub fn ai_status_string(&self) -> String {
        let display = self.ai.active_display();
        let (last_in, last_out) = self.ai.last_usage();
        if last_in > 0 || last_out > 0 {
            format!("[{display}] ↑{last_in} ↓{last_out}")
        } else if !display.is_empty() {
            format!("[{display}]")
        } else {
            String::new()
        }
    }

    // ── Git helpers ────────────────────────────────────────────────────────────

    /// Current git branch name (empty if no repo).
    pub fn git_branch(&self) -> String {
        self.git.current_branch()
    }

    /// Refresh git status and return clone.
    pub fn git_refresh_status(&self) -> Option<git::RepoStatus> {
        self.git.refresh_status();
        self.git.status.read().clone()
    }

    /// Refresh gutter for the active buffer.
    pub fn git_refresh_gutter(&self) -> Vec<git::GutterLine> {
        if let Some(ae) = &self.active_editor {
            if let Some(buf_arc) = self.buffers.get(ae.buffer_id) {
                let buf = buf_arc.read();
                if let Some(path) = &buf.path {
                    let content = buf.text.to_string_lossy();
                    self.git.refresh_gutter(path, &content);
                }
            }
        }
        self.git.gutter_cache.read().clone()
    }

    /// Refresh blame for the active buffer.
    pub fn git_refresh_blame(&self) -> Vec<git::BlameLine> {
        if let Some(ae) = &self.active_editor {
            if let Some(buf_arc) = self.buffers.get(ae.buffer_id) {
                let buf = buf_arc.read();
                if let Some(path) = &buf.path {
                    self.git.refresh_blame(path);
                }
            }
        }
        self.git.blame_cache.read().clone()
    }

    /// Stage the currently selected file in the git panel.
    pub fn git_stage_selected(&self, status: &git::RepoStatus, idx: usize, section: &ui::widgets::git_panel::GitSection) {
        let file = match section {
            ui::widgets::git_panel::GitSection::Unstaged => status.unstaged.get(idx),
            ui::widgets::git_panel::GitSection::Untracked => status.untracked.get(idx),
            _ => None,
        };
        if let Some(entry) = file {
            let _ = self.git.stage_file(&entry.path);
        }
    }

    /// Unstage the currently selected file.
    pub fn git_unstage_selected(&self, status: &git::RepoStatus, idx: usize) {
        if let Some(entry) = status.staged.get(idx) {
            let _ = self.git.unstage_file(&entry.path);
        }
    }

    /// Open a file into a new buffer. Returns the new BufferId.
    pub fn open_file(&mut self, path: &std::path::Path) -> anyhow::Result<BufferId> {
        let data = std::fs::read(path)?;
        let id = self
            .buffers
            .new_buffer_from_file(path.to_path_buf(), data);
        self.active_editor = Some(ActiveEditor::new(id));
        Ok(id)
    }

    // ── LSP helpers ──────────────────────────────────────────────────────────

    /// Notify LSP server that the active buffer was opened.
    pub async fn lsp_did_open(&self) {
        let ae = match &self.active_editor {
            Some(ae) => ae,
            None => return,
        };
        let buf_arc = match self.buffers.get(ae.buffer_id) {
            Some(b) => b,
            None => return,
        };
        let (path, text) = {
            let buf = buf_arc.read();
            let p = buf.path.clone();
            let t = buf.text.to_string_lossy();
            (p, t)
        };
        if let Some(path) = path {
            self.lsp.did_open(&path, &text, self.buffer_version).await;
        }
    }

    /// Notify LSP server that the active buffer changed.
    pub async fn lsp_did_change(&self) {
        let ae = match &self.active_editor {
            Some(ae) => ae,
            None => return,
        };
        let buf_arc = match self.buffers.get(ae.buffer_id) {
            Some(b) => b,
            None => return,
        };
        let (path, text) = {
            let buf = buf_arc.read();
            let p = buf.path.clone();
            let t = buf.text.to_string_lossy();
            (p, t)
        };
        if let Some(path) = path {
            self.lsp.did_change(&path, &text, self.buffer_version).await;
        }
    }

    /// Request completions at the cursor position.
    pub async fn lsp_request_completions(&mut self) -> Vec<lsp::types::CompletionItem> {
        let (path, line, col) = match self.cursor_context() {
            Some(x) => x,
            None => return vec![],
        };
        let items = self.lsp.completions(&path, line as u32, col as u32).await;
        self.completion_items = items.clone();
        self.completion_selected = 0;
        self.completion_visible = !items.is_empty();
        items
    }

    /// Request hover documentation at the cursor.
    pub async fn lsp_request_hover(&mut self) -> Option<String> {
        let (path, line, col) = self.cursor_context()?;
        let text = self.lsp.hover(&path, line as u32, col as u32).await;
        self.hover_text = text.clone();
        text
    }

    /// Go to definition — returns locations (populated into `goto_locations`).
    pub async fn lsp_goto_definition(&mut self) -> Vec<Location> {
        let (path, line, col) = match self.cursor_context() {
            Some(x) => x,
            None => return vec![],
        };
        let locs = self.lsp.definition(&path, line as u32, col as u32).await;
        self.goto_locations = locs.clone();
        locs
    }

    /// Find references — returns locations.
    pub async fn lsp_goto_references(&mut self) -> Vec<Location> {
        let (path, line, col) = match self.cursor_context() {
            Some(x) => x,
            None => return vec![],
        };
        let locs = self.lsp.references(&path, line as u32, col as u32).await;
        self.goto_locations = locs.clone();
        locs
    }

    /// Format the active buffer and apply edits.
    pub async fn lsp_format(&mut self) {
        let (path, _, _) = match self.cursor_context() {
            Some(x) => x,
            None => return,
        };
        let edits = self.lsp.format(&path).await;
        if edits.is_empty() {
            return;
        }
        let ae = match &self.active_editor {
            Some(ae) => ae,
            None => return,
        };
        let buf_arc = match self.buffers.get(ae.buffer_id) {
            Some(b) => b,
            None => return,
        };
        // Apply edits in reverse order (bottom-up) to preserve offsets
        let mut sorted_edits = edits;
        sorted_edits.sort_by(|a, b| {
            b.range.start.line
                .cmp(&a.range.start.line)
                .then(b.range.start.character.cmp(&a.range.start.character))
        });
        let mut buf = buf_arc.write();
        for edit in sorted_edits {
            let start_line = edit.range.start.line as usize;
            let start_char = edit.range.start.character as usize;
            let end_line = edit.range.end.line as usize;
            let end_char = edit.range.end.character as usize;
            let start_off = buf.text.line_col_to_offset(start_line, start_char);
            let end_off = buf.text.line_col_to_offset(end_line, end_char);
            if start_off < end_off {
                buf.delete(start_off, end_off);
            }
            buf.insert(start_off, &edit.new_text);
        }
        buf.dirty = true;
    }

    /// Returns `(path, cursor_line, cursor_col)` for the active editor.
    fn cursor_context(&self) -> Option<(std::path::PathBuf, usize, usize)> {
        let ae = self.active_editor.as_ref()?;
        let buf_arc = self.buffers.get(ae.buffer_id)?;
        let path = buf_arc.read().path.clone()?;
        Some((path, ae.view.cursor.line, ae.view.cursor.col))
    }

    /// Apply an EditorCommand to the active editor + buffer.
    /// Returns true if the display changed (needs re-render).
    pub fn apply_command(&mut self, cmd: EditorCommand) -> bool {
        let ae = match &mut self.active_editor {
            Some(ae) => ae,
            None => return false,
        };
        let buffer_id = ae.buffer_id;
        let buf_arc = match self.buffers.get(buffer_id) {
            Some(b) => b,
            None => return false,
        };

        match cmd {
            EditorCommand::MoveLeft(n) => {
                let mut buf = buf_arc.write();
                let line_text = buf.line_content(ae.view.cursor.line);
                let line_len = line_text.chars().count();
                let new_col = ae.view.cursor.col.saturating_sub(n);
                ae.view.cursor.col = new_col.min(line_len.saturating_sub(1));
                true
            }
            EditorCommand::MoveRight(n) => {
                let buf = buf_arc.read();
                let line_text = buf.line_content(ae.view.cursor.line);
                let line_len = line_text.chars().count();
                let max_col = if line_len > 0 { line_len - 1 } else { 0 };
                ae.view.cursor.col = (ae.view.cursor.col + n).min(max_col);
                true
            }
            EditorCommand::MoveUp(n) => {
                ae.view.cursor.line = ae.view.cursor.line.saturating_sub(n);
                let buf = buf_arc.read();
                clamp_col_to_line(&mut ae.view.cursor, &buf);
                true
            }
            EditorCommand::MoveDown(n) => {
                let buf = buf_arc.read();
                let total = buf.line_count().max(1);
                ae.view.cursor.line = (ae.view.cursor.line + n).min(total - 1);
                clamp_col_to_line(&mut ae.view.cursor, &buf);
                true
            }
            EditorCommand::MoveLineStart => {
                ae.view.cursor.col = 0;
                true
            }
            EditorCommand::MoveLineFirstNonWs => {
                let buf = buf_arc.read();
                let line = buf.line_content(ae.view.cursor.line);
                let col = line.chars().take_while(|c| c.is_whitespace()).count();
                ae.view.cursor.col = col;
                true
            }
            EditorCommand::MoveLineEnd => {
                let buf = buf_arc.read();
                let line = buf.line_content(ae.view.cursor.line);
                let len = line.chars().count();
                ae.view.cursor.col = if len > 0 { len - 1 } else { 0 };
                true
            }
            EditorCommand::MoveFileStart => {
                ae.view.cursor = CursorPos { line: 0, col: 0 };
                true
            }
            EditorCommand::MoveFileEnd => {
                let buf = buf_arc.read();
                let total = buf.line_count().max(1);
                ae.view.cursor.line = total - 1;
                ae.view.cursor.col = 0;
                true
            }
            EditorCommand::MoveToLine(n) => {
                let buf = buf_arc.read();
                let total = buf.line_count().max(1);
                ae.view.cursor.line = n.min(total - 1);
                ae.view.cursor.col = 0;
                true
            }
            EditorCommand::MoveWordForward(n) => {
                let buf = buf_arc.read();
                let total = buf.line_count().max(1);
                let mut cur = ae.view.cursor;
                for _ in 0..n {
                    cur = word_forward(&buf, cur, total);
                }
                ae.view.cursor = cur;
                true
            }
            EditorCommand::MoveWordBackward(n) => {
                let buf = buf_arc.read();
                let mut cur = ae.view.cursor;
                for _ in 0..n {
                    cur = word_backward(&buf, cur);
                }
                ae.view.cursor = cur;
                true
            }
            EditorCommand::MoveWordEnd(n) => {
                let buf = buf_arc.read();
                let total = buf.line_count().max(1);
                let mut cur = ae.view.cursor;
                for _ in 0..n {
                    cur = word_end(&buf, cur, total);
                }
                ae.view.cursor = cur;
                true
            }
            EditorCommand::ScrollHalfPageDown => {
                let buf = buf_arc.read();
                let total = buf.line_count().max(1);
                ae.view.cursor.line = (ae.view.cursor.line + 15).min(total - 1);
                true
            }
            EditorCommand::ScrollHalfPageUp => {
                ae.view.cursor.line = ae.view.cursor.line.saturating_sub(15);
                true
            }
            EditorCommand::InsertChar(c) => {
                let mut buf = buf_arc.write();
                let offset = buf.text.line_col_to_offset(ae.view.cursor.line, ae.view.cursor.col);
                buf.insert(offset, &c.to_string());
                ae.view.cursor.col += 1;
                true
            }
            EditorCommand::InsertNewline => {
                let mut buf = buf_arc.write();
                let offset = buf.text.line_col_to_offset(ae.view.cursor.line, ae.view.cursor.col);
                buf.insert(offset, "\n");
                ae.view.cursor.line += 1;
                ae.view.cursor.col = 0;
                true
            }
            EditorCommand::DeleteCharBackward => {
                let mut buf = buf_arc.write();
                if ae.view.cursor.col > 0 {
                    let end_offset =
                        buf.text.line_col_to_offset(ae.view.cursor.line, ae.view.cursor.col);
                    ae.view.cursor.col -= 1;
                    let start_offset =
                        buf.text.line_col_to_offset(ae.view.cursor.line, ae.view.cursor.col);
                    buf.delete(start_offset, end_offset);
                } else if ae.view.cursor.line > 0 {
                    // Join with previous line
                    let end_offset =
                        buf.text.line_col_to_offset(ae.view.cursor.line, 0);
                    let prev_line = ae.view.cursor.line - 1;
                    let prev_len = buf.line_content(prev_line).chars().count();
                    let start_offset = buf.text.line_col_to_offset(prev_line, prev_len);
                    // Delete the newline character
                    buf.delete(start_offset, end_offset);
                    ae.view.cursor.line = prev_line;
                    ae.view.cursor.col = prev_len;
                }
                true
            }
            EditorCommand::DeleteCharForward => {
                let mut buf = buf_arc.write();
                let line_text = buf.line_content(ae.view.cursor.line);
                let line_len = line_text.chars().count();
                if ae.view.cursor.col < line_len {
                    let start_offset =
                        buf.text.line_col_to_offset(ae.view.cursor.line, ae.view.cursor.col);
                    let end_offset =
                        buf.text.line_col_to_offset(ae.view.cursor.line, ae.view.cursor.col + 1);
                    buf.delete(start_offset, end_offset);
                }
                true
            }
            EditorCommand::DeleteWordBackward => {
                let buf_r = buf_arc.read();
                let word_start = word_backward(&buf_r, ae.view.cursor);
                drop(buf_r);
                let start_offset = {
                    let buf = buf_arc.read();
                    buf.text.line_col_to_offset(word_start.line, word_start.col)
                };
                let end_offset = {
                    let buf = buf_arc.read();
                    buf.text.line_col_to_offset(ae.view.cursor.line, ae.view.cursor.col)
                };
                if start_offset < end_offset {
                    buf_arc.write().delete(start_offset, end_offset);
                    ae.view.cursor = word_start;
                }
                true
            }
            EditorCommand::DeleteToLineStart => {
                let mut buf = buf_arc.write();
                let end_offset =
                    buf.text.line_col_to_offset(ae.view.cursor.line, ae.view.cursor.col);
                let start_offset = buf.text.line_start(ae.view.cursor.line);
                if start_offset < end_offset {
                    buf.delete(start_offset, end_offset);
                    ae.view.cursor.col = 0;
                }
                true
            }
            EditorCommand::DeleteLine(n) => {
                let mut buf = buf_arc.write();
                let total = buf.line_count().max(1);
                let start_line = ae.view.cursor.line;
                let end_line = (start_line + n).min(total);
                let start_offset = buf.text.line_start(start_line);
                let end_offset = if end_line < total {
                    buf.text.line_start(end_line)
                } else {
                    buf.text.len()
                };
                if start_offset < end_offset {
                    let text = String::from_utf8_lossy(
                        &buf.text.bytes_in_range(start_offset, end_offset)
                    ).into_owned();
                    ae.registers.set_unnamed(text, true);
                    buf.delete(start_offset, end_offset);
                }
                let new_total = buf.line_count().max(1);
                if ae.view.cursor.line >= new_total {
                    ae.view.cursor.line = new_total - 1;
                }
                ae.view.cursor.col = 0;
                true
            }
            EditorCommand::YankLine(n) => {
                let buf = buf_arc.read();
                let total = buf.line_count().max(1);
                let start_line = ae.view.cursor.line;
                let end_line = (start_line + n).min(total);
                let start_offset = buf.text.line_start(start_line);
                let end_offset = if end_line < total {
                    buf.text.line_start(end_line)
                } else {
                    buf.text.len()
                };
                let text = String::from_utf8_lossy(
                    &buf.text.bytes_in_range(start_offset, end_offset)
                ).into_owned();
                ae.registers.set_unnamed(text, true);
                true
            }
            EditorCommand::PasteAfter => {
                if let Some(reg) = ae.registers.get_unnamed() {
                    let text = reg.text.clone();
                    let is_line = reg.is_line;
                    let mut buf = buf_arc.write();
                    if is_line {
                        // Paste on next line
                        let total = buf.line_count().max(1);
                        let next_line = (ae.view.cursor.line + 1).min(total);
                        let offset = buf.text.line_start(next_line);
                        buf.insert(offset, &text);
                        ae.view.cursor.line += 1;
                        ae.view.cursor.col = 0;
                    } else {
                        let offset = buf.text.line_col_to_offset(
                            ae.view.cursor.line,
                            ae.view.cursor.col + 1,
                        );
                        buf.insert(offset, &text);
                        ae.view.cursor.col += 1;
                    }
                }
                true
            }
            EditorCommand::PasteBefore => {
                if let Some(reg) = ae.registers.get_unnamed() {
                    let text = reg.text.clone();
                    let is_line = reg.is_line;
                    let mut buf = buf_arc.write();
                    if is_line {
                        let offset = buf.text.line_start(ae.view.cursor.line);
                        buf.insert(offset, &text);
                    } else {
                        let offset = buf.text.line_col_to_offset(
                            ae.view.cursor.line,
                            ae.view.cursor.col,
                        );
                        buf.insert(offset, &text);
                    }
                }
                true
            }
            EditorCommand::ReplaceChar(c) => {
                let mut buf = buf_arc.write();
                let line_text = buf.line_content(ae.view.cursor.line);
                let line_len = line_text.chars().count();
                if ae.view.cursor.col < line_len {
                    let start = buf.text.line_col_to_offset(ae.view.cursor.line, ae.view.cursor.col);
                    let end = buf.text.line_col_to_offset(ae.view.cursor.line, ae.view.cursor.col + 1);
                    buf.delete(start, end);
                    buf.insert(start, &c.to_string());
                }
                true
            }
            EditorCommand::Undo => {
                if let Some(inv_change) = ae.undo.undo() {
                    apply_undo_change(&inv_change, &buf_arc);
                }
                true
            }
            EditorCommand::Redo => {
                if let Some(change) = ae.undo.redo() {
                    apply_undo_change(&change, &buf_arc);
                }
                true
            }
            EditorCommand::EnterInsert => {
                // Nothing extra needed — mode is already set in modal
                true
            }
            EditorCommand::EnterInsertAppend => {
                // Move cursor one right
                let buf = buf_arc.read();
                let line_text = buf.line_content(ae.view.cursor.line);
                let line_len = line_text.chars().count();
                ae.view.cursor.col = (ae.view.cursor.col + 1).min(line_len);
                true
            }
            EditorCommand::EnterInsertBOL => {
                ae.view.cursor.col = 0;
                true
            }
            EditorCommand::EnterInsertEOL => {
                let buf = buf_arc.read();
                let line_text = buf.line_content(ae.view.cursor.line);
                ae.view.cursor.col = line_text.chars().count();
                true
            }
            EditorCommand::EnterInsertNewlineBelow => {
                let mut buf = buf_arc.write();
                let line_text = buf.line_content(ae.view.cursor.line);
                let line_len = line_text.chars().count();
                let offset = buf.text.line_col_to_offset(ae.view.cursor.line, line_len);
                buf.insert(offset, "\n");
                ae.view.cursor.line += 1;
                ae.view.cursor.col = 0;
                true
            }
            EditorCommand::EnterInsertNewlineAbove => {
                let mut buf = buf_arc.write();
                let offset = buf.text.line_start(ae.view.cursor.line);
                buf.insert(offset, "\n");
                ae.view.cursor.col = 0;
                true
            }
            EditorCommand::EnterVisual(_kind) => {
                ae.view.visual_anchor = Some(ae.view.cursor);
                true
            }
            EditorCommand::EnterNormal => {
                ae.view.visual_anchor = None;
                true
            }
            EditorCommand::ToggleFold => {
                ae.folds.toggle(ae.view.cursor.line);
                true
            }
            EditorCommand::FoldAll => {
                ae.folds.fold_all();
                true
            }
            EditorCommand::UnfoldAll => {
                ae.folds.unfold_all();
                true
            }
            EditorCommand::OpenFilePicker => {
                self.file_picker_open = true;
                true
            }
            // These are handled in the main loop:
            EditorCommand::SaveFile
            | EditorCommand::Quit
            | EditorCommand::ForceQuit
            | EditorCommand::SplitH
            | EditorCommand::SplitV
            | EditorCommand::EnterCommandLine
            | EditorCommand::EnterSearch(_)
            | EditorCommand::SearchNext
            | EditorCommand::SearchPrev
            | EditorCommand::SearchInput(_)
            | EditorCommand::SearchBackspace
            | EditorCommand::SearchConfirm
            | EditorCommand::SearchCancel
            | EditorCommand::CmdInput(_)
            | EditorCommand::CmdBackspace
            | EditorCommand::CmdConfirm
            | EditorCommand::CmdCancel
            | EditorCommand::CmdHistoryUp
            | EditorCommand::CmdHistoryDown => false,

            // Stubs
            _ => false,
        }
    }

    /// Process a confirmed command-line string like ":w", ":q", ":e path"
    pub fn process_cmdline(&mut self, cmd: &str) -> Option<AppAction> {
        let cmd = cmd.trim_start_matches(':').trim();
        if cmd == "w" || cmd == "write" {
            // Save active buffer
            if let Some(ae) = &self.active_editor {
                if let Some(buf_arc) = self.buffers.get(ae.buffer_id) {
                    let buf = buf_arc.read();
                    if let Some(path) = &buf.path {
                        let bytes = buf.text.to_bytes();
                        let _ = std::fs::write(path, &bytes);
                    }
                }
            }
            None
        } else if cmd == "q" || cmd == "quit" {
            Some(AppAction::Quit)
        } else if cmd == "q!" || cmd == "quit!" {
            Some(AppAction::ForceQuit)
        } else if cmd == "wq" {
            // Save then quit
            self.process_cmdline("w");
            Some(AppAction::Quit)
        } else if let Some(path) = cmd.strip_prefix("e ").or_else(|| cmd.strip_prefix("edit ")) {
            let p = std::path::Path::new(path.trim());
            let _ = self.open_file(p);
            None
        } else if cmd == "sp" || cmd == "split" {
            Some(AppAction::SplitH)
        } else if cmd == "vs" || cmd == "vsplit" {
            Some(AppAction::SplitV)
        } else if let Some(name) = cmd.strip_prefix("colorscheme ").or_else(|| cmd.strip_prefix("color ")) {
            let name = name.trim().to_string();
            self.theme = ui::theme::load_theme(&name);
            Some(AppAction::SetTheme(name))
        } else if cmd == "AgentResume" {
            if let Some(session_arc) = &self.agent_session {
                if let Ok(mut s) = session_arc.try_lock() {
                    s.resume();
                }
            }
            None
        } else if cmd == "AgentHistory" {
            Some(AppAction::InfoMessage("AgentHistory: session stored in .rmtide/agent-session.json".to_string()))
        } else if cmd == "AgentMemory" {
            self.agent_memory_open = !self.agent_memory_open;
            None
        } else if cmd == "AiContext" {
            self.context_picker_open = !self.context_picker_open;
            None
        } else if cmd == "AiPrompts" {
            self.prompt_library_open = !self.prompt_library_open;
            None
        } else {
            None
        }
    }

    /// Build an EditorDisplay snapshot for the renderer.
    pub fn make_editor_display(&self) -> Option<EditorDisplay> {
        let ae = self.active_editor.as_ref()?;
        let buf_arc = self.buffers.get(ae.buffer_id)?;
        let buf = buf_arc.read();
        let total = buf.line_count();
        let lines: Vec<String> = (0..total).map(|i| buf.line_content(i)).collect();
        let file_name = buf.path.as_ref().and_then(|p| {
            p.file_name().map(|n| n.to_string_lossy().into_owned())
        });
        Some(EditorDisplay {
            lines,
            cursor_line: ae.view.cursor.line,
            cursor_col: ae.view.cursor.col,
            scroll_row: ae.view.scroll_row,
            scroll_col: ae.view.scroll_col,
            is_dirty: buf.dirty,
            file_name,
            selection: ae.view.visual_range().map(|(s, e)| ((s.line, s.col), (e.line, e.col))),
        })
    }
}

pub enum AppAction {
    Quit,
    ForceQuit,
    SplitH,
    SplitV,
    OpenFilePicker,
    SetTheme(String),
    InfoMessage(String),
}

// ─── Helpers ────────────────────────────────────────────────────────────────

fn clamp_col_to_line(cursor: &mut CursorPos, buf: &editor::buffer::EditorBuffer) {
    let line_text = buf.line_content(cursor.line);
    let len = line_text.chars().count();
    if len == 0 {
        cursor.col = 0;
    } else if cursor.col >= len {
        cursor.col = len - 1;
    }
}

fn word_forward(
    buf: &editor::buffer::EditorBuffer,
    mut cur: CursorPos,
    total: usize,
) -> CursorPos {
    let line_text = buf.line_content(cur.line);
    let chars: Vec<char> = line_text.chars().collect();
    let len = chars.len();

    // Skip current word
    while cur.col < len && !chars[cur.col].is_alphanumeric() && chars[cur.col] != '_' {
        cur.col += 1;
    }
    while cur.col < len && (chars[cur.col].is_alphanumeric() || chars[cur.col] == '_') {
        cur.col += 1;
    }
    // Skip whitespace
    while cur.col < len && chars[cur.col].is_whitespace() {
        cur.col += 1;
    }

    if cur.col >= len && cur.line + 1 < total {
        cur.line += 1;
        cur.col = 0;
    }
    cur
}

fn word_backward(buf: &editor::buffer::EditorBuffer, mut cur: CursorPos) -> CursorPos {
    if cur.col == 0 {
        if cur.line > 0 {
            cur.line -= 1;
            let line_text = buf.line_content(cur.line);
            cur.col = line_text.chars().count().saturating_sub(1);
        }
        return cur;
    }

    let line_text = buf.line_content(cur.line);
    let chars: Vec<char> = line_text.chars().collect();

    cur.col = cur.col.saturating_sub(1);
    while cur.col > 0 && chars[cur.col].is_whitespace() {
        cur.col -= 1;
    }
    while cur.col > 0 && (chars[cur.col - 1].is_alphanumeric() || chars[cur.col - 1] == '_') {
        cur.col -= 1;
    }
    cur
}

fn word_end(
    buf: &editor::buffer::EditorBuffer,
    mut cur: CursorPos,
    total: usize,
) -> CursorPos {
    let line_text = buf.line_content(cur.line);
    let chars: Vec<char> = line_text.chars().collect();
    let len = chars.len();

    cur.col += 1;
    if cur.col >= len {
        if cur.line + 1 < total {
            cur.line += 1;
            cur.col = 0;
        }
        return cur;
    }

    while cur.col < len && chars[cur.col].is_whitespace() {
        cur.col += 1;
    }
    while cur.col + 1 < len && (chars[cur.col + 1].is_alphanumeric() || chars[cur.col + 1] == '_') {
        cur.col += 1;
    }
    cur
}

fn apply_undo_change(
    change: &editor::undo::Change,
    buf_arc: &std::sync::Arc<parking_lot::RwLock<editor::buffer::EditorBuffer>>,
) {
    use editor::undo::Change;
    match change {
        Change::Insert { offset, text } => {
            let s = String::from_utf8_lossy(text).into_owned();
            buf_arc.write().insert(*offset, &s);
        }
        Change::Delete { offset, text } => {
            let end = offset + text.len();
            buf_arc.write().delete(*offset, end);
        }
        Change::Group(changes) => {
            for c in changes {
                apply_undo_change(c, buf_arc);
            }
        }
    }
}
