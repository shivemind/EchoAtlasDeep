#![allow(dead_code, unused_imports, unused_variables)]
//! Main render pipeline.
//! Runs in its own tokio task, wakes on Notify, draws at most once per 16ms.
use std::io::Stdout;
use std::sync::Arc;
use std::time::{Duration, Instant};

use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    event::{EnableMouseCapture, DisableMouseCapture},
};
use parking_lot::RwLock;
use ratatui::{backend::CrosstermBackend, Terminal};
use tokio::sync::Notify;
use tracing::{debug, warn};

use core::event::AppEvent;
use crate::layout::LayoutTree;
use crate::pane::PaneKind;
use crate::widgets::terminal_pane::TerminalPaneWidget;
use crate::widgets::status_bar::StatusBar;
use crate::widgets::border::PaneBorder;
use crate::widgets::editor_pane::EditorPaneWidget;
use crate::widgets::cmdline::CmdLineState;
use crate::widgets::file_picker::FilePickerState;
use crate::widgets::quickfix::QuickfixEntry;
use crate::widgets::completion_popup::CompletionEntry;
use crate::widgets::diagnostics_panel::DiagnosticLine;
use crate::widgets::chat_pane::{ChatDisplay, ChatPaneWidget};
use crate::widgets::model_picker::{ModelEntry, ModelPickerWidget};
use crate::widgets::git_panel::GitPanelState;
use crate::widgets::keyring_panel::KeyringPanelState;
use crate::widgets::spend_panel::SpendPanelState;
use crate::widgets::agent_panel::AgentPanelWidget;
use crate::widgets::tool_trace::ToolTraceWidget;
use crate::widgets::prompt_picker::PromptLibraryWidget;
use crate::widgets::context_picker::ContextPickerWidget;

/// Lightweight snapshot of ChatSession display data for the render thread.
struct ChatSnapshot {
    display_lines: Vec<ai::chat::ChatLine>,
    scroll_offset: usize,
    input: String,
    streaming: bool,
}

const TARGET_FPS: u64 = 60;
const FRAME_BUDGET: Duration = Duration::from_millis(1000 / TARGET_FPS);

/// Snapshot of editor display data for the renderer (avoids holding buffer locks during render).
pub struct EditorDisplay {
    pub lines: Vec<String>,
    pub cursor_line: usize,
    pub cursor_col: usize,
    pub scroll_row: usize,
    pub scroll_col: usize,
    pub is_dirty: bool,
    pub file_name: Option<String>,
    pub selection: Option<((usize, usize), (usize, usize))>,
}

/// LSP completion display state.
pub struct CompletionDisplay {
    pub entries: Vec<CompletionEntry>,
    pub selected: usize,
    pub cursor_row: u16,
    pub cursor_col: u16,
}

/// Shared app state passed to the renderer (read-only snapshot each frame).
pub struct RenderState {
    pub layout: LayoutTree,
    pub mode: String,
    pub backend_name: String,
    // Phase 2 additions:
    pub editor_display: Option<EditorDisplay>,
    pub file_picker: Option<FilePickerState>,
    pub cmdline: Option<CmdLineState>,
    pub quickfix: Vec<QuickfixEntry>,
    pub search_query: String,
    pub search_is_active: bool,
    pub cmdline_is_active: bool,
    // Phase 3 — LSP:
    pub completion: Option<CompletionDisplay>,
    pub hover: Option<String>,
    pub diagnostics_panel: Vec<DiagnosticLine>,
    pub diag_panel_selected: usize,
    pub lsp_errors: usize,
    pub lsp_warnings: usize,
    // Phase 4 — AI:
    pub chat_session: Option<ai::chat::ChatSession>,
    pub ghost_text: Option<String>,
    pub model_picker: Option<Vec<ModelEntry>>,
    pub model_picker_selected: usize,
    pub ai_status: String,
    // Phase 4 (git) — Git integration:
    pub git_panel: Option<git::RepoStatus>,
    pub git_panel_open: bool,
    pub git_panel_state: GitPanelState,
    pub git_gutter: Vec<git::GutterLine>,
    pub git_blame: Option<Vec<git::BlameLine>>,
    pub git_blame_active: bool,
    pub git_branches: Vec<git::BranchInfo>,
    pub git_branch_panel_open: bool,
    pub git_branch_selected: usize,
    pub git_branch_name: String,
    // Phase 7 — Theme
    pub theme: Option<super::theme::Theme>,
    pub theme_name: String,
    // Phase 8 — BYOK + Spend + Approvals:
    pub keyring_open: bool,
    pub keyring_state: KeyringPanelState,
    pub spend_panel_open: bool,
    pub spend_state: SpendPanelState,
    pub approval_modal: ai::approval::ApprovalModalState,
    pub model_matrix_open: bool,
    pub model_matrix_selected: usize,
    pub model_matrix_entries: Vec<ai::spend::ModelPricing>,
    pub offline_mode: bool,
    pub spend_status: String,  // "$0.042" shown in status bar
    // Phase 9 — Agent:
    pub agent_panel_open: bool,
    pub agent_panel_state: crate::widgets::agent_panel::AgentPanelState,
    pub tool_trace_open: bool,
    pub prompt_library_state: crate::widgets::prompt_picker::PromptLibraryState,
    pub context_picker_state: crate::widgets::context_picker::ContextPickerState,
    pub agent_memory_entries: Vec<ai::agent_memory::MemoryEntry>,
    pub agent_memory_open: bool,
    pub agent_status_str: String,
    // Phase 10 — File Tree, Tabs, Find/Replace, DAP, etc:
    pub file_tree_open: bool,
    pub file_tree_state: crate::widgets::file_tree::FileTreeState,
    pub tab_bar: crate::widgets::tab_bar::TabBarState,
    pub find_replace: crate::widgets::find_replace::FindReplaceState,
    pub symbol_browser: crate::widgets::symbol_browser::SymbolBrowserState,
    pub dap_panel: crate::widgets::dap_panel::DapPanelState,
    pub minimap_open: bool,
    pub minimap_state: crate::widgets::minimap::MinimapState,
    pub bookmark_picker: crate::widgets::bookmarks::BookmarkPickerState,
    pub macro_panel: crate::widgets::macro_panel::MacroPanelState,
    pub clipboard_picker: crate::widgets::clipboard_ring::ClipboardPickerState,
    pub session_picker: crate::widgets::session_manager::SessionPickerState,
    // Phase 11 — Task Runner, Logs, Live Server, Diff, Process, Port, Deploy, Env, HTTP, DB:
    pub task_runner_open: bool,
    pub task_runner_state: crate::widgets::task_runner_panel::TaskRunnerState,
    pub task_records: Vec<runner::TaskRecord>,
    pub log_viewer_open: bool,
    pub log_viewer_state: crate::widgets::log_viewer::LogViewerState,
    pub live_server_url: Option<String>,
    pub diff_review_state: crate::widgets::diff_review::DiffReviewState,
    pub process_panel_open: bool,
    pub process_panel_state: crate::widgets::process_panel::ProcessPanelState,
    pub processes: Vec<runner::ManagedProcess>,
    pub port_panel_open: bool,
    pub port_panel_state: crate::widgets::port_panel::PortPanelState,
    pub deploy_panel_open: bool,
    pub deploy_panel_state: crate::widgets::deploy_panel::DeployPanelState,
    pub env_panel_open: bool,
    pub env_panel_state: crate::widgets::env_panel::EnvPanelState,
    pub http_panel_open: bool,
    pub http_panel_state: crate::widgets::http_panel::HttpPanelState,
    pub db_panel_open: bool,
    pub db_panel_state: crate::widgets::db_panel::DbPanelState,
    // Phase 12 — Intelligence, Security & Polish:
    pub semantic_search: crate::widgets::semantic_search::SemanticSearchState,
    pub commit_composer: crate::widgets::commit_composer::CommitComposerState,
    pub security_panel: crate::widgets::security_panel::SecurityPanelState,
    pub analytics_panel: crate::widgets::analytics_panel::AnalyticsPanelState,
    pub notebook_state: crate::widgets::notebook::NotebookState,
    pub keymap_editor: crate::widgets::keymap_editor::KeymapEditorState,
    pub plugin_marketplace: crate::widgets::plugin_marketplace::PluginMarketplaceState,
    pub collab_state: crate::widgets::collab_panel::CollabState,
    pub pair_programmer: crate::widgets::pair_programmer::PairProgrammerState,
    pub command_palette: crate::widgets::command_palette::CommandPaletteState,
    // Terminal emulator session
    pub terminal_session: Option<std::sync::Arc<parking_lot::Mutex<terminal::session::PtySession>>>,
}

