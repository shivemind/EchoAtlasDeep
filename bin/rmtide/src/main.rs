#![allow(dead_code, unused_imports, unused_variables, unused_mut)]
mod config;
mod app;

use std::sync::Arc;
use parking_lot::RwLock;
use tokio::sync::Notify;
use tracing::info;

use rmcore::bus::EventBus;
use rmcore::event::AppEvent;
use rmcore::ids::IdGen;

use ui::layout::{LayoutTree, SplitDir};
use ui::pane::{Pane, PaneKind};
use ui::render::{RenderState, spawn_render_task};
use ui::input::spawn_input_task;
use ui::widgets::cmdline::CmdLineState;

use editor::modal::{EditorCommand, SearchDir};

use app::{AppAction, AppState};
use config::Config;
use ui::theme;

// LSP types used in handlers
use lsp;

// AI types used in handlers
use ai;

// Git types used in handlers
use git;
use ui::widgets::git_panel::{GitPanelState, GitSection};

// MCP server
use mcp;

// DAP debugger
use dap;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // ── Logging ──────────────────────────────────────────────────────────────
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("rmtide=debug".parse()?)
                .add_directive("terminal=debug".parse()?)
        )
        .with_writer(std::io::stderr)
        .init();

    info!("EchoAtlasDeep — rmtide starting");

    // ── Config ───────────────────────────────────────────────────────────────
    let config = Config::load().unwrap_or_else(|e| {
        tracing::warn!("Config load error ({e}), using defaults");
        Config::default()
    });

    // ── Event bus ────────────────────────────────────────────────────────────
    let bus = EventBus::new();
    let render_rx = bus.subscribe();
    let input_tx  = bus.sender();

    // ── App state ────────────────────────────────────────────────────────────
    let mut ids = IdGen::default();

    // ── Initial layout: one terminal pane ────────────────────────────────────
    let terminal_session_id = ids.next_session();
    let first_pane = Pane::new(ids.next_pane(), PaneKind::Terminal { session_id: terminal_session_id });
    let layout = LayoutTree::new(first_pane);

    let render_state = Arc::new(RwLock::new(RenderState {
        layout,
        mode: "NORMAL".into(),
        backend_name: config.ai.backend.clone(),
        editor_display: None,
        file_picker: None,
        cmdline: None,
        quickfix: Vec::new(),
        search_query: String::new(),
        search_is_active: false,
        cmdline_is_active: false,
        // Phase 3 — LSP
        completion: None,
        hover: None,
        diagnostics_panel: Vec::new(),
        diag_panel_selected: 0,
        lsp_errors: 0,
        lsp_warnings: 0,
        // Phase 4 — AI
        chat_session: None,
        ghost_text: None,
        model_picker: None,
        model_picker_selected: 0,
        ai_status: String::new(),
        // Phase 4 — Git
        git_panel: None,
        git_panel_open: false,
        git_panel_state: ui::widgets::git_panel::GitPanelState::new(),
        git_gutter: Vec::new(),
        git_blame: None,
        git_blame_active: false,
        git_branches: Vec::new(),
        git_branch_panel_open: false,
        git_branch_selected: 0,
        git_branch_name: String::new(),
        // Phase 7 — Theme
        theme: Some(ui::theme::load_theme(&config.theme)),
        theme_name: config.theme.clone(),
        // Phase 8 — BYOK + Spend + Approvals
        keyring_open: false,
        keyring_state: ui::widgets::keyring_panel::KeyringPanelState::new(),
        spend_panel_open: false,
        spend_state: ui::widgets::spend_panel::SpendPanelState {
            breakdown: String::new(),
            session_cost: 0.0,
            session_budget: 0.0,
            budget_fraction: 0.0,
            over_budget: false,
            warning: false,
            ai_status: String::new(),
        },
        approval_modal: ai::approval::ApprovalModalState::new(),
        model_matrix_open: false,
        model_matrix_selected: 0,
        model_matrix_entries: ai::spend::pricing_table(),
        offline_mode: false,
        spend_status: String::new(),
        // Phase 9 — Agent
        agent_panel_open: false,
        agent_panel_state: ui::widgets::agent_panel::AgentPanelState::new(),
        tool_trace_open: false,
        prompt_library_state: ui::widgets::prompt_picker::PromptLibraryState::new(),
        context_picker_state: ui::widgets::context_picker::ContextPickerState::new(),
        agent_memory_entries: Vec::new(),
        agent_memory_open: false,
        agent_status_str: String::new(),
        // Phase 10 — File Tree, Tabs, Find/Replace, DAP, etc:
        file_tree_open: false,
        file_tree_focused: false,
        file_tree_state: ui::widgets::file_tree::FileTreeState::new(
            &std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."))
        ),
        tab_bar: ui::widgets::tab_bar::TabBarState::new(),
        find_replace: ui::widgets::find_replace::FindReplaceState::new(),
        symbol_browser: ui::widgets::symbol_browser::SymbolBrowserState::new(),
        dap_panel: ui::widgets::dap_panel::DapPanelState::new(),
        minimap_open: false,
        minimap_state: ui::widgets::minimap::MinimapState::new(),
        bookmark_picker: ui::widgets::bookmarks::BookmarkPickerState::new(),
        macro_panel: ui::widgets::macro_panel::MacroPanelState::new(),
        clipboard_picker: ui::widgets::clipboard_ring::ClipboardPickerState::new(),
        session_picker: ui::widgets::session_manager::SessionPickerState::new(),
        // Phase 11
        task_runner_open: false,
        task_runner_state: ui::widgets::task_runner_panel::TaskRunnerState::new(),
        task_records: Vec::new(),
        log_viewer_open: false,
        log_viewer_state: ui::widgets::log_viewer::LogViewerState::new(),
        live_server_url: None,
        diff_review_state: ui::widgets::diff_review::DiffReviewState::new(),
        process_panel_open: false,
        process_panel_state: ui::widgets::process_panel::ProcessPanelState::new(),
        processes: Vec::new(),
        port_panel_open: false,
        port_panel_state: ui::widgets::port_panel::PortPanelState::new(),
        deploy_panel_open: false,
        deploy_panel_state: ui::widgets::deploy_panel::DeployPanelState::new(
            &std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."))
        ),
        env_panel_open: false,
        env_panel_state: ui::widgets::env_panel::EnvPanelState::new(),
        http_panel_open: false,
        http_panel_state: ui::widgets::http_panel::HttpPanelState::new(),
        db_panel_open: false,
        db_panel_state: ui::widgets::db_panel::DbPanelState::new(),
        // Phase 12 — Intelligence, Security & Polish:
        semantic_search: ui::widgets::semantic_search::SemanticSearchState::new(),
        commit_composer: ui::widgets::commit_composer::CommitComposerState::new(),
        security_panel: ui::widgets::security_panel::SecurityPanelState::new(),
        analytics_panel: ui::widgets::analytics_panel::AnalyticsPanelState::new(),
        notebook_state: ui::widgets::notebook::NotebookState::new(),
        keymap_editor: ui::widgets::keymap_editor::KeymapEditorState::new(),
        plugin_marketplace: ui::widgets::plugin_marketplace::PluginMarketplaceState::new(),
        collab_state: ui::widgets::collab_panel::CollabState::new(),
        pair_programmer: ui::widgets::pair_programmer::PairProgrammerState::new(),
        command_palette: ui::widgets::command_palette::CommandPaletteState::new(),
        terminal_session: None,
    }));

    let notify = Arc::new(Notify::new());

    // ── App state ────────────────────────────────────────────────────────────
    let mut app = AppState::new(config);

    // ── Workspace session restore ─────────────────────────────────────────────
    // Populate file_history from last session (files are not auto-opened,
    // just available for quick-open in the file picker).
    {
        let recent = AppState::workspace_load_recent();
        for p in recent {
            app.file_history.push(p);
        }
    }

    // ── Terminal emulator ────────────────────────────────────────────────────
    {
        let pty_config = terminal::pty::PtyConfig::default();
        match terminal::session::PtySession::spawn(
            terminal_session_id,
            pty_config,
            input_tx.clone(),
        ).await {
            Ok(sess_arc) => {
                app.terminal_session = Some(sess_arc.clone());
                app.terminal_mode = true;
                render_state.write().terminal_session = Some(sess_arc);
            }
            Err(e) => {
                tracing::warn!("Failed to spawn terminal session: {e}");
            }
        }
    }

    // ── MCP Server ───────────────────────────────────────────────────────────
    let (mcp_bridge, mut mcp_cmd_rx) = mcp::create_bridge(app.workspace_root.clone());
    mcp::launch(
        &app.config.mcp.bind_addr,
        app.config.mcp.port,
        app.workspace_root.clone(),
        mcp_bridge.clone(),
    );

    // Command-line accumulator (separate from RenderState for easy access)
    let mut cmdline_state = CmdLineState::new();
    let mut search_buf = String::new();
    let mut search_forward = true;

    // Pending approval responder — set when an approval request arrives from the agent.
    let mut pending_approval_responder: Option<tokio::sync::oneshot::Sender<ai::approval::ApprovalDecision>> = None;

    // Phase 8 — approval queue bridge: forward requests from the parking_lot-protected
    // receiver to a tokio channel so the main select! loop can await on it safely.
    let (approval_bridge_tx, mut approval_bridge_rx) =
        tokio::sync::mpsc::unbounded_channel::<ai::approval::ApprovalRequest>();
    {
        let aq = app.approval_queue.clone();
        tokio::spawn(async move {
            loop {
                // Poll without holding the parking_lot lock across an await boundary.
                let maybe_req = aq.rx.lock().try_recv().ok();
                if let Some(req) = maybe_req {
                    if approval_bridge_tx.send(req).is_err() { break; }
                } else {
                    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                }
            }
        });
    }

    // Phase 11 — subscribe to task runner log channel before the main loop
    let mut log_rx = app.task_runner.subscribe_logs();

    // ── Spawn tasks ──────────────────────────────────────────────────────────
    spawn_render_task(notify.clone(), render_state.clone(), render_rx);
    spawn_input_task(input_tx.clone());

    // ── Main event loop ──────────────────────────────────────────────────────
    let mut event_rx = bus.subscribe();
    info!("Entering main event loop");

    loop {
        tokio::select! {
            mcp_cmd = mcp_cmd_rx.recv() => {
                if let Some(cmd) = mcp_cmd {
                    handle_mcp_command(cmd, &mut app, &render_state, &notify).await;
                }
                continue;
            }
            // ── Phase 11: drain task runner log entries ──────────────────
            log_entry = log_rx.recv() => {
                match log_entry {
                    Ok(entry) => {
                        let mut state = render_state.write();
                        state.log_viewer_state.push(entry.clone());
                        drop(state);
                        app.log_viewer_state.push(entry);
                        // Don't force render every log line — high frequency
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {}
                    Err(_) => {}
                }
                continue;
            }
            // ── Phase 8: poll approval queue ────────────────────────────────
            Some(req) = approval_bridge_rx.recv() => {
                let mut state = render_state.write();
                state.approval_modal.set_pending(&req);
                drop(state);
                pending_approval_responder = Some(req.responder);
                notify.notify_one();
                continue;
            }
            // ── Phase 9: drain agent updates ────────────────────────────────
            Some(agent_update) = async {
                if let Some(rx) = &mut app.agent_update_rx {
                    rx.recv().await
                } else {
                    std::future::pending::<Option<ai::agent::AgentUpdate>>().await
                }
            } => {
                use ai::agent::AgentUpdate;
                match agent_update {
                    AgentUpdate::StatusChanged(status) => {
                        let status_str = format!("[Agent: {}]", status.label());
                        let mut state = render_state.write();
                        state.agent_status_str = status_str.clone();
                        if let Some(session_arc) = &app.agent_session {
                            if let Ok(s) = session_arc.try_lock() {
                                state.agent_panel_state.session = Some(s.clone());
                            }
                        }
                        drop(state);
                        notify.notify_one();
                    }
                    AgentUpdate::PlanReady(plan) => {
                        let mut state = render_state.write();
                        if let Some(ref mut panel_session) = state.agent_panel_state.session {
                            panel_session.plan = plan.clone();
                            panel_session.plan_editable = true;
                        }
                        state.agent_status_str = "[Agent: Plan ready — confirm to start]".to_string();
                        drop(state);
                        notify.notify_one();
                    }
                    AgentUpdate::StepComplete(step) => {
                        let mut state = render_state.write();
                        if let Some(ref mut panel_session) = state.agent_panel_state.session {
                            panel_session.current_step = step + 1;
                        }
                        drop(state);
                        notify.notify_one();
                    }
                    AgentUpdate::ToolCalled(call) => {
                        let mut state = render_state.write();
                        if let Some(ref mut panel_session) = state.agent_panel_state.session {
                            panel_session.tool_trace.push(call);
                        }
                        drop(state);
                        notify.notify_one();
                    }
                    AgentUpdate::Complete(summary) => {
                        let preview: String = summary.chars().take(60).collect();
                        let mut state = render_state.write();
                        state.agent_status_str = format!("[Agent: Complete] {}", preview);
                        drop(state);
                        notify.notify_one();
                    }
                    AgentUpdate::Failed(reason) => {
                        let preview: String = reason.chars().take(60).collect();
                        let mut state = render_state.write();
                        state.agent_status_str = format!("[Agent: Failed] {}", preview);
                        drop(state);
                        notify.notify_one();
                    }
                }
                continue;
            }
            event_result = event_rx.recv() => match event_result {
            Ok(AppEvent::Quit) => {
                info!("Quit received — shutting down");
                app.workspace_save();
                break;
            }
            Ok(AppEvent::KeyInput(key)) => {
                use rmcore::event::{KeyCode, KeyModifiers};

                // Ctrl-Q always quits
                if key.code == KeyCode::Char('q')
                    && key.modifiers.contains(KeyModifiers::CONTROL)
                {
                    let _ = bus.sender().send(AppEvent::Quit);
                    continue;
                }

                // Ctrl-W cycles pane focus
                if key.code == KeyCode::Char('w')
                    && key.modifiers.contains(KeyModifiers::CONTROL)
                {
                    let mut state = render_state.write();
                    state.layout.focus_next();
                    drop(state);
                    notify.notify_one();
                    continue;
                }

                // Ctrl-T toggles terminal mode on/off
                if key.code == KeyCode::Char('t')
                    && key.modifiers.contains(KeyModifiers::CONTROL)
                {
                    app.terminal_mode = !app.terminal_mode;
                    app.file_tree_focused = false;
                    let mode_str = if app.terminal_mode { "TERMINAL" } else { "NORMAL" };
                    let mut state = render_state.write();
                    state.mode = mode_str.to_string();
                    state.file_tree_focused = false;
                    notify.notify_one();
                    continue;
                }

                // Ctrl+; — open command line from any mode (even terminal / tree mode)
                if key.code == KeyCode::Char(';')
                    && key.modifiers.contains(KeyModifiers::CONTROL)
                {
                    app.terminal_mode = false;
                    app.file_tree_focused = false;
                    cmdline_state = CmdLineState::new();
                    {
                        let mut state = render_state.write();
                        state.file_tree_focused = false;
                        state.mode = "COMMAND".to_string();
                        state.cmdline_is_active = true;
                    }
                    notify.notify_one();
                    continue;
                }

                // In terminal mode, route keys to the PTY (except global shortcuts above)
                if app.terminal_mode {
                    if let Some(ref sess_arc) = app.terminal_session {
                        if let Some(bytes) = key_to_bytes(&key) {
                            sess_arc.lock().write_input(bytes);
                        }
                    }
                    continue;
                }

                // Intercept model picker navigation
                if app.model_picker_open {
                    use rmcore::event::KeyCode;
                    match key.code {
                        KeyCode::Up => {
                            if app.model_picker_selected > 0 {
                                app.model_picker_selected -= 1;
                            }
                            let sel = app.model_picker_selected;
                            render_state.write().model_picker_selected = sel;
                            notify.notify_one();
                            continue;
                        }
                        KeyCode::Down => {
                            let max = app.model_picker_entries.len().saturating_sub(1);
                            if app.model_picker_selected < max {
                                app.model_picker_selected += 1;
                            }
                            let sel = app.model_picker_selected;
                            render_state.write().model_picker_selected = sel;
                            notify.notify_one();
                            continue;
                        }
                        KeyCode::Enter => {
                            app.model_picker_confirm();
                            let ai_status = app.ai_status_string();
                            let mut state = render_state.write();
                            state.model_picker = None;
                            state.ai_status = ai_status;
                            notify.notify_one();
                            continue;
                        }
                        KeyCode::Escape => {
                            app.model_picker_open = false;
                            render_state.write().model_picker = None;
                            notify.notify_one();
                            continue;
                        }
                        _ => {}
                    }
                }

                // ── Phase 9: global keybinds ─────────────────────────────────
                {
                    use rmcore::event::{KeyCode, KeyModifiers};
                    // \A — toggle agent panel (Backslash then A, or leader+A)
                    // We detect as Alt+A for simplicity
                    if key.code == KeyCode::Char('A') && key.modifiers.contains(KeyModifiers::ALT) {
                        app.agent_panel_open = !app.agent_panel_open;
                        render_state.write().agent_panel_open = app.agent_panel_open;
                        notify.notify_one();
                        continue;
                    }
                    // \p — prompt library (Alt+p)
                    if key.code == KeyCode::Char('p') && key.modifiers.contains(KeyModifiers::ALT) {
                        app.prompt_library_open = !app.prompt_library_open;
                        {
                            let mut state = render_state.write();
                            state.prompt_library_state.open = app.prompt_library_open;
                        }
                        notify.notify_one();
                        continue;
                    }
                    // ── Phase 10 global keybinds ─────────────────────────────
                    // Alt+e — toggle file tree sidebar; focuses it on open
                    if key.code == KeyCode::Char('e') && key.modifiers.contains(KeyModifiers::ALT) {
                        app.file_tree_open = !app.file_tree_open;
                        // Focus the tree when opening, unfocus when closing
                        app.file_tree_focused = app.file_tree_open;
                        if app.file_tree_open { app.terminal_mode = false; }
                        {
                            let mut state = render_state.write();
                            state.file_tree_open = app.file_tree_open;
                            state.file_tree_focused = app.file_tree_focused;
                            if app.file_tree_open {
                                state.file_tree_state = ui::widgets::file_tree::FileTreeState::new(&app.workspace_root);
                                state.mode = "TREE".to_string();
                            } else {
                                state.mode = "NORMAL".to_string();
                            }
                        }
                        notify.notify_one();
                        continue;
                    }
                    // Alt+m — toggle minimap
                    if key.code == KeyCode::Char('m') && key.modifiers.contains(KeyModifiers::ALT) {
                        app.minimap_open = !app.minimap_open;
                        {
                            let mut state = render_state.write();
                            state.minimap_open = app.minimap_open;
                        }
                        notify.notify_one();
                        continue;
                    }
                    // F5 — launch/toggle DAP panel
                    if key.code == KeyCode::F(5) {
                        app.dap_panel_open = !app.dap_panel_open;
                        {
                            let mut state = render_state.write();
                            state.dap_panel.open = app.dap_panel_open;
                        }
                        if app.dap_panel_open {
                            // Attempt launch
                            let program = {
                                let maybe_buf_id = app.active_editor.as_ref().map(|ae| ae.buffer_id);
                                maybe_buf_id
                                    .and_then(|bid| app.buffers.get(bid))
                                    .and_then(|b| {
                                        let r = b.read();
                                        r.path.as_ref().map(|p| p.to_string_lossy().to_string())
                                    })
                                    .unwrap_or_else(|| "./target/debug/rmtide".to_string())
                            };
                            let client = app.dap_client.clone();
                            let cwd = app.workspace_root.to_string_lossy().to_string();
                            tokio::spawn(async move {
                                let _ = client.launch(&program, Vec::new(), &cwd).await;
                            });
                        } else {
                            app.dap_client.terminate();
                        }
                        notify.notify_one();
                        continue;
                    }
                    // Alt+b — open bookmark picker
                    if key.code == KeyCode::Char('b') && key.modifiers.contains(KeyModifiers::ALT) {
                        app.bookmark_picker_open = !app.bookmark_picker_open;
                        {
                            let mut state = render_state.write();
                            state.bookmark_picker.open = app.bookmark_picker_open;
                            if app.bookmark_picker_open {
                                state.bookmark_picker.bookmarks = app.bookmarks.list();
                            }
                        }
                        notify.notify_one();
                        continue;
                    }
                    // Alt+c — open clipboard picker
                    if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::ALT) {
                        app.clipboard_picker_open = !app.clipboard_picker_open;
                        {
                            let mut state = render_state.write();
                            state.clipboard_picker.open = app.clipboard_picker_open;
                            if app.clipboard_picker_open {
                                state.clipboard_picker.entries = app.clipboard_ring.list();
                            }
                        }
                        notify.notify_one();
                        continue;
                    }
                    // Alt+q — toggle macro panel
                    if key.code == KeyCode::Char('q') && key.modifiers.contains(KeyModifiers::ALT) {
                        app.macro_panel_open = !app.macro_panel_open;
                        {
                            let mut state = render_state.write();
                            state.macro_panel.open = app.macro_panel_open;
                        }
                        notify.notify_one();
                        continue;
                    }

                    // ── Phase 11 global keybinds ─────────────────────────────
                    // Alt+T — toggle task runner
                    if key.code == KeyCode::Char('T') && key.modifiers.contains(KeyModifiers::ALT) {
                        app.task_runner_open = !app.task_runner_open;
                        {
                            let mut state = render_state.write();
                            state.task_runner_open = app.task_runner_open;
                            if app.task_runner_open {
                                state.task_records = app.task_runner.get_records();
                            }
                        }
                        notify.notify_one();
                        continue;
                    }
                    // Alt+L — toggle log viewer
                    if key.code == KeyCode::Char('L') && key.modifiers.contains(KeyModifiers::ALT) {
                        app.log_viewer_open = !app.log_viewer_open;
                        render_state.write().log_viewer_open = app.log_viewer_open;
                        notify.notify_one();
                        continue;
                    }
                    // Alt+P — toggle process panel
                    if key.code == KeyCode::Char('P') && key.modifiers.contains(KeyModifiers::ALT) {
                        app.process_panel_open = !app.process_panel_open;
                        {
                            let mut state = render_state.write();
                            state.process_panel_open = app.process_panel_open;
                            if app.process_panel_open {
                                state.processes = app.process_mgr.list();
                            }
                        }
                        notify.notify_one();
                        continue;
                    }

                    // \R — code review via agent (Alt+R)
                    if key.code == KeyCode::Char('R') && key.modifiers.contains(KeyModifiers::ALT) {
                        let selection = app.active_editor.as_ref()
                            .and_then(|ae| {
                                ae.view.visual_range().map(|(s, e)| {
                                    if let Some(buf_arc) = app.buffers.get(ae.buffer_id) {
                                        let buf = buf_arc.read();
                                        let lines: Vec<String> = (s.line..=e.line.min(buf.line_count().saturating_sub(1)))
                                            .map(|l| buf.line_content(l))
                                            .collect();
                                        lines.join("\n")
                                    } else {
                                        String::new()
                                    }
                                })
                            })
                            .unwrap_or_else(|| "the current file".to_string());
                        let preview: String = selection.chars().take(100).collect();
                        let task = format!("Code review: {}", preview);
                        let session = ai::agent::AgentSession::new(&task);
                        let session_arc = Arc::new(tokio::sync::Mutex::new(session));
                        let (update_tx, update_rx) = tokio::sync::mpsc::unbounded_channel();
                        ai::spawn_agent_loop(
                            session_arc.clone(),
                            Arc::new(app.ai.clone_registry()),
                            app.approval_queue.clone(),
                            app.spend.clone(),
                            app.workspace_root.clone(),
                            update_tx,
                        );
                        app.agent_session = Some(session_arc);
                        app.agent_update_rx = Some(update_rx);
                        app.agent_panel_open = true;
                        render_state.write().agent_panel_open = true;
                        notify.notify_one();
                        continue;
                    }

                    // ── Phase 12 global keybinds ─────────────────────────────

                    // Ctrl+P — toggle command palette
                    if key.code == KeyCode::Char('p') && key.modifiers.contains(KeyModifiers::CONTROL) {
                        app.command_palette_open = !app.command_palette_open;
                        {
                            let mut state = render_state.write();
                            state.command_palette.open = app.command_palette_open;
                            if app.command_palette_open {
                                state.command_palette.query.clear();
                                state.command_palette.filter();
                            }
                        }
                        notify.notify_one();
                        continue;
                    }

                    // Command palette key interception when open
                    if app.command_palette_open {
                        match key.code {
                            KeyCode::Escape => {
                                app.command_palette_open = false;
                                render_state.write().command_palette.open = false;
                                notify.notify_one();
                                continue;
                            }
                            KeyCode::Up => {
                                render_state.write().command_palette.move_up();
                                notify.notify_one();
                                continue;
                            }
                            KeyCode::Down => {
                                render_state.write().command_palette.move_down();
                                notify.notify_one();
                                continue;
                            }
                            KeyCode::Enter => {
                                let cmd_id = render_state.read().command_palette.execute_id();
                                if let Some(id) = cmd_id {
                                    let mut state = render_state.write();
                                    state.command_palette.record_recent(&id);
                                    state.command_palette.open = false;
                                    drop(state);
                                    app.command_palette_open = false;
                                    // Dispatch command by ID
                                    handle_palette_command(&id, &mut app, &render_state, &notify, &bus).await;
                                }
                                notify.notify_one();
                                continue;
                            }
                            KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) && !key.modifiers.contains(KeyModifiers::ALT) => {
                                {
                                    let mut state = render_state.write();
                                    state.command_palette.query.push(c);
                                    state.command_palette.selected = 0;
                                    state.command_palette.filter();
                                }
                                notify.notify_one();
                                continue;
                            }
                            KeyCode::Backspace => {
                                {
                                    let mut state = render_state.write();
                                    state.command_palette.query.pop();
                                    state.command_palette.selected = 0;
                                    state.command_palette.filter();
                                }
                                notify.notify_one();
                                continue;
                            }
                            _ => { continue; }
                        }
                    }

                    // Alt+f — semantic search
                    if key.code == KeyCode::Char('f') && key.modifiers.contains(KeyModifiers::ALT) {
                        app.semantic_search_open = !app.semantic_search_open;
                        render_state.write().semantic_search.open = app.semantic_search_open;
                        notify.notify_one();
                        continue;
                    }

                    // Alt+Shift+P — AI pair programmer (uppercase 'P' with alt = shift+alt+p)
                    if key.code == KeyCode::Char('p') && key.modifiers.contains(KeyModifiers::ALT) && key.modifiers.contains(KeyModifiers::SHIFT) {
                        app.pair_programmer_active = !app.pair_programmer_active;
                        render_state.write().pair_programmer.active = app.pair_programmer_active;
                        notify.notify_one();
                        continue;
                    }
                }

                // ── File tree keyboard navigation ─────────────────────────
                if app.file_tree_focused && app.file_tree_open {
                    use rmcore::event::KeyCode;
                    match key.code {
                        KeyCode::Char('j') | KeyCode::Down => {
                            render_state.write().file_tree_state.move_down();
                            notify.notify_one();
                            continue;
                        }
                        KeyCode::Char('k') | KeyCode::Up => {
                            render_state.write().file_tree_state.move_up();
                            notify.notify_one();
                            continue;
                        }
                        KeyCode::Enter | KeyCode::Char('l') | KeyCode::Right => {
                            let action = {
                                let state = render_state.read();
                                state.file_tree_state.selected_path()
                                    .map(|p| (p.to_path_buf(), p.is_dir()))
                            };
                            if let Some((path, is_dir)) = action {
                                if is_dir {
                                    render_state.write().file_tree_state.toggle_expand();
                                } else {
                                    // Open the file and return focus to editor
                                    if let Ok(buf_id) = app.open_file(&path) {
                                        let tab_name = path.file_name()
                                            .map(|n| n.to_string_lossy().into_owned())
                                            .unwrap_or_else(|| path.to_string_lossy().into_owned());
                                        let ed_display = app.make_editor_display();
                                        let mut state = render_state.write();
                                        state.tab_bar.add_tab(buf_id.0 as usize, &tab_name);
                                        state.editor_display = ed_display;
                                        state.file_tree_focused = false;
                                        state.mode = "NORMAL".to_string();
                                    }
                                    app.file_tree_focused = false;
                                    app.terminal_mode = false;
                                }
                            }
                            notify.notify_one();
                            continue;
                        }
                        KeyCode::Char('h') | KeyCode::Left => {
                            // Collapse selected dir (toggle_expand collapses if already expanded)
                            render_state.write().file_tree_state.toggle_expand();
                            notify.notify_one();
                            continue;
                        }
                        KeyCode::Escape | KeyCode::Char('q') => {
                            app.file_tree_focused = false;
                            {
                                let mut state = render_state.write();
                                state.file_tree_focused = false;
                                state.mode = "NORMAL".to_string();
                            }
                            notify.notify_one();
                            continue;
                        }
                        _ => { continue; }
                    }
                }

                // Route key through modal engine
                let cmds = if let Some(ae) = &mut app.active_editor {
                    ae.modal.handle_key(&key)
                } else {
                    // No active editor — handle global keys
                    handle_global_key(&key, &mut app, &bus)
                };

                let mut needs_render = false;

                for cmd in cmds {
                    match &cmd {
                        // ── File picker ──────────────────────────────────────
                        EditorCommand::OpenFilePicker => {
                            let root = app.workspace_root.clone();
                            let fp = ui::widgets::file_picker::FilePickerState::new(&root);
                            let mut state = render_state.write();
                            state.file_picker = Some(fp);
                            app.file_picker_open = true;
                            needs_render = true;
                        }
                        EditorCommand::FilePickerUp => {
                            let mut state = render_state.write();
                            if let Some(fp) = &mut state.file_picker {
                                fp.move_up();
                            }
                            needs_render = true;
                        }
                        EditorCommand::FilePickerDown => {
                            let mut state = render_state.write();
                            if let Some(fp) = &mut state.file_picker {
                                fp.move_down();
                            }
                            needs_render = true;
                        }
                        EditorCommand::FilePickerInput(c) => {
                            let mut state = render_state.write();
                            if let Some(fp) = &mut state.file_picker {
                                fp.push_char(*c);
                            }
                            needs_render = true;
                        }
                        EditorCommand::FilePickerBackspace => {
                            let mut state = render_state.write();
                            if let Some(fp) = &mut state.file_picker {
                                fp.pop_char();
                            }
                            needs_render = true;
                        }
                        EditorCommand::FilePickerConfirm => {
                            let path = {
                                let state = render_state.read();
                                state.file_picker.as_ref()
                                    .and_then(|fp| fp.selected_path().cloned())
                            };
                            if let Some(path) = path {
                                if let Ok(buf_id) = app.open_file(&path) {
                                    let tab_name = path.file_name()
                                        .map(|n| n.to_string_lossy().into_owned())
                                        .unwrap_or_else(|| path.to_string_lossy().into_owned());
                                    render_state.write().tab_bar.add_tab(buf_id.0 as usize, &tab_name);
                                }
                            }
                            let mut state = render_state.write();
                            state.file_picker = None;
                            app.file_picker_open = false;
                            needs_render = true;
                        }
                        EditorCommand::FilePickerCancel => {
                            let mut state = render_state.write();
                            state.file_picker = None;
                            app.file_picker_open = false;
                            needs_render = true;
                        }

                        // ── Command line ──────────────────────────────────────
                        EditorCommand::EnterCommandLine => {
                            cmdline_state = CmdLineState::new();
                            let mut state = render_state.write();
                            state.cmdline_is_active = true;
                            state.cmdline = Some(cmdline_state.clone());
                            needs_render = true;
                        }
                        EditorCommand::CmdInput(c) => {
                            cmdline_state.push(*c);
                            render_state.write().cmdline = Some(cmdline_state.clone());
                            needs_render = true;
                        }
                        EditorCommand::CmdBackspace => {
                            cmdline_state.backspace();
                            render_state.write().cmdline = Some(cmdline_state.clone());
                            needs_render = true;
                        }
                        EditorCommand::CmdHistoryUp => {
                            cmdline_state.history_up();
                            render_state.write().cmdline = Some(cmdline_state.clone());
                            needs_render = true;
                        }
                        EditorCommand::CmdHistoryDown => {
                            cmdline_state.history_down();
                            render_state.write().cmdline = Some(cmdline_state.clone());
                            needs_render = true;
                        }
                        EditorCommand::CmdConfirm => {
                            let cmd_str = cmdline_state.confirm();
                            {
                                let mut state = render_state.write();
                                state.cmdline_is_active = false;
                                state.cmdline = None;
                                state.mode = "NORMAL".to_string();
                            }
                            // LSP rename intercept — if pending, treat input as new name
                            if app.lsp_rename_pending {
                                app.lsp_rename_pending = false;
                                if !cmd_str.trim().is_empty() {
                                    let new_name = cmd_str.trim().to_string();
                                    let applied = app.lsp_rename(&new_name).await;
                                    let msg = if applied { format!("[LSP] Renamed to '{new_name}'") } else { "[LSP] Rename failed".to_string() };
                                    render_state.write().ai_status = msg;
                                    needs_render = true;
                                }
                                continue;
                            }
                            if let Some(action) = app.process_cmdline(&cmd_str) {
                                match action {
                                    AppAction::Quit | AppAction::ForceQuit => {
                                        let _ = bus.sender().send(AppEvent::Quit);
                                    }
                                    AppAction::SplitH => {
                                        let new_pane_id = app.ids.next_pane();
                                        let new_pane = ui::pane::Pane::new(new_pane_id, ui::pane::PaneKind::Editor);
                                        let mut state = render_state.write();
                                        let focused = state.layout.focused;
                                        state.layout.split(focused, ui::layout::SplitDir::Horizontal, new_pane);
                                    }
                                    AppAction::SplitV => {
                                        let new_pane_id = app.ids.next_pane();
                                        let new_pane = ui::pane::Pane::new(new_pane_id, ui::pane::PaneKind::Editor);
                                        let mut state = render_state.write();
                                        let focused = state.layout.focused;
                                        state.layout.split(focused, ui::layout::SplitDir::Vertical, new_pane);
                                    }
                                    AppAction::OpenFilePicker => {
                                        app.file_picker_open = true;
                                    }
                                    AppAction::SetTheme(name) => {
                                        let new_theme = theme::load_theme(&name);
                                        let mut state = render_state.write();
                                        state.theme_name = name;
                                        state.theme = Some(new_theme);
                                    }
                                    AppAction::InfoMessage(_msg) => {
                                        // Info messages displayed via ai_status for now
                                    }
                                }
                            }
                            // If a file was opened, switch out of terminal mode.
                            if app.active_editor.is_some() && app.terminal_mode {
                                app.terminal_mode = false;
                                render_state.write().mode = "NORMAL".to_string();
                            }
                            // Phase 9 — sync open/close state to RenderState
                            {
                                let mut state = render_state.write();
                                state.agent_panel_open = app.agent_panel_open;
                                state.agent_memory_open = app.agent_memory_open;
                                state.context_picker_state.open = app.context_picker_open;
                                state.prompt_library_state.open = app.prompt_library_open;
                            }
                            // Phase 11 — sync open/close state to RenderState
                            {
                                let mut state = render_state.write();
                                state.task_runner_open = app.task_runner_open;
                                state.log_viewer_open = app.log_viewer_open;
                                state.diff_review_state.open = app.diff_review_open;
                                state.process_panel_open = app.process_panel_open;
                                state.port_panel_open = app.port_panel_open;
                                state.deploy_panel_open = app.deploy_panel_open;
                                state.env_panel_open = app.env_panel_open;
                                state.http_panel_open = app.http_panel_open;
                                state.db_panel_open = app.db_panel_open;
                                if app.task_runner_open {
                                    state.task_records = app.task_runner.get_records();
                                }
                                if app.process_panel_open {
                                    state.processes = app.process_mgr.list();
                                }
                            }
                            // Phase 10 — sync open/close state to RenderState
                            {
                                let mut state = render_state.write();
                                state.file_tree_open = app.file_tree_open;
                                state.minimap_open = app.minimap_open;
                                state.dap_panel.open = app.dap_panel_open;
                                state.bookmark_picker.open = app.bookmark_picker_open;
                                state.macro_panel.open = app.macro_panel_open;
                                state.clipboard_picker.open = app.clipboard_picker_open;
                                state.session_picker.open = app.session_picker_open;
                                state.find_replace.open = app.find_replace.open;
                                state.symbol_browser.open = app.symbol_browser.open;
                            }
                            // Phase 10 — handle cmdline commands
                            {
                                let p10_cmd = cmd_str.trim_start_matches(':').trim();
                                match p10_cmd {
                                    "FindReplace" | "Rg" => {
                                        app.find_replace.open = true;
                                        render_state.write().find_replace.open = true;
                                    }
                                    "Symbols" => {
                                        app.symbol_browser.open = true;
                                        app.symbol_browser.filter();
                                        {
                                            let mut state = render_state.write();
                                            state.symbol_browser.open = true;
                                            state.symbol_browser.symbols = app.symbol_browser.symbols.clone();
                                            state.symbol_browser.query = app.symbol_browser.query.clone();
                                            state.symbol_browser.filter();
                                        }
                                    }
                                    "DAP" => {
                                        app.dap_panel_open = !app.dap_panel_open;
                                        render_state.write().dap_panel.open = app.dap_panel_open;
                                    }
                                    "Minimap" => {
                                        app.minimap_open = !app.minimap_open;
                                        render_state.write().minimap_open = app.minimap_open;
                                    }
                                    "Bookmarks" => {
                                        app.bookmark_picker_open = !app.bookmark_picker_open;
                                        let bms = app.bookmarks.list();
                                        let mut state = render_state.write();
                                        state.bookmark_picker.open = app.bookmark_picker_open;
                                        state.bookmark_picker.bookmarks = bms;
                                    }
                                    "Macros" => {
                                        app.macro_panel_open = !app.macro_panel_open;
                                        render_state.write().macro_panel.open = app.macro_panel_open;
                                    }
                                    "Clipboard" => {
                                        app.clipboard_picker_open = !app.clipboard_picker_open;
                                        let entries = app.clipboard_ring.list();
                                        let mut state = render_state.write();
                                        state.clipboard_picker.open = app.clipboard_picker_open;
                                        state.clipboard_picker.entries = entries;
                                    }
                                    "Sessions" | "SessionLoad" => {
                                        app.session_picker_open = !app.session_picker_open;
                                        let sessions = app.session_manager.list();
                                        let mut state = render_state.write();
                                        state.session_picker.open = app.session_picker_open;
                                        state.session_picker.sessions = sessions;
                                        state.session_picker.mode = ui::widgets::session_manager::SessionMode::Browse;
                                    }
                                    "SessionSave" => {
                                        app.session_picker_open = true;
                                        let mut state = render_state.write();
                                        state.session_picker.open = true;
                                        state.session_picker.mode = ui::widgets::session_manager::SessionMode::SaveAs;
                                    }
                                    // ── Terminal commands ─────────────────────
                                    "term" | "terminal" => {
                                        let new_session_id = app.ids.next_session();
                                        let pty_cfg = terminal::pty::PtyConfig::default();
                                        let itx = input_tx.clone();
                                        let rs = render_state.clone();
                                        let n = notify.clone();
                                        tokio::spawn(async move {
                                            if let Ok(sess_arc) = terminal::session::PtySession::spawn(new_session_id, pty_cfg, itx).await {
                                                rs.write().terminal_session = Some(sess_arc);
                                                n.notify_one();
                                            }
                                        });
                                        app.terminal_mode = true;
                                        render_state.write().mode = "TERMINAL".to_string();
                                    }
                                    // ── Phase 11 cmdline commands ─────────────
                                    "Tasks" | "tr" => {
                                        app.task_runner_open = !app.task_runner_open;
                                        let records = app.task_runner.get_records();
                                        let mut state = render_state.write();
                                        state.task_runner_open = app.task_runner_open;
                                        state.task_records = records;
                                    }
                                    "Logs" => {
                                        app.log_viewer_open = !app.log_viewer_open;
                                        render_state.write().log_viewer_open = app.log_viewer_open;
                                    }
                                    "LiveServer" => {
                                        let server = app.live_server.clone();
                                        let rs = render_state.clone();
                                        let n = notify.clone();
                                        tokio::spawn(async move {
                                            let _ = server.start().await;
                                            let url = server.get_url();
                                            let mut state = rs.write();
                                            state.live_server_url = url;
                                            n.notify_one();
                                        });
                                    }
                                    "DiffReview" => {
                                        app.diff_review_open = !app.diff_review_open;
                                        render_state.write().diff_review_state.open = app.diff_review_open;
                                    }
                                    "Procs" => {
                                        app.process_panel_open = !app.process_panel_open;
                                        let procs = app.process_mgr.list();
                                        let mut state = render_state.write();
                                        state.process_panel_open = app.process_panel_open;
                                        state.processes = procs;
                                    }
                                    "Ports" => {
                                        app.port_panel_open = !app.port_panel_open;
                                        let mut state = render_state.write();
                                        state.port_panel_open = app.port_panel_open;
                                        if app.port_panel_open {
                                            state.port_panel_state.refresh();
                                        }
                                    }
                                    "Deploy" => {
                                        app.deploy_panel_open = !app.deploy_panel_open;
                                        render_state.write().deploy_panel_open = app.deploy_panel_open;
                                    }
                                    "Env" => {
                                        app.env_panel_open = !app.env_panel_open;
                                        render_state.write().env_panel_open = app.env_panel_open;
                                        if app.env_panel_open {
                                            app.env_mgr.load();
                                        }
                                    }
                                    "Http" | "HTTP" => {
                                        app.http_panel_open = !app.http_panel_open;
                                        render_state.write().http_panel_open = app.http_panel_open;
                                    }
                                    "DB" | "Db" => {
                                        app.db_panel_open = !app.db_panel_open;
                                        render_state.write().db_panel_open = app.db_panel_open;
                                    }
                                    // ── Phase 12 cmdline commands ─────────────
                                    "SemanticSearch" => {
                                        app.semantic_search_open = !app.semantic_search_open;
                                        render_state.write().semantic_search.open = app.semantic_search_open;
                                    }
                                    "SecScan" => {
                                        app.security_panel_open = !app.security_panel_open;
                                        {
                                            let mut state = render_state.write();
                                            state.security_panel.open = app.security_panel_open;
                                            if app.security_panel_open {
                                                state.security_panel.scanning = false;
                                                state.security_panel.scan_log.push(
                                                    "Security scan started...".into()
                                                );
                                            }
                                        }
                                    }
                                    "Analytics" => {
                                        app.analytics_panel_open = !app.analytics_panel_open;
                                        {
                                            let mut state = render_state.write();
                                            state.analytics_panel.open = app.analytics_panel_open;
                                            if app.analytics_panel_open {
                                                state.analytics_panel.loading = true;
                                                // Compute lang stats from workspace
                                                let lang_stats = ui::widgets::analytics_panel::AnalyticsPanelState::compute_lang_stats(&app.workspace_root);
                                                state.analytics_panel.lang_stats = lang_stats;
                                                state.analytics_panel.loading = false;
                                            }
                                        }
                                    }
                                    "Notebook" => {
                                        app.notebook_open = !app.notebook_open;
                                        render_state.write().notebook_state.open = app.notebook_open;
                                    }
                                    "Keymaps" => {
                                        app.keymap_editor_open = !app.keymap_editor_open;
                                        render_state.write().keymap_editor.open = app.keymap_editor_open;
                                    }
                                    "PluginBrowse" => {
                                        app.plugin_marketplace_open = !app.plugin_marketplace_open;
                                        render_state.write().plugin_marketplace.open = app.plugin_marketplace_open;
                                    }
                                    "Collab" | "CollabShare" => {
                                        app.collab_open = !app.collab_open;
                                        render_state.write().collab_state.open = app.collab_open;
                                    }
                                    "PairProg" => {
                                        app.pair_programmer_active = !app.pair_programmer_active;
                                        render_state.write().pair_programmer.active = app.pair_programmer_active;
                                    }
                                    "Palette" | "CommandPalette" => {
                                        app.command_palette_open = !app.command_palette_open;
                                        {
                                            let mut state = render_state.write();
                                            state.command_palette.open = app.command_palette_open;
                                            if app.command_palette_open {
                                                state.command_palette.query.clear();
                                                state.command_palette.filter();
                                            }
                                        }
                                    }
                                    _ => {}
                                }
                            }
                            // Phase 12 — sync open/close state to RenderState
                            {
                                let mut state = render_state.write();
                                state.semantic_search.open = app.semantic_search_open;
                                state.commit_composer.open = app.commit_composer_open;
                                state.security_panel.open = app.security_panel_open;
                                state.analytics_panel.open = app.analytics_panel_open;
                                state.notebook_state.open = app.notebook_open;
                                state.keymap_editor.open = app.keymap_editor_open;
                                state.plugin_marketplace.open = app.plugin_marketplace_open;
                                state.collab_state.open = app.collab_open;
                                state.pair_programmer.active = app.pair_programmer_active;
                                state.command_palette.open = app.command_palette_open;
                            }
                            needs_render = true;
                        }
                        EditorCommand::CmdCancel => {
                            cmdline_state.cancel();
                            let mut state = render_state.write();
                            state.cmdline_is_active = false;
                            state.cmdline = None;
                            state.mode = "NORMAL".to_string();
                            needs_render = true;
                        }

                        // ── Search ────────────────────────────────────────────
                        EditorCommand::EnterSearch(dir) => {
                            search_buf.clear();
                            search_forward = *dir == SearchDir::Forward;
                            let mut state = render_state.write();
                            state.search_is_active = true;
                            needs_render = true;
                        }
                        EditorCommand::SearchInput(c) => {
                            search_buf.push(*c);
                            if let Some(ae) = &mut app.active_editor {
                                ae.search.set_pattern(
                                    search_buf.clone(),
                                    if search_forward {
                                        editor::search::SearchDir::Forward
                                    } else {
                                        editor::search::SearchDir::Backward
                                    },
                                );
                                if let Some(buf_arc) = app.buffers.get(ae.buffer_id) {
                                    let buf = buf_arc.read();
                                    let total = buf.line_count();
                                    let lines_vec: Vec<String> =
                                        (0..total).map(|i| buf.line_content(i)).collect();
                                    let lines_ref: Vec<&str> =
                                        lines_vec.iter().map(|s| s.as_str()).collect();
                                    ae.search.find_all(&lines_ref);
                                }
                            }
                            let mut state = render_state.write();
                            state.search_query = search_buf.clone();
                            needs_render = true;
                        }
                        EditorCommand::SearchBackspace => {
                            search_buf.pop();
                            let mut state = render_state.write();
                            state.search_query = search_buf.clone();
                            needs_render = true;
                        }
                        EditorCommand::SearchConfirm => {
                            let mut state = render_state.write();
                            state.search_is_active = false;
                            drop(state);
                            // Jump to first match
                            if let Some(ae) = &mut app.active_editor {
                                let cur_line = ae.view.cursor.line;
                                let cur_col = ae.view.cursor.col;
                                if let Some(m) = ae.search.next_match(cur_line, cur_col) {
                                    ae.view.cursor.line = m.line;
                                    ae.view.cursor.col = m.start_col;
                                }
                            }
                            needs_render = true;
                        }
                        EditorCommand::SearchCancel => {
                            search_buf.clear();
                            let mut state = render_state.write();
                            state.search_is_active = false;
                            state.search_query.clear();
                            needs_render = true;
                        }
                        EditorCommand::SearchNext => {
                            if let Some(ae) = &mut app.active_editor {
                                let cur_line = ae.view.cursor.line;
                                let cur_col = ae.view.cursor.col;
                                if let Some(m) = ae.search.next_match(cur_line, cur_col) {
                                    ae.view.cursor.line = m.line;
                                    ae.view.cursor.col = m.start_col;
                                }
                            }
                            needs_render = true;
                        }
                        EditorCommand::SearchPrev => {
                            if let Some(ae) = &mut app.active_editor {
                                let cur_line = ae.view.cursor.line;
                                let cur_col = ae.view.cursor.col;
                                if let Some(m) = ae.search.prev_match(cur_line, cur_col) {
                                    ae.view.cursor.line = m.line;
                                    ae.view.cursor.col = m.start_col;
                                }
                            }
                            needs_render = true;
                        }

                        // ── Save / Quit ───────────────────────────────────────
                        EditorCommand::SaveFile => {
                            if let Some(ae) = &app.active_editor {
                                if let Some(buf_arc) = app.buffers.get(ae.buffer_id) {
                                    let mut buf = buf_arc.write();
                                    if let Some(path) = &buf.path.clone() {
                                        let bytes = buf.text.to_bytes();
                                        if std::fs::write(path, &bytes).is_ok() {
                                            buf.dirty = false;
                                        }
                                    }
                                }
                            }
                            needs_render = true;
                        }
                        EditorCommand::Quit => {
                            let _ = bus.sender().send(AppEvent::Quit);
                        }
                        EditorCommand::ForceQuit => {
                            let _ = bus.sender().send(AppEvent::Quit);
                        }

                        // ── LSP commands ─────────────────────────────────────
                        EditorCommand::LspHover => {
                            let text = app.lsp_request_hover().await;
                            let mut state = render_state.write();
                            state.hover = text;
                            needs_render = true;
                        }
                        EditorCommand::LspComplete => {
                            let items = app.lsp_request_completions().await;
                            if !items.is_empty() {
                                let mut state = render_state.write();
                                use ui::widgets::completion_popup::{CompletionEntry, kind_label};
                                state.completion = Some(ui::render::CompletionDisplay {
                                    entries: items.iter().map(|item| CompletionEntry {
                                        label: item.label.clone(),
                                        kind_label: kind_label(item.kind.map(|k| k.0)).to_string(),
                                        detail: item.detail.clone(),
                                        insert_text: item.insert_text.clone(),
                                        is_snippet: item.insert_text_format == Some(2),
                                    }).collect(),
                                    selected: 0,
                                    cursor_row: 0,
                                    cursor_col: 0,
                                });
                                needs_render = true;
                            }
                        }
                        EditorCommand::LspGotoDef => {
                            let locs = app.lsp_goto_definition().await;
                            if let Some(first) = locs.first() {
                                let path = lsp::types::uri_to_path(&first.uri);
                                let _ = app.open_file(&path);
                                if let Some(ae) = &mut app.active_editor {
                                    ae.view.cursor.line = first.range.start.line as usize;
                                    ae.view.cursor.col = first.range.start.character as usize;
                                }
                                needs_render = true;
                            }
                        }
                        EditorCommand::LspGotoRef => {
                            let locs = app.lsp_goto_references().await;
                            if !locs.is_empty() {
                                use ui::widgets::quickfix::QuickfixEntry;
                                let mut state = render_state.write();
                                state.quickfix = locs.iter().map(|l| {
                                    let path = lsp::types::uri_to_path(&l.uri);
                                    QuickfixEntry {
                                        file: path.to_string_lossy().into_owned(),
                                        line: l.range.start.line as usize + 1,
                                        col: l.range.start.character as usize + 1,
                                        message: String::new(),
                                    }
                                }).collect();
                                needs_render = true;
                            }
                        }
                        EditorCommand::LspFormat => {
                            app.lsp_format().await;
                            app.buffer_version += 1;
                            needs_render = true;
                        }
                        EditorCommand::CompletionSelectNext => {
                            let mut state = render_state.write();
                            if let Some(cd) = &mut state.completion {
                                let len = cd.entries.len();
                                if len > 0 {
                                    cd.selected = (cd.selected + 1) % len;
                                }
                            }
                            needs_render = true;
                        }
                        EditorCommand::CompletionSelectPrev => {
                            let mut state = render_state.write();
                            if let Some(cd) = &mut state.completion {
                                let len = cd.entries.len();
                                if len > 0 {
                                    cd.selected = cd.selected.wrapping_sub(1).min(len - 1);
                                }
                            }
                            needs_render = true;
                        }
                        EditorCommand::CompletionConfirm => {
                            let entry = {
                                let state = render_state.read();
                                state.completion.as_ref().and_then(|cd| {
                                    cd.entries.get(cd.selected).map(|e| e.insert_text.clone().unwrap_or_else(|| e.label.clone()))
                                })
                            };
                            if let Some(text) = entry {
                                for c in text.chars() {
                                    app.apply_command(EditorCommand::InsertChar(c));
                                }
                            }
                            render_state.write().completion = None;
                            app.completion_visible = false;
                            needs_render = true;
                        }
                        EditorCommand::CompletionCancel => {
                            render_state.write().completion = None;
                            app.completion_visible = false;
                            needs_render = true;
                        }
                        EditorCommand::LspRename => {
                            // Enter rename mode: prompt for the new name via command line
                            app.lsp_rename_pending = true;
                            app.lsp_rename_new_name.clear();
                            cmdline_state = CmdLineState::new();
                            {
                                let mut state = render_state.write();
                                state.cmdline_is_active = true;
                                state.cmdline = Some(cmdline_state.clone());
                                state.mode = "RENAME".to_string();
                                state.ai_status = "[LSP] Enter new name, <CR> to confirm".to_string();
                            }
                            needs_render = true;
                        }
                        EditorCommand::LspCodeAction => {
                            let actions = app.lsp_get_code_actions().await;
                            let items: Vec<String> = actions.iter().map(|a| a.title.clone()).collect();
                            app.code_actions = actions;
                            {
                                let mut state = render_state.write();
                                state.quickfix = items;
                                state.ai_status = "[LSP] Code actions — select with j/k, Enter to apply".to_string();
                            }
                            needs_render = true;
                        }
                        EditorCommand::LspDiagNext => {
                            if let Some((line, col)) = app.lsp_diag_jump(true).await {
                                let ed_display = app.make_editor_display();
                                let mut state = render_state.write();
                                state.editor_display = ed_display;
                                state.ai_status = format!("[LSP] Diagnostic at {}:{}", line + 1, col + 1);
                            }
                            needs_render = true;
                        }
                        EditorCommand::LspDiagPrev => {
                            if let Some((line, col)) = app.lsp_diag_jump(false).await {
                                let ed_display = app.make_editor_display();
                                let mut state = render_state.write();
                                state.editor_display = ed_display;
                                state.ai_status = format!("[LSP] Diagnostic at {}:{}", line + 1, col + 1);
                            }
                            needs_render = true;
                        }

                        // ── AI commands (Phase 4) ────────────────────────────
                        EditorCommand::AiChat => {
                            // Open or focus the AI chat pane
                            let display = app.ai.active_display();
                            let session = ai::ChatSession::new(display);
                            render_state.write().chat_session = Some(session);
                            needs_render = true;
                        }
                        EditorCommand::AiExplain
                        | EditorCommand::AiFix
                        | EditorCommand::AiTests
                        | EditorCommand::AiDocstring
                        | EditorCommand::AiRefactor => {
                            // Build context and dispatch to AI backend asynchronously
                            let ctx = app.build_editor_context();
                            let messages = match &cmd {
                                EditorCommand::AiExplain   => ctx.explain_prompt(),
                                EditorCommand::AiFix       => ctx.fix_prompt(),
                                EditorCommand::AiTests     => ctx.tests_prompt(),
                                EditorCommand::AiDocstring => ctx.docstring_prompt(),
                                EditorCommand::AiRefactor  => ctx.refactor_prompt("improve clarity and performance"),
                                _ => unreachable!(),
                            };
                            // Open chat pane and show request
                            let display = app.ai.active_display();
                            let mut session = ai::ChatSession::new(display);
                            let task_name = match &cmd {
                                EditorCommand::AiExplain   => "Explain code",
                                EditorCommand::AiFix       => "Fix diagnostics",
                                EditorCommand::AiTests     => "Generate tests",
                                EditorCommand::AiDocstring => "Generate docstring",
                                EditorCommand::AiRefactor  => "Refactor code",
                                _ => "AI task",
                            };
                            session.push_user_message(task_name.to_string());
                            render_state.write().chat_session = Some(session);

                            // Spawn async streaming task
                            if let Some(backend) = app.ai.get_active() {
                                let rs = render_state.clone();
                                let notify_clone = notify.clone();
                                let opts = ai::CompletionOptions::default();
                                tokio::spawn(async move {
                                    match backend.stream_completion(messages, opts).await {
                                        Err(e) => {
                                            let mut state = rs.write();
                                            if let Some(cs) = &mut state.chat_session {
                                                cs.push_error(&e.to_string());
                                            }
                                            notify_clone.notify_one();
                                        }
                                        Ok(mut stream) => {
                                            use futures::StreamExt;
                                            let mut in_tok = 0u32;
                                            let mut out_tok = 0u32;
                                            while let Some(chunk_result) = stream.next().await {
                                                match chunk_result {
                                                    Ok(chunk) => {
                                                        if let Some(n) = chunk.input_tokens { in_tok = n; }
                                                        if let Some(n) = chunk.output_tokens { out_tok = n; }
                                                        if !chunk.text.is_empty() {
                                                            let mut state = rs.write();
                                                            if let Some(cs) = &mut state.chat_session {
                                                                cs.push_chunk(&chunk.text);
                                                            }
                                                            notify_clone.notify_one();
                                                        }
                                                        if chunk.is_final {
                                                            break;
                                                        }
                                                    }
                                                    Err(e) => {
                                                        let mut state = rs.write();
                                                        if let Some(cs) = &mut state.chat_session {
                                                            cs.push_error(&e.to_string());
                                                        }
                                                        notify_clone.notify_one();
                                                        break;
                                                    }
                                                }
                                            }
                                            let mut state = rs.write();
                                            if let Some(cs) = &mut state.chat_session {
                                                cs.finish_streaming(in_tok, out_tok);
                                            }
                                            notify_clone.notify_one();
                                        }
                                    }
                                });
                            }
                            needs_render = true;
                        }

                        EditorCommand::AiModelPicker => {
                            app.open_model_picker().await;
                            let entries = app.model_picker_entries.clone();
                            let sel = app.model_picker_selected;
                            let mut state = render_state.write();
                            state.model_picker = Some(entries);
                            state.model_picker_selected = sel;
                            needs_render = true;
                        }

                        EditorCommand::AiSend => {
                            // Take text from chat session input and send it
                            let text = {
                                let mut state = render_state.write();
                                state.chat_session.as_mut().and_then(|cs| cs.input_confirm())
                            };
                            if let Some(msg_text) = text {
                                // Get history for multi-turn conversation
                                let history = {
                                    let state = render_state.read();
                                    state.chat_session.as_ref()
                                        .map(|cs| cs.history.clone())
                                        .unwrap_or_default()
                                };
                                {
                                    let mut state = render_state.write();
                                    if let Some(cs) = &mut state.chat_session {
                                        cs.push_user_message(msg_text.clone());
                                    }
                                }
                                if let Some(backend) = app.ai.get_active() {
                                    let rs = render_state.clone();
                                    let notify_clone = notify.clone();
                                    let opts = ai::CompletionOptions::default();
                                    tokio::spawn(async move {
                                        match backend.stream_completion(history, opts).await {
                                            Err(e) => {
                                                let mut state = rs.write();
                                                if let Some(cs) = &mut state.chat_session {
                                                    cs.push_error(&e.to_string());
                                                }
                                                notify_clone.notify_one();
                                            }
                                            Ok(mut stream) => {
                                                use futures::StreamExt;
                                                let mut in_tok = 0u32;
                                                let mut out_tok = 0u32;
                                                while let Some(chunk_result) = stream.next().await {
                                                    match chunk_result {
                                                        Ok(chunk) => {
                                                            if let Some(n) = chunk.input_tokens { in_tok = n; }
                                                            if let Some(n) = chunk.output_tokens { out_tok = n; }
                                                            if !chunk.text.is_empty() {
                                                                let mut state = rs.write();
                                                                if let Some(cs) = &mut state.chat_session {
                                                                    cs.push_chunk(&chunk.text);
                                                                }
                                                                notify_clone.notify_one();
                                                            }
                                                            if chunk.is_final { break; }
                                                        }
                                                        Err(e) => {
                                                            let mut state = rs.write();
                                                            if let Some(cs) = &mut state.chat_session {
                                                                cs.push_error(&e.to_string());
                                                            }
                                                            notify_clone.notify_one();
                                                            break;
                                                        }
                                                    }
                                                }
                                                let mut state = rs.write();
                                                if let Some(cs) = &mut state.chat_session {
                                                    cs.finish_streaming(in_tok, out_tok);
                                                }
                                                notify_clone.notify_one();
                                            }
                                        }
                                    });
                                }
                                needs_render = true;
                            }
                        }

                        EditorCommand::AiChatInput(c) => {
                            let mut state = render_state.write();
                            if let Some(cs) = &mut state.chat_session {
                                cs.input_push(*c);
                            }
                            needs_render = true;
                        }

                        EditorCommand::AiChatBackspace => {
                            let mut state = render_state.write();
                            if let Some(cs) = &mut state.chat_session {
                                cs.input_backspace();
                            }
                            needs_render = true;
                        }

                        EditorCommand::AiGhostAccept => {
                            if let Some(suggestion) = app.ghost.accept_full() {
                                for c in suggestion.chars() {
                                    app.apply_command(EditorCommand::InsertChar(c));
                                }
                                render_state.write().ghost_text = None;
                            }
                            needs_render = true;
                        }

                        EditorCommand::AiGhostAcceptWord => {
                            if let Some(word) = app.ghost.accept_word() {
                                for c in word.chars() {
                                    app.apply_command(EditorCommand::InsertChar(c));
                                }
                                render_state.write().ghost_text = app.ghost.suggestion.clone();
                            }
                            needs_render = true;
                        }

                        EditorCommand::AiGhostDismiss => {
                            app.ghost.dismiss();
                            render_state.write().ghost_text = None;
                            needs_render = true;
                        }

                        // ── Git commands (Phase 4) ───────────────────────────
                        EditorCommand::GitPanel => {
                            app.git_panel_open = !app.git_panel_open;
                            if app.git_panel_open {
                                let status = app.git_refresh_status();
                                let mut state = render_state.write();
                                state.git_panel_open = true;
                                if let Some(s) = status.clone() {
                                    state.git_panel_state.update_status(s.clone());
                                    state.git_panel = Some(s);
                                }
                                state.git_branch_name = app.git_branch();
                            } else {
                                let mut state = render_state.write();
                                state.git_panel_open = false;
                            }
                            needs_render = true;
                        }
                        EditorCommand::GitBlame => {
                            app.git_blame_active = !app.git_blame_active;
                            if app.git_blame_active {
                                let blame = app.git_refresh_blame();
                                let mut state = render_state.write();
                                state.git_blame = Some(blame);
                                state.git_blame_active = true;
                            } else {
                                let mut state = render_state.write();
                                state.git_blame = None;
                                state.git_blame_active = false;
                            }
                            needs_render = true;
                        }
                        EditorCommand::GitBranchPanel => {
                            app.git_branch_panel_open = !app.git_branch_panel_open;
                            if app.git_branch_panel_open {
                                app.git.refresh_branches();
                                let branches = app.git.branches.read().clone();
                                let mut state = render_state.write();
                                state.git_branches = branches;
                                state.git_branch_panel_open = true;
                                state.git_branch_selected = 0;
                            } else {
                                render_state.write().git_branch_panel_open = false;
                            }
                            needs_render = true;
                        }
                        EditorCommand::GitStageFile => {
                            let (status, idx, section) = {
                                let state = render_state.read();
                                (
                                    state.git_panel.clone(),
                                    state.git_panel_state.selected,
                                    state.git_panel_state.section.clone(),
                                )
                            };
                            if let Some(s) = status {
                                app.git_stage_selected(&s, idx, &section);
                                let new_status = app.git_refresh_status();
                                let mut state = render_state.write();
                                if let Some(ns) = new_status {
                                    state.git_panel_state.update_status(ns.clone());
                                    state.git_panel = Some(ns);
                                }
                            }
                            needs_render = true;
                        }
                        EditorCommand::GitUnstageFile => {
                            let (status, idx) = {
                                let state = render_state.read();
                                (state.git_panel.clone(), state.git_panel_state.selected)
                            };
                            if let Some(s) = status {
                                app.git_unstage_selected(&s, idx);
                                let new_status = app.git_refresh_status();
                                let mut state = render_state.write();
                                if let Some(ns) = new_status {
                                    state.git_panel_state.update_status(ns.clone());
                                    state.git_panel = Some(ns);
                                }
                            }
                            needs_render = true;
                        }
                        EditorCommand::GitRefreshStatus => {
                            let status = app.git_refresh_status();
                            let gutter = app.git_refresh_gutter();
                            let branch = app.git_branch();
                            let mut state = render_state.write();
                            state.git_gutter = gutter;
                            state.git_branch_name = branch;
                            if let Some(s) = status {
                                state.git_panel_state.update_status(s.clone());
                                state.git_panel = Some(s);
                            }
                            needs_render = true;
                        }

                        // ── Phase 8 — BYOK + Spend + Approvals ──────────────
                        EditorCommand::KeyVaultOpen => {
                            let keyring_open = !render_state.read().keyring_open;
                            let mut state = render_state.write();
                            state.keyring_open = keyring_open;
                            if keyring_open {
                                state.keyring_state.entries = app.keyring_entries();
                            }
                            needs_render = true;
                        }
                        EditorCommand::SpendPanelOpen => {
                            let spend_panel_open = !render_state.read().spend_panel_open;
                            let mut state = render_state.write();
                            state.spend_panel_open = spend_panel_open;
                            if spend_panel_open {
                                let cost = app.spend.session_cost();
                                let budget = *app.spend.session_budget_usd.read();
                                let fraction = app.spend.session_budget_fraction().unwrap_or(0.0);
                                state.spend_state = ui::widgets::spend_panel::SpendPanelState {
                                    breakdown: app.spend.breakdown_text(),
                                    session_cost: cost,
                                    session_budget: budget,
                                    budget_fraction: fraction,
                                    over_budget: app.spend.session_over_budget(),
                                    warning: app.spend.session_budget_warning(),
                                    ai_status: app.spend.status_string(),
                                };
                            }
                            needs_render = true;
                        }
                        EditorCommand::ModelMatrixOpen => {
                            let matrix_open = !render_state.read().model_matrix_open;
                            let mut state = render_state.write();
                            state.model_matrix_open = matrix_open;
                            app.model_matrix_open = matrix_open;
                            needs_render = true;
                        }
                        EditorCommand::ToggleOffline => {
                            let new_mode = app.toggle_offline();
                            render_state.write().offline_mode = new_mode;
                            needs_render = true;
                        }
                        EditorCommand::ApprovalApprove => {
                            if let Some(responder) = pending_approval_responder.take() {
                                let _ = responder.send(ai::approval::ApprovalDecision::Approve);
                                let mut state = render_state.write();
                                state.approval_modal.record_decision("approved", "action", "agent");
                                state.approval_modal.pending = None;
                            }
                            needs_render = true;
                        }
                        EditorCommand::ApprovalDeny => {
                            if let Some(responder) = pending_approval_responder.take() {
                                let _ = responder.send(ai::approval::ApprovalDecision::Deny);
                                let mut state = render_state.write();
                                state.approval_modal.record_decision("denied", "action", "agent");
                                state.approval_modal.pending = None;
                            }
                            needs_render = true;
                        }
                        EditorCommand::ApprovalApproveAll => {
                            if let Some(responder) = pending_approval_responder.take() {
                                let _ = responder.send(ai::approval::ApprovalDecision::ApproveAll);
                                let mut state = render_state.write();
                                state.approval_modal.approve_all = true;
                                state.approval_modal.record_decision("approve-all", "action", "agent");
                                state.approval_modal.pending = None;
                            }
                            needs_render = true;
                        }
                        EditorCommand::ApprovalDenyAll => {
                            if let Some(responder) = pending_approval_responder.take() {
                                let _ = responder.send(ai::approval::ApprovalDecision::DenyAll);
                                let mut state = render_state.write();
                                state.approval_modal.approve_all = false;
                                state.approval_modal.record_decision("deny-all", "action", "agent");
                                state.approval_modal.pending = None;
                            }
                            needs_render = true;
                        }

                        // ── General editor commands ───────────────────────────
                        _ => {
                            if app.apply_command(cmd.clone()) {
                                needs_render = true;
                            }
                        }
                    }
                }

                if needs_render {
                    // Update render state with editor display
                    let ed_display = app.make_editor_display();
                    let mode_str = app.active_editor.as_ref()
                        .map(|ae| ae.modal.mode_str().to_string())
                        .unwrap_or_else(|| "NORMAL".to_string());
                    let ai_status = app.ai_status_string();
                    let ghost_text = app.ghost.suggestion.clone();
                    let model_picker_sel = app.model_picker_selected;
                    let model_picker_open = app.model_picker_open;
                    let spend_status = app.spend_status();
                    let offline_mode = app.offline_mode;
                    let git_branch = app.git_branch();
                    let mut state = render_state.write();
                    state.editor_display = ed_display;
                    state.mode = mode_str;
                    state.ai_status = ai_status;
                    state.ghost_text = ghost_text;
                    state.model_picker_selected = model_picker_sel;
                    state.spend_status = spend_status;
                    state.offline_mode = offline_mode;
                    state.git_branch_name = git_branch;
                    if !model_picker_open {
                        state.model_picker = None;
                    }
                    // ── Sync MCP bridge with latest editor state ─────────────
                    {
                        let active_file = state.editor_display.as_ref()
                            .and_then(|e| e.file_name.clone());
                        let diags: Vec<mcp::McpDiagnostic> = state.diagnostics_panel.iter().map(|d| {
                            let sev = match d.severity {
                                1 => "error",
                                2 => "warning",
                                3 => "info",
                                _ => "hint",
                            };
                            mcp::McpDiagnostic {
                                file: d.file.clone(),
                                line: d.line as usize,
                                col: d.col as usize,
                                severity: sev.to_string(),
                                message: d.message.clone(),
                            }
                        }).collect();
                        drop(state);
                        let git_branch = app.git_branch();
                        let mut b = mcp_bridge.write();
                        b.git_branch = git_branch;
                        b.active_file = active_file;
                        b.diagnostics = diags;
                    }
                    notify.notify_one();
                }
            }
            Ok(AppEvent::Resize(_size)) => {
                notify.notify_one();
            }
            Ok(_) => {}
            Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                tracing::warn!("Main loop lagged {n} events");
            }
            Err(_) => break,
            } // end match event_result
        } // end tokio::select!
    }

    info!("rmtide exiting");
    Ok(())
}

