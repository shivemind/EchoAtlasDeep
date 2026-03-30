#![allow(dead_code, unused_imports, unused_variables)]
//! Unified command palette — Phase 12 Point 50.
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::Widget,
};

#[derive(Clone, Debug)]
pub struct PaletteCommand {
    pub id: String,
    pub label: String,
    pub category: String,
    pub shortcut: Option<String>,
    pub icon: char,
    pub pinned: bool,
}

pub struct CommandPaletteState {
    pub open: bool,
    pub query: String,
    pub commands: Vec<PaletteCommand>,
    pub filtered: Vec<usize>,
    pub selected: usize,
    pub recent: Vec<String>,
    pub pinned: Vec<String>,
}

impl CommandPaletteState {
    pub fn new() -> Self {
        let mut s = Self {
            open: false,
            query: String::new(),
            commands: Vec::new(),
            filtered: Vec::new(),
            selected: 0,
            recent: Vec::new(),
            pinned: Vec::new(),
        };
        s.load_commands();
        s
    }

    /// Populate all commands (~50+).
    pub fn load_commands(&mut self) {
        self.commands = vec![
            // ── Editor ──────────────────────────────────────────────────────
            PaletteCommand { id: "editor.save".into(), label: "Save File".into(), category: "Editor".into(), shortcut: Some("Ctrl+S".into()), icon: '💾', pinned: false },
            PaletteCommand { id: "editor.save_all".into(), label: "Save All Files".into(), category: "Editor".into(), shortcut: Some(":wa".into()), icon: '💾', pinned: false },
            PaletteCommand { id: "editor.quit".into(), label: "Quit".into(), category: "Editor".into(), shortcut: Some(":q".into()), icon: '✕', pinned: false },
            PaletteCommand { id: "editor.force_quit".into(), label: "Force Quit".into(), category: "Editor".into(), shortcut: Some(":q!".into()), icon: '✕', pinned: false },
            PaletteCommand { id: "editor.split_h".into(), label: "Split Horizontally".into(), category: "Editor".into(), shortcut: Some(":split".into()), icon: '⬒', pinned: false },
            PaletteCommand { id: "editor.split_v".into(), label: "Split Vertically".into(), category: "Editor".into(), shortcut: Some(":vsplit".into()), icon: '⬓', pinned: false },
            PaletteCommand { id: "editor.undo".into(), label: "Undo".into(), category: "Editor".into(), shortcut: Some("u".into()), icon: '↩', pinned: false },
            PaletteCommand { id: "editor.redo".into(), label: "Redo".into(), category: "Editor".into(), shortcut: Some("Ctrl+R".into()), icon: '↪', pinned: false },
            PaletteCommand { id: "editor.select_all".into(), label: "Select All".into(), category: "Editor".into(), shortcut: Some("ggVG".into()), icon: '◻', pinned: false },
            PaletteCommand { id: "editor.format".into(), label: "Format Document".into(), category: "Editor".into(), shortcut: Some(":Format".into()), icon: '≡', pinned: false },
            // ── File ────────────────────────────────────────────────────────
            PaletteCommand { id: "file.open".into(), label: "Open File".into(), category: "File".into(), shortcut: Some("Ctrl+P".into()), icon: '📂', pinned: true },
            PaletteCommand { id: "file.new".into(), label: "New File".into(), category: "File".into(), shortcut: Some(":New".into()), icon: '📄', pinned: false },
            PaletteCommand { id: "file.tree".into(), label: "Toggle File Tree".into(), category: "File".into(), shortcut: Some("Alt+E".into()), icon: '🌲', pinned: false },
            PaletteCommand { id: "file.find_replace".into(), label: "Find & Replace".into(), category: "File".into(), shortcut: Some(":FindReplace".into()), icon: '🔍', pinned: false },
            PaletteCommand { id: "file.recent".into(), label: "Recent Files".into(), category: "File".into(), shortcut: None, icon: '🕐', pinned: false },
            PaletteCommand { id: "file.sessions".into(), label: "Sessions".into(), category: "File".into(), shortcut: Some(":Sessions".into()), icon: '📦', pinned: false },
            PaletteCommand { id: "file.session_save".into(), label: "Save Session".into(), category: "File".into(), shortcut: Some(":SessionSave".into()), icon: '📦', pinned: false },
            // ── Symbols ─────────────────────────────────────────────────────
            PaletteCommand { id: "symbols.browser".into(), label: "Symbol Browser".into(), category: "Symbols".into(), shortcut: Some(":Symbols".into()), icon: '⊕', pinned: false },
            PaletteCommand { id: "symbols.goto_def".into(), label: "Go to Definition".into(), category: "Symbols".into(), shortcut: Some("gd".into()), icon: '→', pinned: false },
            PaletteCommand { id: "symbols.goto_ref".into(), label: "Go to References".into(), category: "Symbols".into(), shortcut: Some("gr".into()), icon: '⇥', pinned: false },
            PaletteCommand { id: "symbols.hover".into(), label: "Hover Documentation".into(), category: "Symbols".into(), shortcut: Some("K".into()), icon: 'ℹ', pinned: false },
            PaletteCommand { id: "symbols.rename".into(), label: "Rename Symbol".into(), category: "Symbols".into(), shortcut: Some(":Rename".into()), icon: '✎', pinned: false },
            PaletteCommand { id: "symbols.bookmarks".into(), label: "Bookmarks".into(), category: "Symbols".into(), shortcut: Some("Alt+B".into()), icon: '🔖', pinned: false },
            // ── Git ─────────────────────────────────────────────────────────
            PaletteCommand { id: "git.panel".into(), label: "Git Panel".into(), category: "Git".into(), shortcut: Some(":Git".into()), icon: '⎇', pinned: false },
            PaletteCommand { id: "git.blame".into(), label: "Toggle Git Blame".into(), category: "Git".into(), shortcut: Some(":GitBlame".into()), icon: '👁', pinned: false },
            PaletteCommand { id: "git.branches".into(), label: "Branch Switcher".into(), category: "Git".into(), shortcut: Some(":GitBranch".into()), icon: '⎇', pinned: false },
            PaletteCommand { id: "git.commit".into(), label: "AI Commit Composer".into(), category: "Git".into(), shortcut: Some("cc".into()), icon: '✉', pinned: false },
            PaletteCommand { id: "git.diff_review".into(), label: "Diff Review".into(), category: "Git".into(), shortcut: Some(":DiffReview".into()), icon: '±', pinned: false },
            // ── AI ──────────────────────────────────────────────────────────
            PaletteCommand { id: "ai.agent".into(), label: "Toggle Agent Panel".into(), category: "AI".into(), shortcut: Some("Alt+A".into()), icon: '🤖', pinned: true },
            PaletteCommand { id: "ai.semantic_search".into(), label: "Semantic Code Search".into(), category: "AI".into(), shortcut: Some(":SemanticSearch".into()), icon: '🔎', pinned: false },
            PaletteCommand { id: "ai.pair_programmer".into(), label: "AI Pair Programmer".into(), category: "AI".into(), shortcut: Some("Alt+Shift+P".into()), icon: '👥', pinned: false },
            PaletteCommand { id: "ai.prompt_library".into(), label: "Prompt Library".into(), category: "AI".into(), shortcut: Some("Alt+P".into()), icon: '📚', pinned: false },
            PaletteCommand { id: "ai.context_picker".into(), label: "Context Picker".into(), category: "AI".into(), shortcut: Some(":Context".into()), icon: '📎', pinned: false },
            PaletteCommand { id: "ai.model_picker".into(), label: "Switch AI Model".into(), category: "AI".into(), shortcut: Some(":Model".into()), icon: '🧠', pinned: false },
            PaletteCommand { id: "ai.code_review".into(), label: "AI Code Review".into(), category: "AI".into(), shortcut: Some("Alt+R".into()), icon: '🔍', pinned: false },
            // ── Tasks ───────────────────────────────────────────────────────
            PaletteCommand { id: "tasks.runner".into(), label: "Task Runner".into(), category: "Tasks".into(), shortcut: Some("Alt+T".into()), icon: '▶', pinned: false },
            PaletteCommand { id: "tasks.logs".into(), label: "Log Viewer".into(), category: "Tasks".into(), shortcut: Some("Alt+L".into()), icon: '📜', pinned: false },
            PaletteCommand { id: "tasks.processes".into(), label: "Process Manager".into(), category: "Tasks".into(), shortcut: Some("Alt+P".into()), icon: '⚙', pinned: false },
            PaletteCommand { id: "tasks.ports".into(), label: "Port Panel".into(), category: "Tasks".into(), shortcut: Some(":Ports".into()), icon: '🔌', pinned: false },
            PaletteCommand { id: "tasks.live_server".into(), label: "Start Live Server".into(), category: "Tasks".into(), shortcut: Some(":LiveServer".into()), icon: '🌐', pinned: false },
            PaletteCommand { id: "tasks.debug".into(), label: "Start Debugger".into(), category: "Tasks".into(), shortcut: Some("F5".into()), icon: '🐛', pinned: false },
            // ── Deploy ──────────────────────────────────────────────────────
            PaletteCommand { id: "deploy.panel".into(), label: "Deploy Panel".into(), category: "Deploy".into(), shortcut: Some(":Deploy".into()), icon: '🚀', pinned: false },
            PaletteCommand { id: "deploy.http".into(), label: "HTTP Client".into(), category: "Deploy".into(), shortcut: Some(":Http".into()), icon: '🌐', pinned: false },
            PaletteCommand { id: "deploy.db".into(), label: "Database Client".into(), category: "Deploy".into(), shortcut: Some(":DB".into()), icon: '🗃', pinned: false },
            PaletteCommand { id: "deploy.env".into(), label: "Environment Manager".into(), category: "Deploy".into(), shortcut: Some(":Env".into()), icon: '🔧', pinned: false },
            // ── Settings ────────────────────────────────────────────────────
            PaletteCommand { id: "settings.keymaps".into(), label: "Keymap Editor".into(), category: "Settings".into(), shortcut: Some(":Keymaps".into()), icon: '⌨', pinned: false },
            PaletteCommand { id: "settings.theme".into(), label: "Change Theme".into(), category: "Settings".into(), shortcut: Some(":Theme".into()), icon: '🎨', pinned: false },
            PaletteCommand { id: "settings.plugins".into(), label: "Plugin Marketplace".into(), category: "Settings".into(), shortcut: Some(":PluginBrowse".into()), icon: '🧩', pinned: false },
            PaletteCommand { id: "settings.spend".into(), label: "Spend Panel".into(), category: "Settings".into(), shortcut: Some(":Spend".into()), icon: '💰', pinned: false },
            PaletteCommand { id: "settings.keyring".into(), label: "API Key Manager".into(), category: "Settings".into(), shortcut: Some(":Keys".into()), icon: '🔑', pinned: false },
            PaletteCommand { id: "settings.offline".into(), label: "Toggle Offline Mode".into(), category: "Settings".into(), shortcut: Some(":Offline".into()), icon: '✈', pinned: false },
            // ── Intelligence ────────────────────────────────────────────────
            PaletteCommand { id: "intel.security_scan".into(), label: "Security Scanner".into(), category: "Intelligence".into(), shortcut: Some(":SecScan".into()), icon: '🔒', pinned: false },
            PaletteCommand { id: "intel.analytics".into(), label: "Workspace Analytics".into(), category: "Intelligence".into(), shortcut: Some(":Analytics".into()), icon: '📊', pinned: false },
            PaletteCommand { id: "intel.notebook".into(), label: "Notebook Mode".into(), category: "Intelligence".into(), shortcut: Some(":Notebook".into()), icon: '📓', pinned: false },
            PaletteCommand { id: "intel.collab".into(), label: "Collaborative Session".into(), category: "Intelligence".into(), shortcut: Some(":Collab".into()), icon: '👥', pinned: false },
            PaletteCommand { id: "intel.minimap".into(), label: "Toggle Minimap".into(), category: "Intelligence".into(), shortcut: Some("Alt+M".into()), icon: '🗺', pinned: false },
            PaletteCommand { id: "intel.clipboard".into(), label: "Clipboard Ring".into(), category: "Intelligence".into(), shortcut: Some("Alt+C".into()), icon: '📋', pinned: false },
            PaletteCommand { id: "intel.macros".into(), label: "Macro Manager".into(), category: "Intelligence".into(), shortcut: Some("Alt+Q".into()), icon: '⏺', pinned: false },
        ];
        self.apply_filter();
    }