/// RAII guard: enables raw mode + alternate screen on construction, restores on drop.
pub struct HostTerminal {
    terminal: Terminal<CrosstermBackend<Stdout>>,
}

impl HostTerminal {
    pub fn enter() -> anyhow::Result<Self> {
        enable_raw_mode()?;
        let mut stdout = std::io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;
        Ok(Self { terminal })
    }

    pub fn draw<F>(&mut self, f: F) -> anyhow::Result<()>
    where
        F: FnOnce(&mut ratatui::Frame),
    {
        self.terminal.draw(f)?;
        Ok(())
    }

    pub fn size(&self) -> anyhow::Result<ratatui::layout::Rect> {
        let size = self.terminal.size()?;
        Ok(ratatui::layout::Rect::new(0, 0, size.width, size.height))
    }
}

impl Drop for HostTerminal {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(
            self.terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture,
        );
        let _ = self.terminal.show_cursor();
    }
}

/// Spawn the render task.
pub fn spawn_render_task(
    notify: Arc<Notify>,
    state: Arc<RwLock<RenderState>>,
    mut event_rx: tokio::sync::broadcast::Receiver<AppEvent>,
) {
    tokio::spawn(async move {
        let mut host = match HostTerminal::enter() {
            Ok(h)  => h,
            Err(e) => { tracing::error!("Failed to enter terminal: {e}"); return; }
        };

        let mut last_frame = Instant::now() - FRAME_BUDGET;

        loop {
            tokio::select! {
                _ = notify.notified() => {}
                _ = tokio::time::sleep(FRAME_BUDGET) => {}
                result = event_rx.recv() => {
                    match result {
                        Ok(AppEvent::Quit) | Err(_) => break,
                        _ => {}
                    }
                }
            }

            let elapsed = last_frame.elapsed();
            if elapsed < FRAME_BUDGET {
                tokio::time::sleep(FRAME_BUDGET - elapsed).await;
            }
            last_frame = Instant::now();

            let state_snap = state.read();
            let area = match host.size() {
                Ok(a)  => a,
                Err(e) => { warn!("Terminal size error: {e}"); continue; }
            };

            let pane_rects = state_snap.layout.rects(area);
            let focused    = state_snap.layout.focused;
            let mode       = state_snap.mode.clone();
            let backend    = state_snap.backend_name.clone();

            // Snapshot terminal session Arc (cheap clone of Arc)
            let terminal_session = state_snap.terminal_session.clone();

            // Snapshot editor display data
            let editor_lines: Option<Vec<String>> = state_snap.editor_display.as_ref().map(|e| e.lines.clone());
            let editor_cursor = state_snap.editor_display.as_ref().map(|e| (e.cursor_line, e.cursor_col));
            let editor_file = state_snap.editor_display.as_ref().and_then(|e| e.file_name.clone());
            let editor_dirty = state_snap.editor_display.as_ref().map(|e| e.is_dirty).unwrap_or(false);
            let editor_scroll = state_snap.editor_display.as_ref().map(|e| (e.scroll_row, e.scroll_col));
            let search_query = state_snap.search_query.clone();
            let cmdline_active = state_snap.cmdline_is_active;
            let search_active = state_snap.search_is_active;
            let ai_status = state_snap.ai_status.clone();
            let model_picker_entries = state_snap.model_picker.as_ref().map(|e| e.clone());
            let model_picker_selected = state_snap.model_picker_selected;
            // Snapshot chat session display data (ChatSession is not Clone)
            let chat_snapshot: Option<ChatSnapshot> = state_snap.chat_session.as_ref().map(|cs| {
                ChatSnapshot {
                    display_lines: cs.display_lines.clone(),
                    scroll_offset: cs.scroll_offset,
                    input: cs.input.clone(),
                    streaming: cs.streaming,
                }
            });
            // Phase 9 — Agent snapshots
            let agent_panel_open = state_snap.agent_panel_open;
            let tool_trace_open = state_snap.tool_trace_open;
            let prompt_library_open = state_snap.prompt_library_state.open;
            let context_picker_open = state_snap.context_picker_state.open;
            let agent_status_str = state_snap.agent_status_str.clone();
            // Phase 11 — open flags snapshot
            let task_runner_open = state_snap.task_runner_open;
            let log_viewer_open = state_snap.log_viewer_open;
            let process_panel_open = state_snap.process_panel_open;
            let port_panel_open = state_snap.port_panel_open;
            let deploy_panel_open = state_snap.deploy_panel_open;
            let env_panel_open = state_snap.env_panel_open;
            let http_panel_open = state_snap.http_panel_open;
            let db_panel_open = state_snap.db_panel_open;
            let diff_review_open = state_snap.diff_review_state.open;
            // Phase 12 — open flags snapshot
            let semantic_search_open = state_snap.semantic_search.open;
            let commit_composer_open = state_snap.commit_composer.open;
            let security_panel_open = state_snap.security_panel.open;
            let analytics_panel_open = state_snap.analytics_panel.open;
            let notebook_open = state_snap.notebook_state.open;
            let keymap_editor_open = state_snap.keymap_editor.open;
            let plugin_marketplace_open = state_snap.plugin_marketplace.open;
            let collab_open = state_snap.collab_state.open;
            let pair_programmer_active = state_snap.pair_programmer.active;
            let command_palette_open = state_snap.command_palette.open;
            // Phase 10 — open flags snapshot
            let file_tree_open = state_snap.file_tree_open;
            let minimap_open = state_snap.minimap_open;
            let find_replace_open = state_snap.find_replace.open;
            let symbol_browser_open = state_snap.symbol_browser.open;
            let dap_panel_open = state_snap.dap_panel.open;
            let bookmark_picker_open = state_snap.bookmark_picker.open;
            let macro_panel_open = state_snap.macro_panel.open;
            let clipboard_picker_open = state_snap.clipboard_picker.open;
            let session_picker_open = state_snap.session_picker.open;
            let tab_bar_has_tabs = !state_snap.tab_bar.tabs.is_empty();

            drop(state_snap);

            if let Err(e) = host.draw(|frame| {
                let full = frame.area();
                // Reserve last row for status bar (+ 1 if cmdline active)
                let status_height: u16 = 1 + if cmdline_active || search_active { 1 } else { 0 };
                let main_area = ratatui::layout::Rect {
                    height: full.height.saturating_sub(status_height),
                    ..full
                };
                let status_area = ratatui::layout::Rect {
                    y: full.y + full.height.saturating_sub(1),
                    height: 1,
                    ..full
                };

                // Render panes.
                for (pane_id, rect) in &pane_rects {
                    let is_focused = *pane_id == focused;
                    // Clip to main area
                    let clipped = clip_rect(*rect, main_area);
                    if clipped.width == 0 || clipped.height == 0 {
                        continue;
                    }
                    // Title: file name when editing, nothing when empty
                    let border_title: Option<&str> = if editor_lines.is_some() {
                        editor_file.as_deref().or(Some("editing"))
                    } else {
                        None
                    };
                    frame.render_widget(
                        PaneBorder { title: border_title, focused: is_focused },
                        clipped,
                    );

                    // Render terminal pane if session is available and focused
                    if let Some(ref sess_arc) = terminal_session {
                        if editor_lines.is_none() {
                            if let Some(sess) = sess_arc.try_lock() {
                                let inner = ratatui::layout::Rect {
                                    x: clipped.x + 1,
                                    y: clipped.y + 1,
                                    width: clipped.width.saturating_sub(2),
                                    height: clipped.height.saturating_sub(2),
                                };
                                frame.render_widget(
                                    TerminalPaneWidget { screen: &sess.screen, focused: is_focused },
                                    inner,
                                );
                                continue;
                            }
                        }
                    }

                    // Welcome screen when no file is open
                    if editor_lines.is_none() && is_focused {
                        let inner = ratatui::layout::Rect {
                            x: clipped.x + 1,
                            y: clipped.y + 1,
                            width: clipped.width.saturating_sub(2),
                            height: clipped.height.saturating_sub(2),
                        };
                        if inner.width >= 30 && inner.height >= 8 {
                            use ratatui::text::{Line, Span};
                            use ratatui::widgets::Paragraph;
                            use ratatui::layout::Alignment;
                            use ratatui::style::{Color, Modifier, Style};
                            let title_st = Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD);
                            let key_st   = Style::default().fg(Color::Yellow);
                            let desc_st  = Style::default().fg(Color::White);
                            let dim_st   = Style::default().fg(Color::DarkGray);
                            let sep_st   = Style::default().fg(Color::DarkGray);
                            let lines_w = vec![
                                Line::from(""),
                                Line::from(Span::styled("rmtide", title_st)),
                                Line::from(Span::styled("modal terminal IDE", dim_st)),
                                Line::from(""),
                                Line::from(vec![Span::styled(":e <path>  ", key_st), Span::styled("open file", desc_st)]),
                                Line::from(vec![Span::styled("Ctrl+P     ", key_st), Span::styled("command palette", desc_st)]),
                                Line::from(vec![Span::styled("Alt+E      ", key_st), Span::styled("file tree", desc_st)]),
                                Line::from(vec![Span::styled("Alt+A      ", key_st), Span::styled("AI agent", desc_st)]),
                                Line::from(vec![Span::styled("Alt+T      ", key_st), Span::styled("task runner", desc_st)]),
                                Line::from(vec![Span::styled("Ctrl+Q     ", key_st), Span::styled("quit", desc_st)]),
                                Line::from(""),
                                Line::from(vec![
                                    Span::styled("i", key_st), Span::styled(" → INSERT  ", sep_st),
                                    Span::styled("Esc", key_st), Span::styled(" → NORMAL  ", sep_st),
                                    Span::styled(":", key_st), Span::styled(" → COMMAND", sep_st),
                                ]),
                            ];
                            let content_h = lines_w.len() as u16;
                            let top_pad = inner.height.saturating_sub(content_h) / 2;
                            let welcome_area = ratatui::layout::Rect {
                                y: inner.y + top_pad,
                                height: content_h.min(inner.height.saturating_sub(top_pad)),
                                ..inner
                            };
                            frame.render_widget(
                                Paragraph::new(lines_w).alignment(Alignment::Center),
                                welcome_area,
                            );
                        }
                    }

                    // Render editor content if we have display data
                    if let Some(ref lines) = editor_lines {
                        if let (Some((cline, ccol)), Some((srow, scol))) = (editor_cursor, editor_scroll) {
                            // Render lines inside the border
                            let inner = ratatui::layout::Rect {
                                x: clipped.x + 1,
                                y: clipped.y + 1,
                                width: clipped.width.saturating_sub(2),
                                height: clipped.height.saturating_sub(2),
                            };
                            let gutter_w: u16 = 5;
                            let content_x = inner.x + gutter_w;
                            let content_w = inner.width.saturating_sub(gutter_w) as usize;

                            for row in 0..inner.height as usize {
                                let buf_line = srow + row;
                                if buf_line >= lines.len() { break; }
                                let screen_y = inner.y + row as u16;
                                let line_text = &lines[buf_line];
                                let line_chars: Vec<char> = line_text.chars().collect();

                                // Gutter
                                let num_str = format!("{:>4} ", buf_line + 1);
                                for (i, ch) in num_str.chars().enumerate() {
                                    if i >= gutter_w as usize { break; }
                                    let gstyle = if buf_line == cline {
                                        ratatui::style::Style::default().fg(ratatui::style::Color::Yellow)
                                    } else {
                                        ratatui::style::Style::default().fg(ratatui::style::Color::DarkGray)
                                    };
                                    frame.buffer_mut().get_mut(inner.x + i as u16, screen_y)
                                        .set_char(ch).set_style(gstyle);
                                }

                                // Text
                                for col in 0..content_w {
                                    let char_col = scol + col;
                                    let screen_x = content_x + col as u16;
                                    let ch = line_chars.get(char_col).copied().unwrap_or(' ');
                                    let is_cursor = buf_line == cline && char_col == ccol;
                                    let style = if is_cursor && is_focused {
                                        ratatui::style::Style::default()
                                            .bg(ratatui::style::Color::White)
                                            .fg(ratatui::style::Color::Black)
                                    } else {
                                        ratatui::style::Style::default()
                                    };
                                    frame.buffer_mut().get_mut(screen_x, screen_y)
                                        .set_char(ch).set_style(style);
                                }
                            }
                        }
                    }
                }

                // Render AI chat pane if a chat session snapshot is available.
                // It renders in the last pane area as an overlay (simple approach).
                if let Some(ref snap) = chat_snapshot {
                    // Find largest pane rect to render chat in
                    if let Some((_, rect)) = pane_rects.iter().max_by_key(|(_, r)| r.area()) {
                        let clipped = clip_rect(*rect, main_area);
                        if clipped.width > 4 && clipped.height > 4 {
                            let inner = ratatui::layout::Rect {
                                x: clipped.x + 1,
                                y: clipped.y + 1,
                                width: clipped.width.saturating_sub(2),
                                height: clipped.height.saturating_sub(2),
                            };
                            frame.render_widget(
                                ChatPaneWidget {
                                    display: ChatDisplay {
                                        display_lines: &snap.display_lines,
                                        scroll_offset: snap.scroll_offset,
                                        input: &snap.input,
                                        streaming: snap.streaming,
                                    },
                                    focused: true,
                                },
                                inner,
                            );
                        }
                    }
                }

                // Render status bar.
                let cursor_pos = editor_cursor.unwrap_or((0, 0));
                let backend_display = if !ai_status.is_empty() {
                    format!("{} | {}", backend, ai_status)
                } else {
                    backend.clone()
                };
                frame.render_widget(
                    StatusBar {
                        mode: &mode,
                        file_name: editor_file.as_deref(),
                        branch: None,
                        backend: &backend_display,
                        cursor_pos: (cursor_pos.0, cursor_pos.1),
                        is_modified: editor_dirty,
                    },
                    status_area,
                );

                // Render model picker overlay if open.
                if let Some(ref entries) = model_picker_entries {
                    let picker_w = (full.width * 3 / 5).max(40).min(full.width);
                    let picker_h = ((entries.len() + 2) as u16).min(full.height * 3 / 5).max(4);
                    let picker_x = full.x + (full.width.saturating_sub(picker_w)) / 2;
                    let picker_y = full.y + (full.height.saturating_sub(picker_h)) / 2;
                    let picker_area = ratatui::layout::Rect {
                        x: picker_x,
                        y: picker_y,
                        width: picker_w,
                        height: picker_h,
                    };
                    frame.render_widget(
                        ModelPickerWidget {
                            entries: entries.as_slice(),
                            selected: model_picker_selected,
                        },
                        picker_area,
                    );
                }

                // Phase 9 — Render agent panel overlay when open.
                if agent_panel_open {
                    let state_snap2 = state.read();
                    let panel_w = (full.width * 2 / 3).max(50).min(full.width);
                    let panel_h = (full.height * 3 / 4).min(full.height.saturating_sub(2));
                    let panel_x = full.x + (full.width.saturating_sub(panel_w)) / 2;
                    let panel_y = full.y + (full.height.saturating_sub(panel_h)) / 2;
                    let panel_area = ratatui::layout::Rect {
                        x: panel_x, y: panel_y, width: panel_w, height: panel_h,
                    };
                    // Render then release lock
                    let panel_widget = AgentPanelWidget {
                        state: &state_snap2.agent_panel_state,
                        focused: true,
                    };
                    frame.render_widget(panel_widget, panel_area);
                    // state_snap2 drops here
                }

                // Phase 9 — Render tool trace sidebar when open.
                if tool_trace_open {
                    // Clone calls first so we don't hold the lock during render
                    let (calls, trace_w) = {
                        let state_snap2 = state.read();
                        let c: Vec<ai::agent::ToolCall> = state_snap2
                            .agent_panel_state.session.as_ref()
                            .map(|s| s.tool_trace.clone())
                            .unwrap_or_default();
                        (c, (full.width / 3).max(30).min(full.width))
                    };
                    let trace_area = ratatui::layout::Rect {
                        x: full.x + full.width.saturating_sub(trace_w),
                        y: full.y,
                        width: trace_w,
                        height: full.height.saturating_sub(1),
                    };
                    frame.render_widget(
                        ToolTraceWidget {
                            calls: &calls,
                            scroll: 0,
                            selected: 0,
                            focused: false,
                        },
                        trace_area,
                    );
                }

                // Phase 9 — Render prompt library overlay when open.
                if prompt_library_open {
                    let state_snap2 = state.read();
                    let lib_w = (full.width * 3 / 4).max(60).min(full.width);
                    let lib_h = (full.height * 3 / 4).min(full.height.saturating_sub(2));
                    let lib_x = full.x + (full.width.saturating_sub(lib_w)) / 2;
                    let lib_y = full.y + (full.height.saturating_sub(lib_h)) / 2;
                    let lib_area = ratatui::layout::Rect {
                        x: lib_x, y: lib_y, width: lib_w, height: lib_h,
                    };
                    let prompt_widget = PromptLibraryWidget { state: &state_snap2.prompt_library_state };
                    frame.render_widget(prompt_widget, lib_area);
                    // state_snap2 drops here
                }

                // Phase 9 — Render context picker overlay when open.
                if context_picker_open {
                    let state_snap2 = state.read();
                    let ctx_widget = ContextPickerWidget { state: &state_snap2.context_picker_state };
                    frame.render_widget(ctx_widget, full);
                    // state_snap2 drops here
                }

                // ── Phase 10 rendering ───────────────────────────────────────

                // File tree: left sidebar (20 cols) when open
                if file_tree_open {
                    let tree_w = 22u16;
                    let tree_area = ratatui::layout::Rect {
                        x: full.x,
                        y: full.y,
                        width: tree_w.min(full.width),
                        height: main_area.height,
                    };
                    let state_snap2 = state.read();
                    use crate::widgets::file_tree::FileTreeWidget;
                    frame.render_widget(
                        FileTreeWidget { state: &state_snap2.file_tree_state, focused: false },
                        tree_area,
                    );
                }

                // Tab bar: 1 row above editor area when tabs exist
                if tab_bar_has_tabs {
                    let tab_area = ratatui::layout::Rect {
                        x: full.x,
                        y: full.y,
                        width: full.width,
                        height: 1,
                    };
                    let state_snap2 = state.read();
                    use crate::widgets::tab_bar::TabBarWidget;
                    frame.render_widget(
                        TabBarWidget { state: &state_snap2.tab_bar },
                        tab_area,
                    );
                }

                // Minimap: right sidebar when open
                if minimap_open {
                    let state_snap2 = state.read();
                    let mm_w = state_snap2.minimap_state.width;
                    let mm_area = ratatui::layout::Rect {
                        x: full.x + full.width.saturating_sub(mm_w),
                        y: full.y,
                        width: mm_w.min(full.width),
                        height: main_area.height,
                    };
                    // We need editor lines for the minimap
                    let lines: Vec<String> = state_snap2.editor_display.as_ref()
                        .map(|e| e.lines.clone())
                        .unwrap_or_default();
                    use crate::widgets::minimap::MinimapWidget;
                    frame.render_widget(
                        MinimapWidget { state: &state_snap2.minimap_state, lines: &lines },
                        mm_area,
                    );
                }

                // DAP panel: bottom panel when open
                if dap_panel_open {
                    let dap_h = (full.height / 4).max(8).min(full.height / 2);
                    let dap_area = ratatui::layout::Rect {
                        x: full.x,
                        y: full.y + full.height.saturating_sub(dap_h + 1),
                        width: full.width,
                        height: dap_h,
                    };
                    // Borrow Arc<DapClient> and Arc<BreakpointManager> from state
                    // Since we only have RenderState, we render a placeholder.
                    // The app state holds the actual Arc<DapClient>; here we render
                    // using DapPanelState only (no live client access from render thread).
                    // Use a local stub client for display purposes.
                    {
                        let state_snap2 = state.read();
                        // Render just the panel border/tab bar using a stub client
                        let bg = ratatui::style::Style::default().bg(ratatui::style::Color::Rgb(20,20,30));
                        for y in dap_area.y..dap_area.y + dap_area.height {
                            for x in dap_area.x..dap_area.x + dap_area.width {
                                frame.buffer_mut().get_mut(x, y).set_char(' ').set_style(bg);
                            }
                        }
                        let title = " DEBUG PANEL (F5 to launch) ";
                        let style = ratatui::style::Style::default().fg(ratatui::style::Color::Red);
                        frame.buffer_mut().get_mut(dap_area.x, dap_area.y).set_char('┌').set_style(style);
                        for (i, ch) in title.chars().enumerate() {
                            if dap_area.x + 1 + i as u16 >= dap_area.x + dap_area.width { break; }
                            frame.buffer_mut().get_mut(dap_area.x + 1 + i as u16, dap_area.y)
                                .set_char(ch).set_style(style);
                        }
                    }
                }

                // Find/replace overlay when open
                if find_replace_open {
                    let state_snap2 = state.read();
                    use crate::widgets::find_replace::FindReplaceWidget;
                    frame.render_widget(
                        FindReplaceWidget { state: &state_snap2.find_replace },
                        full,
                    );
                }

                // Symbol browser overlay when open
                if symbol_browser_open {
                    let state_snap2 = state.read();
                    use crate::widgets::symbol_browser::SymbolBrowserWidget;
                    frame.render_widget(
                        SymbolBrowserWidget { state: &state_snap2.symbol_browser },
                        full,
                    );
                }

                // Bookmark picker overlay when open
                if bookmark_picker_open {
                    let state_snap2 = state.read();
                    use crate::widgets::bookmarks::BookmarkPickerWidget;
                    frame.render_widget(
                        BookmarkPickerWidget { state: &state_snap2.bookmark_picker },
                        full,
                    );
                }

                // Macro panel overlay when open
                if macro_panel_open {
                    let state_snap2 = state.read();
                    use crate::widgets::macro_panel::MacroPanelWidget;
                    frame.render_widget(
                        MacroPanelWidget { state: &state_snap2.macro_panel, manager: None },
                        full,
                    );
                }

                // Clipboard picker overlay when open
                if clipboard_picker_open {
                    let state_snap2 = state.read();
                    use crate::widgets::clipboard_ring::ClipboardPickerWidget;
                    frame.render_widget(
                        ClipboardPickerWidget { state: &state_snap2.clipboard_picker },
                        full,
                    );
                }

                // Session picker overlay when open
                if session_picker_open {
                    let state_snap2 = state.read();
                    use crate::widgets::session_manager::SessionPickerWidget;
                    frame.render_widget(
                        SessionPickerWidget { state: &state_snap2.session_picker },
                        full,
                    );
                }

                // ── Phase 11 rendering ───────────────────────────────────────

                // Task runner panel — centered overlay
                if task_runner_open {
                    let state_snap2 = state.read();
                    let panel_w = (full.width * 3 / 4).max(60).min(full.width);
                    let panel_h = (full.height * 3 / 4).min(full.height.saturating_sub(2));
                    let panel_x = full.x + (full.width.saturating_sub(panel_w)) / 2;
                    let panel_y = full.y + (full.height.saturating_sub(panel_h)) / 2;
                    let panel_area = ratatui::layout::Rect { x: panel_x, y: panel_y, width: panel_w, height: panel_h };
                    use crate::widgets::task_runner_panel::TaskRunnerWidget;
                    frame.render_widget(
                        TaskRunnerWidget {
                            state: &state_snap2.task_runner_state,
                            records: &state_snap2.task_records,
                        },
                        panel_area,
                    );
                }

                // Log viewer — full screen overlay
                if log_viewer_open {
                    let state_snap2 = state.read();
                    let lv_area = ratatui::layout::Rect {
                        x: full.x + 2,
                        y: full.y + 1,
                        width: full.width.saturating_sub(4),
                        height: full.height.saturating_sub(2),
                    };
                    use crate::widgets::log_viewer::LogViewerWidget;
                    frame.render_widget(
                        LogViewerWidget { state: &state_snap2.log_viewer_state },
                        lv_area,
                    );
                }

                // Diff review — full screen overlay
                if diff_review_open {
                    let state_snap2 = state.read();
                    use crate::widgets::diff_review::DiffReviewWidget;
                    frame.render_widget(
                        DiffReviewWidget { state: &state_snap2.diff_review_state },
                        full,
                    );
                }

                // Process panel — centered overlay
                if process_panel_open {
                    let state_snap2 = state.read();
                    let panel_w = (full.width * 3 / 4).max(70).min(full.width);
                    let panel_h = (full.height / 2).max(12).min(full.height.saturating_sub(2));
                    let panel_x = full.x + (full.width.saturating_sub(panel_w)) / 2;
                    let panel_y = full.y + (full.height.saturating_sub(panel_h)) / 2;
                    let panel_area = ratatui::layout::Rect { x: panel_x, y: panel_y, width: panel_w, height: panel_h };
                    use crate::widgets::process_panel::ProcessPanelWidget;
                    frame.render_widget(
                        ProcessPanelWidget {
                            state: &state_snap2.process_panel_state,
                            processes: &state_snap2.processes,
                        },
                        panel_area,
                    );
                }

                // Port panel — centered overlay
                if port_panel_open {
                    let state_snap2 = state.read();
                    let panel_w = (full.width * 2 / 3).max(60).min(full.width);
                    let panel_h = (full.height / 2).max(12).min(full.height.saturating_sub(2));
                    let panel_x = full.x + (full.width.saturating_sub(panel_w)) / 2;
                    let panel_y = full.y + (full.height.saturating_sub(panel_h)) / 2;
                    let panel_area = ratatui::layout::Rect { x: panel_x, y: panel_y, width: panel_w, height: panel_h };
                    use crate::widgets::port_panel::PortPanelWidget;
                    frame.render_widget(
                        PortPanelWidget { state: &state_snap2.port_panel_state },
                        panel_area,
                    );
                }

                // Deploy panel — centered overlay
                if deploy_panel_open {
                    let state_snap2 = state.read();
                    let panel_w = (full.width * 2 / 3).max(60).min(full.width);
                    let panel_h = (full.height * 2 / 3).max(20).min(full.height.saturating_sub(2));
                    let panel_x = full.x + (full.width.saturating_sub(panel_w)) / 2;
                    let panel_y = full.y + (full.height.saturating_sub(panel_h)) / 2;
                    let panel_area = ratatui::layout::Rect { x: panel_x, y: panel_y, width: panel_w, height: panel_h };
                    use crate::widgets::deploy_panel::DeployPanelWidget;
                    frame.render_widget(
                        DeployPanelWidget { state: &state_snap2.deploy_panel_state },
                        panel_area,
                    );
                }

                // Env panel — full screen overlay
                if env_panel_open {
                    let env_area = ratatui::layout::Rect {
                        x: full.x + 2,
                        y: full.y + 1,
                        width: full.width.saturating_sub(4),
                        height: full.height.saturating_sub(2),
                    };
                    // EnvPanelWidget needs &runner::EnvManager, but render thread only has RenderState.
                    // We render a placeholder panel using state data from RenderState.
                    let state_snap2 = state.read();
                    // Draw the env panel using embedded state only — EnvManager is not in RenderState,
                    // so we render the panel border and hint text here, and rely on env_panel_state.
                    let bg_style = ratatui::style::Style::default().bg(ratatui::style::Color::Rgb(14, 14, 22));
                    for y in env_area.y..env_area.y + env_area.height {
                        for x in env_area.x..env_area.x + env_area.width {
                            frame.buffer_mut().get_mut(x, y).set_char(' ').set_style(bg_style);
                        }
                    }
                    let title = " Env Manager — use :Env to manage environment variables ";
                    let title_style = ratatui::style::Style::default().fg(ratatui::style::Color::Yellow);
                    for (i, ch) in title.chars().enumerate() {
                        if env_area.x + i as u16 >= env_area.x + env_area.width {
                            break;
                        }
                        frame.buffer_mut().get_mut(env_area.x + i as u16, env_area.y)
                            .set_char(ch).set_style(title_style);
                    }
                    let mask_info = if state_snap2.env_panel_state.show_values {
                        " [Values visible — press v to mask] "
                    } else {
                        " [Values masked — press v to reveal] "
                    };
                    for (i, ch) in mask_info.chars().enumerate() {
                        if env_area.x + i as u16 >= env_area.x + env_area.width {
                            break;
                        }
                        frame.buffer_mut().get_mut(env_area.x + i as u16, env_area.y + 1)
                            .set_char(ch)
                            .set_style(ratatui::style::Style::default().fg(ratatui::style::Color::DarkGray));
                    }
                }

                // HTTP panel — full screen overlay
                if http_panel_open {
                    let state_snap2 = state.read();
                    // HTTP panel needs &runner::HttpClient, render thread doesn't have it.
                    // Draw panel frame only using HttpPanelState.
                    let http_area = ratatui::layout::Rect {
                        x: full.x + 1,
                        y: full.y,
                        width: full.width.saturating_sub(2),
                        height: full.height.saturating_sub(1),
                    };
                    let bg_style = ratatui::style::Style::default().bg(ratatui::style::Color::Rgb(12, 16, 22));
                    for y in http_area.y..http_area.y + http_area.height {
                        for x in http_area.x..http_area.x + http_area.width {
                            frame.buffer_mut().get_mut(x, y).set_char(' ').set_style(bg_style);
                        }
                    }
                    let state = &state_snap2.http_panel_state;
                    let tab_labels = [" Collections ", " Request ", " Response "];
                    let active_tab = match state.tab {
                        crate::widgets::http_panel::HttpPanelTab::Collections => 0,
                        crate::widgets::http_panel::HttpPanelTab::Request => 1,
                        crate::widgets::http_panel::HttpPanelTab::Response => 2,
                    };
                    let title = " HTTP Client ";
                    let title_style = ratatui::style::Style::default()
                        .fg(ratatui::style::Color::Blue)
                        .add_modifier(ratatui::style::Modifier::BOLD);
                    for (i, ch) in title.chars().enumerate() {
                        if http_area.x + i as u16 >= http_area.x + http_area.width {
                            break;
                        }
                        frame.buffer_mut().get_mut(http_area.x + i as u16, http_area.y)
                            .set_char(ch).set_style(title_style);
                    }
                    let mut tx = http_area.x + title.len() as u16;
                    for (i, label) in tab_labels.iter().enumerate() {
                        let is_active = i == active_tab;
                        let tab_style = if is_active {
                            ratatui::style::Style::default()
                                .fg(ratatui::style::Color::Black)
                                .bg(ratatui::style::Color::Blue)
                        } else {
                            ratatui::style::Style::default().fg(ratatui::style::Color::DarkGray)
                        };
                        for ch in label.chars() {
                            if tx >= http_area.x + http_area.width {
                                break;
                            }
                            frame.buffer_mut().get_mut(tx, http_area.y).set_char(ch).set_style(tab_style);
                            tx += 1;
                        }
                    }
                    let req_line = format!("{} {}", state.current_request.method, state.current_request.url);
                    let req_style = ratatui::style::Style::default().fg(ratatui::style::Color::White);
                    for (i, ch) in req_line.chars().enumerate() {
                        if http_area.x + i as u16 >= http_area.x + http_area.width {
                            break;
                        }
                        frame.buffer_mut().get_mut(http_area.x + i as u16, http_area.y + 2)
                            .set_char(ch).set_style(req_style);
                    }
                    if let Some(ref resp) = state.last_response {
                        let resp_line = format!("Response: HTTP {} — {}ms", resp.status, resp.duration_ms);
                        let resp_color = if resp.status < 300 { ratatui::style::Color::Green } else { ratatui::style::Color::Red };
                        for (i, ch) in resp_line.chars().enumerate() {
                            if http_area.x + i as u16 >= http_area.x + http_area.width {
                                break;
                            }
                            frame.buffer_mut().get_mut(http_area.x + i as u16, http_area.y + 3)
                                .set_char(ch)
                                .set_style(ratatui::style::Style::default().fg(resp_color));
                        }
                    }
                    let hint = " [1]=Collections  [2]=Request  [3]=Response  [Enter]=Send  [q]=Close ";
                    let hint_y = http_area.y + http_area.height - 1;
                    for (i, ch) in hint.chars().enumerate() {
                        if http_area.x + i as u16 >= http_area.x + http_area.width {
                            break;
                        }
                        frame.buffer_mut().get_mut(http_area.x + i as u16, hint_y)
                            .set_char(ch)
                            .set_style(ratatui::style::Style::default().fg(ratatui::style::Color::DarkGray));
                    }
                }

                // DB panel — full screen overlay
                if db_panel_open {
                    let state_snap2 = state.read();
                    let db_area = ratatui::layout::Rect {
                        x: full.x + 1,
                        y: full.y,
                        width: full.width.saturating_sub(2),
                        height: full.height.saturating_sub(1),
                    };
                    let bg_style = ratatui::style::Style::default().bg(ratatui::style::Color::Rgb(14, 12, 22));
                    for y in db_area.y..db_area.y + db_area.height {
                        for x in db_area.x..db_area.x + db_area.width {
                            frame.buffer_mut().get_mut(x, y).set_char(' ').set_style(bg_style);
                        }
                    }
                    let db_state = &state_snap2.db_panel_state;
                    let tab_labels = [" Schema ", " Query ", " Results "];
                    let active_tab = match db_state.tab {
                        crate::widgets::db_panel::DbPanelTab::Schema => 0,
                        crate::widgets::db_panel::DbPanelTab::Query => 1,
                        crate::widgets::db_panel::DbPanelTab::Results => 2,
                    };
                    let title = " Database Client ";
                    let title_style = ratatui::style::Style::default()
                        .fg(ratatui::style::Color::Magenta)
                        .add_modifier(ratatui::style::Modifier::BOLD);
                    for (i, ch) in title.chars().enumerate() {
                        if db_area.x + i as u16 >= db_area.x + db_area.width {
                            break;
                        }
                        frame.buffer_mut().get_mut(db_area.x + i as u16, db_area.y)
                            .set_char(ch).set_style(title_style);
                    }
                    let mut tx = db_area.x + title.len() as u16;
                    for (i, label) in tab_labels.iter().enumerate() {
                        let is_active = i == active_tab;
                        let tab_style = if is_active {
                            ratatui::style::Style::default()
                                .fg(ratatui::style::Color::Black)
                                .bg(ratatui::style::Color::Magenta)
                        } else {
                            ratatui::style::Style::default().fg(ratatui::style::Color::DarkGray)
                        };
                        for ch in label.chars() {
                            if tx >= db_area.x + db_area.width {
                                break;
                            }
                            frame.buffer_mut().get_mut(tx, db_area.y).set_char(ch).set_style(tab_style);
                            tx += 1;
                        }
                    }
                    // Query buffer display
                    let query_y = db_area.y + 2;
                    if !db_state.query_buf.is_empty() && query_y < db_area.y + db_area.height {
                        let q_style = ratatui::style::Style::default()
                            .bg(ratatui::style::Color::Rgb(18, 18, 30))
                            .fg(ratatui::style::Color::White);
                        for (i, ch) in db_state.query_buf.chars().take(db_area.width as usize).enumerate() {
                            if db_area.x + i as u16 >= db_area.x + db_area.width {
                                break;
                            }
                            frame.buffer_mut().get_mut(db_area.x + i as u16, query_y)
                                .set_char(ch).set_style(q_style);
                        }
                    }
                    if let Some(ref result) = db_state.last_result {
                        let result_y = db_area.y + 4;
                        if result_y < db_area.y + db_area.height {
                            let info = if let Some(ref err) = result.error {
                                format!("Error: {}", err.chars().take(60).collect::<String>())
                            } else {
                                format!("{} rows × {} cols — {}ms", result.rows.len(), result.columns.len(), result.duration_ms)
                            };
                            let info_color = if result.error.is_some() {
                                ratatui::style::Color::Red
                            } else {
                                ratatui::style::Color::Green
                            };
                            for (i, ch) in info.chars().enumerate() {
                                if db_area.x + i as u16 >= db_area.x + db_area.width {
                                    break;
                                }
                                frame.buffer_mut().get_mut(db_area.x + i as u16, result_y)
                                    .set_char(ch)
                                    .set_style(ratatui::style::Style::default().fg(info_color));
                            }
                        }
                    }
                    let hint = " [1]=Schema  [2]=Query  [3]=Results  [Enter]=Run  [c]=Connect  [q]=Close ";
                    let hint_y = db_area.y + db_area.height - 1;
                    for (i, ch) in hint.chars().enumerate() {
                        if db_area.x + i as u16 >= db_area.x + db_area.width {
                            break;
                        }
                        frame.buffer_mut().get_mut(db_area.x + i as u16, hint_y)
                            .set_char(ch)
                            .set_style(ratatui::style::Style::default().fg(ratatui::style::Color::DarkGray));
                    }
                }

                // ── Phase 12 rendering ───────────────────────────────────────

                // Semantic search — centered overlay
                if semantic_search_open {
                    let state_snap2 = state.read();
                    use crate::widgets::semantic_search::SemanticSearchWidget;
                    frame.render_widget(SemanticSearchWidget { state: &state_snap2.semantic_search }, full);
                }

                // Commit composer — centered overlay
                if commit_composer_open {
                    let state_snap2 = state.read();
                    use crate::widgets::commit_composer::CommitComposerWidget;
                    frame.render_widget(CommitComposerWidget { state: &state_snap2.commit_composer }, full);
                }

                // Security panel — centered overlay
                if security_panel_open {
                    let state_snap2 = state.read();
                    use crate::widgets::security_panel::SecurityPanelWidget;
                    frame.render_widget(SecurityPanelWidget { state: &state_snap2.security_panel }, full);
                }

                // Analytics panel — centered overlay
                if analytics_panel_open {
                    let state_snap2 = state.read();
                    use crate::widgets::analytics_panel::AnalyticsPanelWidget;
                    frame.render_widget(AnalyticsPanelWidget { state: &state_snap2.analytics_panel }, full);
                }

                // Notebook mode — full screen
                if notebook_open {
                    let state_snap2 = state.read();
                    use crate::widgets::notebook::NotebookWidget;
                    frame.render_widget(NotebookWidget { state: &state_snap2.notebook_state }, full);
                }

                // Keymap editor — full screen overlay
                if keymap_editor_open {
                    let state_snap2 = state.read();
                    use crate::widgets::keymap_editor::KeymapEditorWidget;
                    frame.render_widget(KeymapEditorWidget { state: &state_snap2.keymap_editor }, full);
                }

                // Plugin marketplace — centered overlay
                if plugin_marketplace_open {
                    let state_snap2 = state.read();
                    use crate::widgets::plugin_marketplace::PluginMarketplaceWidget;
                    frame.render_widget(PluginMarketplaceWidget { state: &state_snap2.plugin_marketplace }, full);
                }

                // Collab panel — centered overlay
                if collab_open {
                    let state_snap2 = state.read();
                    use crate::widgets::collab_panel::CollabPanelWidget;
                    frame.render_widget(CollabPanelWidget { state: &state_snap2.collab_state }, full);
                }

                // Pair programmer — right side panel (20 cols) when active
                if pair_programmer_active {
                    let state_snap2 = state.read();
                    let pp_w = 22u16.min(full.width / 3);
                    let pp_area = ratatui::layout::Rect {
                        x: full.x + full.width.saturating_sub(pp_w),
                        y: full.y,
                        width: pp_w,
                        height: main_area.height,
                    };
                    use crate::widgets::pair_programmer::PairProgrammerWidget;
                    frame.render_widget(PairProgrammerWidget { state: &state_snap2.pair_programmer }, pp_area);
                }

                // Command palette — rendered LAST so it appears on top of everything
                if command_palette_open {
                    let state_snap2 = state.read();
                    use crate::widgets::command_palette::CommandPaletteWidget;
                    frame.render_widget(CommandPaletteWidget { state: &state_snap2.command_palette }, full);
                }

            }) {
                warn!("Render error: {e}");
            }
        }

        debug!("Render task exiting");
    });
}

fn clip_rect(r: ratatui::layout::Rect, bounds: ratatui::layout::Rect) -> ratatui::layout::Rect {
    let x = r.x.max(bounds.x);
    let y = r.y.max(bounds.y);
    let right = (r.x + r.width).min(bounds.x + bounds.width);
    let bottom = (r.y + r.height).min(bounds.y + bounds.height);
    ratatui::layout::Rect {
        x,
        y,
        width: right.saturating_sub(x),
        height: bottom.saturating_sub(y),
    }
}