#[allow(dead_code, unused_variables)]
fn handle_global_key(
    _key: &rmcore::event::KeyEvent,
    _app: &mut AppState,
    _bus: &rmcore::bus::EventBus,
) -> Vec<EditorCommand> {
    Vec::new()
}

/// Handle execution of a command palette command by its ID.
#[allow(dead_code, unused_variables)]
async fn handle_palette_command(
    id: &str,
    app: &mut AppState,
    render_state: &Arc<RwLock<RenderState>>,
    notify: &tokio::sync::Notify,
    bus: &rmcore::bus::EventBus,
) {
    match id {
        "editor.save" => {
            if let Some(ae) = &app.active_editor {
                if let Some(buf_arc) = app.buffers.get(ae.buffer_id) {
                    let mut buf = buf_arc.write();
                    if let Some(path) = &buf.path.clone() {
                        let bytes = buf.text.to_bytes();
                        if std::fs::write(path, &bytes).is_ok() {
                            buf.dirty = false;
                        }
                    }
                }
            }
        }
        "editor.quit" | "editor.force_quit" => {
            let _ = bus.sender().send(rmcore::event::AppEvent::Quit);
        }
        "file.open" | "file.new" => {
            let root = app.workspace_root.clone();
            let fp = ui::widgets::file_picker::FilePickerState::new(&root);
            let mut state = render_state.write();
            state.file_picker = Some(fp);
            app.file_picker_open = true;
        }
        "file.tree" => {
            app.file_tree_open = !app.file_tree_open;
            app.file_tree_focused = app.file_tree_open;
            if app.file_tree_open { app.terminal_mode = false; }
            let mut state = render_state.write();
            state.file_tree_open = app.file_tree_open;
            state.file_tree_focused = app.file_tree_focused;
            if app.file_tree_open {
                state.file_tree_state = ui::widgets::file_tree::FileTreeState::new(&app.workspace_root);
                state.mode = "TREE".to_string();
            } else {
                state.mode = "NORMAL".to_string();
            }
        }
        "file.find_replace" => {
            app.find_replace.open = !app.find_replace.open;
            render_state.write().find_replace.open = app.find_replace.open;
        }
        "file.sessions" => {
            app.session_picker_open = !app.session_picker_open;
            let sessions = app.session_manager.list();
            let mut state = render_state.write();
            state.session_picker.open = app.session_picker_open;
            state.session_picker.sessions = sessions;
        }
        "symbols.browser" => {
            app.symbol_browser.open = !app.symbol_browser.open;
            app.symbol_browser.filter();
            let mut state = render_state.write();
            state.symbol_browser.open = app.symbol_browser.open;
        }
        "symbols.bookmarks" => {
            app.bookmark_picker_open = !app.bookmark_picker_open;
            let bms = app.bookmarks.list();
            let mut state = render_state.write();
            state.bookmark_picker.open = app.bookmark_picker_open;
            state.bookmark_picker.bookmarks = bms;
        }
        "git.panel" => {
            app.git_panel_open = !app.git_panel_open;
            render_state.write().git_panel_open = app.git_panel_open;
        }
        "git.diff_review" => {
            app.diff_review_open = !app.diff_review_open;
            render_state.write().diff_review_state.open = app.diff_review_open;
        }
        "git.commit" => {
            app.commit_composer_open = !app.commit_composer_open;
            render_state.write().commit_composer.open = app.commit_composer_open;
            // If opening the composer, kick off AI commit message generation in background
            if app.commit_composer_open {
                let rs = render_state.clone();
                let n = notify.clone();
                let ai_backend = app.ai.get_active();
                let diff = app.get_staged_diff();
                tokio::spawn(async move {
                    if let (Some(backend), Some(diff_text)) = (ai_backend, diff) {
                        let messages = ai::EditorContext::commit_prompt(&diff_text);
                        let opts = ai::CompletionOptions { max_tokens: Some(256), temperature: Some(0.3), ..Default::default() };
                        if let Ok(result) = backend.complete(messages, opts).await {
                            let subject = result.lines()
                                .find(|l| !l.trim().is_empty())
                                .unwrap_or("Update code")
                                .trim().trim_matches('"').to_string();
                            let mut state = rs.write();
                            state.commit_composer.draft = subject;
                            state.commit_composer.cursor_pos = state.commit_composer.draft.len();
                            drop(state);
                            n.notify_one();
                        }
                    }
                });
            }
        }
        "ai.agent" => {
            app.agent_panel_open = !app.agent_panel_open;
            render_state.write().agent_panel_open = app.agent_panel_open;
        }
        "ai.semantic_search" => {
            app.semantic_search_open = !app.semantic_search_open;
            render_state.write().semantic_search.open = app.semantic_search_open;
        }
        "ai.pair_programmer" => {
            app.pair_programmer_active = !app.pair_programmer_active;
            render_state.write().pair_programmer.active = app.pair_programmer_active;
        }
        "ai.prompt_library" => {
            app.prompt_library_open = !app.prompt_library_open;
            render_state.write().prompt_library_state.open = app.prompt_library_open;
        }
        "ai.context_picker" => {
            app.context_picker_open = !app.context_picker_open;
            render_state.write().context_picker_state.open = app.context_picker_open;
        }
        "tasks.runner" => {
            app.task_runner_open = !app.task_runner_open;
            let records = app.task_runner.get_records();
            let mut state = render_state.write();
            state.task_runner_open = app.task_runner_open;
            state.task_records = records;
        }
        "tasks.logs" => {
            app.log_viewer_open = !app.log_viewer_open;
            render_state.write().log_viewer_open = app.log_viewer_open;
        }
        "tasks.processes" => {
            app.process_panel_open = !app.process_panel_open;
            let procs = app.process_mgr.list();
            let mut state = render_state.write();
            state.process_panel_open = app.process_panel_open;
            state.processes = procs;
        }
        "tasks.ports" => {
            app.port_panel_open = !app.port_panel_open;
            let mut state = render_state.write();
            state.port_panel_open = app.port_panel_open;
            if app.port_panel_open {
                state.port_panel_state.refresh();
            }
        }
        "tasks.debug" => {
            app.dap_panel_open = !app.dap_panel_open;
            render_state.write().dap_panel.open = app.dap_panel_open;
        }
        "deploy.panel" => {
            app.deploy_panel_open = !app.deploy_panel_open;
            render_state.write().deploy_panel_open = app.deploy_panel_open;
        }
        "deploy.http" => {
            app.http_panel_open = !app.http_panel_open;
            render_state.write().http_panel_open = app.http_panel_open;
        }
        "deploy.db" => {
            app.db_panel_open = !app.db_panel_open;
            render_state.write().db_panel_open = app.db_panel_open;
        }
        "deploy.env" => {
            app.env_panel_open = !app.env_panel_open;
            render_state.write().env_panel_open = app.env_panel_open;
        }
        "settings.keymaps" => {
            app.keymap_editor_open = !app.keymap_editor_open;
            render_state.write().keymap_editor.open = app.keymap_editor_open;
        }
        "settings.plugins" => {
            app.plugin_marketplace_open = !app.plugin_marketplace_open;
            render_state.write().plugin_marketplace.open = app.plugin_marketplace_open;
        }
        "settings.spend" => {
            let mut state = render_state.write();
            state.spend_panel_open = !state.spend_panel_open;
        }
        "settings.keyring" => {
            let mut state = render_state.write();
            state.keyring_open = !state.keyring_open;
        }
        "intel.security_scan" => {
            app.security_panel_open = !app.security_panel_open;
            render_state.write().security_panel.open = app.security_panel_open;
        }
        "intel.analytics" => {
            app.analytics_panel_open = !app.analytics_panel_open;
            {
                let mut state = render_state.write();
                state.analytics_panel.open = app.analytics_panel_open;
                if app.analytics_panel_open {
                    let lang_stats = ui::widgets::analytics_panel::AnalyticsPanelState::compute_lang_stats(&app.workspace_root);
                    state.analytics_panel.lang_stats = lang_stats;
                }
            }
        }
        "intel.notebook" => {
            app.notebook_open = !app.notebook_open;
            render_state.write().notebook_state.open = app.notebook_open;
        }
        "intel.collab" => {
            app.collab_open = !app.collab_open;
            render_state.write().collab_state.open = app.collab_open;
        }
        "intel.minimap" => {
            app.minimap_open = !app.minimap_open;
            render_state.write().minimap_open = app.minimap_open;
        }
        "intel.clipboard" => {
            app.clipboard_picker_open = !app.clipboard_picker_open;
            let entries = app.clipboard_ring.list();
            let mut state = render_state.write();
            state.clipboard_picker.open = app.clipboard_picker_open;
            state.clipboard_picker.entries = entries;
        }
        "intel.macros" => {
            app.macro_panel_open = !app.macro_panel_open;
            render_state.write().macro_panel.open = app.macro_panel_open;
        }
        _ => {
            // Unknown command — no-op
        }
    }
    notify.notify_one();
}