    pub fn filter(&mut self) {
        self.apply_filter();
    }

    fn apply_filter(&mut self) {
        if self.query.is_empty() {
            // Show pinned + recent first, then all
            let mut indices: Vec<usize> = Vec::new();

            // Recent commands (most recent first, limited to 5)
            for recent_id in self.recent.iter().take(5) {
                if let Some(idx) = self.commands.iter().position(|c| &c.id == recent_id) {
                    if !indices.contains(&idx) {
                        indices.push(idx);
                    }
                }
            }

            // Pinned commands
            for pinned_id in &self.pinned {
                if let Some(idx) = self.commands.iter().position(|c| &c.id == pinned_id) {
                    if !indices.contains(&idx) {
                        indices.push(idx);
                    }
                }
            }

            // Then all commands
            for idx in 0..self.commands.len() {
                if !indices.contains(&idx) {
                    indices.push(idx);
                }
            }

            self.filtered = indices;
        } else {
            let q = self.query.to_lowercase();
            // Fuzzy match: score by how closely the query matches
            let mut scored: Vec<(usize, usize)> = (0..self.commands.len())
                .filter_map(|i| {
                    let cmd = &self.commands[i];
                    let label_lc = cmd.label.to_lowercase();
                    let cat_lc = cmd.category.to_lowercase();
                    let id_lc = cmd.id.to_lowercase();
                    let shortcut_lc = cmd.shortcut.as_deref().unwrap_or("").to_lowercase();

                    if label_lc.contains(&q) {
                        // Exact prefix match scores highest
                        let score = if label_lc.starts_with(&q) { 100 }
                            else if label_lc.contains(&q) { 50 }
                            else { 10 };
                        Some((i, score))
                    } else if cat_lc.contains(&q) {
                        Some((i, 20))
                    } else if id_lc.contains(&q) {
                        Some((i, 30))
                    } else if shortcut_lc.contains(&q) {
                        Some((i, 25))
                    } else {
                        // Character-level fuzzy match
                        let mut qi = 0;
                        let q_chars: Vec<char> = q.chars().collect();
                        for ch in label_lc.chars() {
                            if qi < q_chars.len() && ch == q_chars[qi] {
                                qi += 1;
                            }
                        }
                        if qi == q_chars.len() {
                            Some((i, 5))
                        } else {
                            None
                        }
                    }
                })
                .collect();

            scored.sort_by(|a, b| b.1.cmp(&a.1));
            self.filtered = scored.into_iter().map(|(i, _)| i).collect();
        }

        if self.selected >= self.filtered.len() && !self.filtered.is_empty() {
            self.selected = self.filtered.len() - 1;
        }
        if self.filtered.is_empty() {
            self.selected = 0;
        }
    }

