pub mod terminal_pane;
pub mod status_bar;
pub mod border;
pub mod editor_pane;
pub mod file_picker;
pub mod cmdline;
pub mod quickfix;
pub mod completion_popup;
pub mod hover_popup;
pub mod diagnostics_panel;
pub mod chat_pane;
pub mod model_picker;
pub mod git_panel;
pub mod git_diff_gutter;
pub mod git_blame;
pub mod git_branch;
// Phase 8 — BYOK + Spend + Approvals
pub mod keyring_panel;
pub mod spend_panel;
pub mod approval_modal;
pub mod model_matrix;
// Phase 9 — Agent + Prompt Library + Context Picker
pub mod agent_panel;
pub mod tool_trace;
pub mod prompt_picker;
pub mod context_picker;
// Phase 10 — File Tree, Tabs, Find/Replace, Symbol Browser, DAP, Minimap, Bookmarks, Macros, Clipboard, Sessions
pub mod file_tree;
pub mod tab_bar;
pub mod find_replace;
pub mod symbol_browser;
pub mod dap_panel;
pub mod minimap;
pub mod bookmarks;
pub mod macro_panel;
pub mod clipboard_ring;
pub mod session_manager;
// Phase 11 — Task Runner, Log Viewer, Diff Review, Process Panel, Port Panel,
//             Deploy Panel, Env Panel, HTTP Panel, DB Panel
pub mod task_runner_panel;
pub mod log_viewer;
pub mod diff_review;
pub mod process_panel;
pub mod port_panel;
pub mod deploy_panel;
pub mod env_panel;
pub mod http_panel;
pub mod db_panel;
// Phase 12 — Intelligence, Security & Polish
pub mod semantic_search;
pub mod commit_composer;
pub mod security_panel;
pub mod analytics_panel;
pub mod notebook;
pub mod keymap_editor;
pub mod plugin_marketplace;
pub mod collab_panel;
pub mod pair_programmer;
pub mod command_palette;