/// Process a command sent from the MCP server to the main editor loop.
#[allow(dead_code, unused_variables)]
async fn handle_mcp_command(
    cmd: mcp::McpEditorCommand,
    app: &mut AppState,
    render_state: &Arc<RwLock<RenderState>>,
    notify: &tokio::sync::Notify,
) {
    use mcp::McpEditorCommand;
    match cmd {
        McpEditorCommand::OpenFile { path, line } => {
            let p = std::path::PathBuf::from(&path);
            if app.open_file(&p).is_ok() {
                if let Some(line) = line {
                    if let Some(ae) = &mut app.active_editor {
                        ae.view.cursor.line = line.saturating_sub(1);
                        ae.view.cursor.col = 0;
                    }
                }
                let ed_display = app.make_editor_display();
                render_state.write().editor_display = ed_display;
                notify.notify_one();
            }
        }
        McpEditorCommand::ApplyEdit { path, start_line, start_col, end_line, end_col, new_text } => {
            use lsp::types::{WorkspaceEdit, TextEdit, Range, Position};
            let uri = format!("file://{}", path);
            let edit = WorkspaceEdit {
                changes: Some(std::collections::HashMap::from([(
                    uri,
                    vec![TextEdit {
                        range: Range {
                            start: Position { line: start_line as u32, character: start_col as u32 },
                            end: Position { line: end_line as u32, character: end_col as u32 },
                        },
                        new_text,
                    }],
                )])),
            };
            app.apply_workspace_edit(edit);
            let ed_display = app.make_editor_display();
            render_state.write().editor_display = ed_display;
            notify.notify_one();
        }
        McpEditorCommand::RunCommand { command } => {
            // Process as if the user typed the command in command-line mode
            if let Some(_action) = app.process_cmdline(&command) {
                notify.notify_one();
            }
        }
        McpEditorCommand::NewTerminal { cwd, shell } => {
            let new_session_id = app.ids.next_session();
            let mut pty_cfg = terminal::pty::PtyConfig::default();
            if let Some(s) = shell { pty_cfg.shell = s; }
            if let Some(c) = cwd { pty_cfg.working_dir = Some(std::path::PathBuf::from(c)); }
            match terminal::session::PtySession::spawn(new_session_id, pty_cfg, input_tx.clone()).await {
                Ok(sess_arc) => {
                    app.extra_terminal_sessions.push((new_session_id, sess_arc.clone()));
                    app.active_terminal_idx = app.extra_terminal_sessions.len();
                    render_state.write().terminal_session = Some(sess_arc);
                    notify.notify_one();
                }
                Err(e) => tracing::warn!("MCP NewTerminal failed: {e}"),
            }
        }
        McpEditorCommand::SendTerminalInput { pane_id: _, text } => {
            // Route text to the active terminal session
            let target = if app.active_terminal_idx == 0 {
                app.terminal_session.clone()
            } else {
                app.extra_terminal_sessions.get(app.active_terminal_idx - 1).map(|(_, s)| s.clone())
            };
            if let Some(sess_arc) = target {
                sess_arc.lock().write_input(text.into_bytes());
            }
        }
    }
}