    pub fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    pub fn move_down(&mut self) {
        if self.selected + 1 < self.filtered.len() {
            self.selected += 1;
        }
    }

    pub fn selected_command(&self) -> Option<&PaletteCommand> {
        self.filtered.get(self.selected).and_then(|&i| self.commands.get(i))
    }

    pub fn record_recent(&mut self, id: &str) {
        self.recent.retain(|r| r != id);
        self.recent.insert(0, id.to_string());
        if self.recent.len() > 20 {
            self.recent.truncate(20);
        }
    }

    pub fn execute_id(&self) -> Option<String> {
        self.selected_command().map(|c| c.id.clone())
    }
}

impl Default for CommandPaletteState {
    fn default() -> Self {
        Self::new()
    }
}

pub struct CommandPaletteWidget<'a> {
    pub state: &'a CommandPaletteState,
}

impl Widget for CommandPaletteWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 10 || area.height < 6 {
            return;
        }

        let w = (area.width * 7 / 10).max(60).min(area.width);
        let visible_cmds = self.state.filtered.len().min(12);
        let h = (visible_cmds as u16 + 5).max(8).min(area.height);
        let x = area.x + (area.width.saturating_sub(w)) / 2;
        let y = area.y + (area.height.saturating_sub(h)) / 3; // Slightly above center

        let bg = Style::default().bg(Color::Rgb(18, 20, 30));
        let border_style = Style::default().fg(Color::White);
        let query_style = Style::default().fg(Color::White).bg(Color::Rgb(24, 28, 42));
        let prompt_style = Style::default().fg(Color::DarkGray).bg(Color::Rgb(24, 28, 42));
        let selected_style = Style::default().bg(Color::Rgb(40, 50, 80)).fg(Color::White);
        let normal_style = Style::default().fg(Color::Gray);
        let category_style = Style::default().fg(Color::DarkGray);
        let shortcut_style = Style::default().fg(Color::DarkGray);
        let pinned_style = Style::default().fg(Color::Yellow);
        let recent_style = Style::default().fg(Color::Cyan);
        let label_style = Style::default().fg(Color::DarkGray);

        // Semi-transparent background fill
        for row in y..y + h {
            for col in x..x + w {
                buf.get_mut(col, row).set_char(' ').set_style(bg);
            }
        }

        // Shadow effect (right and bottom edges, 1 char)
        let shadow_style = Style::default().bg(Color::Rgb(5, 5, 8));
        for row in y + 1..y + h + 1 {
            let sc = x + w;
            if sc < area.x + area.width {
                buf.get_mut(sc, row).set_char(' ').set_style(shadow_style);
            }
        }
        for col in x + 1..x + w + 1 {
            let sr = y + h;
            if sr < area.y + area.height {
                buf.get_mut(col, sr).set_char(' ').set_style(shadow_style);
            }
        }

        // Border
        buf.get_mut(x, y).set_char('╭').set_style(border_style);
        buf.get_mut(x + w - 1, y).set_char('╮').set_style(border_style);
        for col in x + 1..x + w - 1 {
            buf.get_mut(col, y).set_char('─').set_style(border_style);
        }
        buf.get_mut(x, y + h - 1).set_char('╰').set_style(border_style);
        buf.get_mut(x + w - 1, y + h - 1).set_char('╯').set_style(border_style);
        for col in x + 1..x + w - 1 {
            buf.get_mut(col, y + h - 1).set_char('─').set_style(border_style);
        }
        for row in y + 1..y + h - 1 {
            buf.get_mut(x, row).set_char('│').set_style(border_style);
            buf.get_mut(x + w - 1, row).set_char('│').set_style(border_style);
        }

        // Query bar (row y+1): "> query"
        for col in x + 1..x + w - 1 {
            buf.get_mut(col, y + 1).set_char(' ').set_style(query_style);
        }
        buf.get_mut(x + 1, y + 1).set_char(' ').set_style(prompt_style);
        buf.get_mut(x + 2, y + 1).set_char('>').set_style(prompt_style);
        buf.get_mut(x + 3, y + 1).set_char(' ').set_style(query_style);

        let query_display: String = self.state.query.chars().take((w as usize).saturating_sub(8)).collect();
        for (i, ch) in query_display.chars().enumerate() {
            let col = x + 4 + i as u16;
            if col >= x + w - 1 { break; }
            buf.get_mut(col, y + 1).set_char(ch).set_style(query_style);
        }

        // Cursor in query bar
        let cursor_col = x + 4 + query_display.len() as u16;
        if cursor_col < x + w - 1 {
            buf.get_mut(cursor_col, y + 1)
                .set_char('▍')
                .set_style(Style::default().fg(Color::White).bg(Color::Rgb(24, 28, 42)));
        }

        // Separator under query bar
        for col in x + 1..x + w - 1 {
            buf.get_mut(col, y + 2).set_char('─').set_style(
                Style::default().fg(Color::Rgb(50, 55, 80))
            );
        }
        buf.get_mut(x, y + 2).set_char('├').set_style(border_style);
        buf.get_mut(x + w - 1, y + 2).set_char('┤').set_style(border_style);

        // Results list
        let list_start = y + 3;
        let max_visible = (h as usize).saturating_sub(4);

        let scroll = if self.state.selected >= max_visible {
            self.state.selected - max_visible + 1
        } else {
            0
        };

        // Determine which indices are "recent"
        let recent_set: std::collections::HashSet<&str> = self.state.recent.iter()
            .take(5)
            .map(|s| s.as_str())
            .collect();

        // Determine which indices are "pinned"
        let pinned_set: std::collections::HashSet<&str> = self.state.pinned.iter()
            .map(|s| s.as_str())
            .collect();

        for (view_idx, &cmd_idx) in self.state.filtered.iter().enumerate().skip(scroll).take(max_visible) {
            let ry = list_start + (view_idx - scroll) as u16;
            if ry >= y + h - 1 { break; }

            let cmd = &self.state.commands[cmd_idx];
            let is_sel = view_idx == self.state.selected;
            let is_recent = recent_set.contains(cmd.id.as_str()) && self.state.query.is_empty();
            let is_pinned = pinned_set.contains(cmd.id.as_str()) || cmd.pinned;

            // Row background
            for col in x + 1..x + w - 1 {
                buf.get_mut(col, ry).set_char(' ').set_style(if is_sel { selected_style } else { normal_style });
            }

            // Recent/pinned badge
            let badge = if is_pinned {
                '★'
            } else if is_recent {
                '⏱'
            } else {
                ' '
            };
            let badge_style = if is_pinned {
                Style::default().fg(Color::Yellow).bg(if is_sel { Color::Rgb(40,50,80) } else { Color::Reset })
            } else if is_recent {
                Style::default().fg(Color::Cyan).bg(if is_sel { Color::Rgb(40,50,80) } else { Color::Reset })
            } else {
                if is_sel { selected_style } else { normal_style }
            };
            buf.get_mut(x + 1, ry).set_char(badge).set_style(badge_style);

            // Icon
            buf.get_mut(x + 2, ry).set_char(' ').set_style(if is_sel { selected_style } else { normal_style });
            buf.get_mut(x + 3, ry).set_char(cmd.icon).set_style(
                Style::default()
                    .fg(category_color_for(&cmd.category))
                    .bg(if is_sel { Color::Rgb(40,50,80) } else { Color::Reset })
            );

            // Category badge (short)
            let cat_short: String = cmd.category.chars().take(8).collect();
            let cat_display = format!("{:<9}", cat_short);
            let cat_col = x + 5;
            for (i, ch) in cat_display.chars().enumerate() {
                let col = cat_col + i as u16;
                if col >= x + w - 1 { break; }
                buf.get_mut(col, ry).set_char(ch).set_style(
                    Style::default()
                        .fg(category_color_for(&cmd.category))
                        .bg(if is_sel { Color::Rgb(40,50,80) } else { Color::Reset })
                );
            }

            // Label
            let label_col = cat_col + 10;
            let label_w = (w as usize).saturating_sub(32);
            let label_display: String = cmd.label.chars().take(label_w).collect();
            for (i, ch) in label_display.chars().enumerate() {
                let col = label_col + i as u16;
                if col >= x + w - 1 { break; }
                buf.get_mut(col, ry).set_char(ch).set_style(
                    if is_sel {
                        Style::default().fg(Color::White).bg(Color::Rgb(40,50,80)).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::Gray)
                    }
                );
            }

            // Shortcut (right-aligned)
            if let Some(ref shortcut) = cmd.shortcut {
                let sc_display: String = shortcut.chars().take(15).collect();
                let sc_x = x + w - 1 - sc_display.len() as u16 - 1;
                if sc_x > label_col + label_w as u16 {
                    for (i, ch) in sc_display.chars().enumerate() {
                        let col = sc_x + i as u16;
                        if col >= x + w - 1 { break; }
                        buf.get_mut(col, ry).set_char(ch).set_style(
                            if is_sel {
                                Style::default().fg(Color::LightYellow).bg(Color::Rgb(40,50,80))
                            } else {
                                shortcut_style
                            }
                        );
                    }
                }
            }
        }

        // Status hint at bottom
        let total = self.state.filtered.len();
        let shown = visible_cmds.min(total);
        let hint = if total == 0 {
            " No commands found".to_string()
        } else {
            format!(" {}/{} commands  [↑↓] navigate  [Enter] run  [Esc] close", shown, total)
        };
        for (i, ch) in hint.chars().take((w as usize).saturating_sub(2)).enumerate() {
            let col = x + 1 + i as u16;
            if col >= x + w - 1 { break; }
            buf.get_mut(col, y + h - 2).set_char(ch).set_style(label_style);
        }
    }
}

fn category_color_for(category: &str) -> Color {
    match category {
        "Editor" => Color::White,
        "File" => Color::Cyan,
        "Symbols" => Color::Blue,
        "Git" => Color::LightRed,
        "AI" => Color::Magenta,
        "Tasks" => Color::Yellow,
        "Deploy" => Color::Green,
        "Settings" => Color::DarkGray,
        "Intelligence" => Color::LightMagenta,
        _ => Color::Gray,
    }
}