/// Convert a KeyEvent to the VT byte sequence the shell expects.
fn key_to_bytes(key: &rmcore::event::KeyEvent) -> Option<Vec<u8>> {
    use rmcore::event::{KeyCode, KeyModifiers};
    match &key.code {
        KeyCode::Char(c) => {
            if key.modifiers.contains(KeyModifiers::CONTROL) {
                let b = (*c as u8).to_ascii_lowercase();
                if b >= b'a' && b <= b'z' {
                    Some(vec![b - b'a' + 1])
                } else if b >= b'@' && b <= b'_' {
                    Some(vec![b - b'@'])
                } else {
                    None
                }
            } else if key.modifiers.contains(KeyModifiers::ALT) {
                let mut v = c.to_string().into_bytes();
                v.insert(0, 0x1b);
                Some(v)
            } else {
                Some(c.to_string().into_bytes())
            }
        }
        KeyCode::Enter     => Some(b"\r".to_vec()),
        KeyCode::Backspace => Some(b"\x7f".to_vec()),
        KeyCode::Tab       => Some(b"\t".to_vec()),
        KeyCode::Escape    => Some(b"\x1b".to_vec()),
        KeyCode::Up        => Some(b"\x1b[A".to_vec()),
        KeyCode::Down      => Some(b"\x1b[B".to_vec()),
        KeyCode::Right     => Some(b"\x1b[C".to_vec()),
        KeyCode::Left      => Some(b"\x1b[D".to_vec()),
        KeyCode::Home      => Some(b"\x1b[H".to_vec()),
        KeyCode::End       => Some(b"\x1b[F".to_vec()),
        KeyCode::PageUp    => Some(b"\x1b[5~".to_vec()),
        KeyCode::PageDown  => Some(b"\x1b[6~".to_vec()),
        KeyCode::Delete    => Some(b"\x1b[3~".to_vec()),
        _ => None,
    }
}
