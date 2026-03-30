#![allow(dead_code, unused_imports, unused_variables)]
//! Custom keybinding editor — Phase 12 Point 46.
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::Widget,
};

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct KeyBinding {
    pub command: String,
    pub category: String,
    pub keys: Vec<String>,
    pub description: String,
}

pub struct KeymapEditorState {
    pub open: bool,
    pub bindings: Vec<KeyBinding>,
    pub filtered_indices: Vec<usize>,
    pub selected: usize,
    pub filter: String,
    pub capturing: bool,
    pub conflicts: Vec<(usize, usize)>,
}

impl KeymapEditorState {
    pub fn new() -> Self {
        let mut s = Self {
            open: false,
            bindings: Vec::new(),
            filtered_indices: Vec::new(),
            selected: 0,
            filter: String::new(),
            capturing: false,
            conflicts: Vec::new(),
        };
        s.load_defaults();
        s
    }

    pub fn load_defaults(&mut self) {
        self.bindings = vec![
            // Editor
            KeyBinding { command: "quit".into(), category: "Editor".into(), keys: vec!["Ctrl+q".into()], description: "Quit rmtide".into() },
            KeyBinding { command: "force_quit".into(), category: "Editor".into(), keys: vec!["Ctrl+q".into(), ":q!".into()], description: "Force quit without save".into() },
            KeyBinding { command: "save_file".into(), category: "Editor".into(), keys: vec!["Ctrl+s".into(), ":w".into()], description: "Save current buffer".into() },
            KeyBinding { command: "split_horizontal".into(), category: "Editor".into(), keys: vec![":split".into()], description: "Split editor horizontally".into() },
            KeyBinding { command: "split_vertical".into(), category: "Editor".into(), keys: vec![":vsplit".into()], description: "Split editor vertically".into() },
            KeyBinding { command: "focus_next_pane".into(), category: "Editor".into(), keys: vec!["Ctrl+w".into()], description: "Cycle pane focus".into() },
            KeyBinding { command: "enter_normal".into(), category: "Editor".into(), keys: vec!["Esc".into()], description: "Return to Normal mode".into() },
            KeyBinding { command: "enter_insert".into(), category: "Editor".into(), keys: vec!["i".into()], description: "Enter Insert mode".into() },
            KeyBinding { command: "enter_visual".into(), category: "Editor".into(), keys: vec!["v".into()], description: "Enter Visual mode".into() },
            // File
            KeyBinding { command: "file_picker".into(), category: "File".into(), keys: vec!["leader+f+f".into(), "Ctrl+p".into()], description: "Open file picker".into() },
            KeyBinding { command: "file_tree".into(), category: "File".into(), keys: vec!["Alt+e".into()], description: "Toggle file tree".into() },
            KeyBinding { command: "find_replace".into(), category: "File".into(), keys: vec![":FindReplace".into()], description: "Open find & replace".into() },
            KeyBinding { command: "tabs".into(), category: "File".into(), keys: vec![":Tabs".into()], description: "Show tab bar".into() },
            // AI
            KeyBinding { command: "agent_panel".into(), category: "AI".into(), keys: vec!["Alt+A".into()], description: "Toggle AI agent panel".into() },
            KeyBinding { command: "semantic_search".into(), category: "AI".into(), keys: vec!["Alt+f".into(), ":SemanticSearch".into()], description: "Semantic code search".into() },
            KeyBinding { command: "pair_programmer".into(), category: "AI".into(), keys: vec!["Alt+Shift+P".into()], description: "Toggle AI pair programmer".into() },
            KeyBinding { command: "prompt_library".into(), category: "AI".into(), keys: vec!["Alt+p".into()], description: "Open prompt library".into() },
            // Git
            KeyBinding { command: "git_panel".into(), category: "Git".into(), keys: vec![":Git".into()], description: "Open git panel".into() },
            KeyBinding { command: "git_blame".into(), category: "Git".into(), keys: vec![":GitBlame".into()], description: "Toggle git blame".into() },
            KeyBinding { command: "commit_composer".into(), category: "Git".into(), keys: vec!["cc".into()], description: "Open AI commit composer".into() },
            // Search
            KeyBinding { command: "search_forward".into(), category: "Search".into(), keys: vec!["/".into()], description: "Search forward".into() },
            KeyBinding { command: "search_backward".into(), category: "Search".into(), keys: vec!["?".into()], description: "Search backward".into() },
            KeyBinding { command: "search_next".into(), category: "Search".into(), keys: vec!["n".into()], description: "Next search match".into() },
            KeyBinding { command: "search_prev".into(), category: "Search".into(), keys: vec!["N".into()], description: "Previous search match".into() },
            // LSP
            KeyBinding { command: "lsp_hover".into(), category: "LSP".into(), keys: vec!["K".into()], description: "LSP hover info".into() },
            KeyBinding { command: "lsp_goto_def".into(), category: "LSP".into(), keys: vec!["gd".into()], description: "Go to definition".into() },
            KeyBinding { command: "lsp_goto_ref".into(), category: "LSP".into(), keys: vec!["gr".into()], description: "Go to references".into() },
            KeyBinding { command: "lsp_complete".into(), category: "LSP".into(), keys: vec!["Ctrl+Space".into()], description: "Trigger completion".into() },
            // Tools
            KeyBinding { command: "command_palette".into(), category: "Tools".into(), keys: vec!["Ctrl+P".into()], description: "Open command palette".into() },
            KeyBinding { command: "security_scan".into(), category: "Tools".into(), keys: vec![":SecScan".into()], description: "Run security scanner".into() },
            KeyBinding { command: "analytics".into(), category: "Tools".into(), keys: vec![":Analytics".into()], description: "Open workspace analytics".into() },
            KeyBinding { command: "keymaps".into(), category: "Tools".into(), keys: vec![":Keymaps".into()], description: "Open keymap editor".into() },
            KeyBinding { command: "plugin_browse".into(), category: "Tools".into(), keys: vec![":PluginBrowse".into()], description: "Browse plugin marketplace".into() },
            KeyBinding { command: "collab".into(), category: "Tools".into(), keys: vec![":Collab".into()], description: "Open collaborative session".into() },
            KeyBinding { command: "notebook".into(), category: "Tools".into(), keys: vec![":Notebook".into()], description: "Open notebook mode".into() },
            // Debug
            KeyBinding { command: "dap_panel".into(), category: "Debug".into(), keys: vec!["F5".into()], description: "Launch debugger".into() },
            KeyBinding { command: "dap_breakpoint".into(), category: "Debug".into(), keys: vec!["F9".into()], description: "Toggle breakpoint".into() },
            // Tasks
            KeyBinding { command: "task_runner".into(), category: "Tasks".into(), keys: vec!["Alt+T".into()], description: "Open task runner".into() },
            KeyBinding { command: "log_viewer".into(), category: "Tasks".into(), keys: vec!["Alt+L".into()], description: "Open log viewer".into() },
            KeyBinding { command: "process_panel".into(), category: "Tasks".into(), keys: vec!["Alt+P".into()], description: "Open process manager".into() },
        ];
        self.apply_filter();
        self.check_conflicts();
    }

    pub fn filter(&mut self) {
        self.apply_filter();
    }

    fn apply_filter(&mut self) {
        if self.filter.is_empty() {
            self.filtered_indices = (0..self.bindings.len()).collect();
        } else {
            let q = self.filter.to_lowercase();
            self.filtered_indices = (0..self.bindings.len())
                .filter(|&i| {
                    let b = &self.bindings[i];
                    b.command.to_lowercase().contains(&q)
                        || b.category.to_lowercase().contains(&q)
                        || b.description.to_lowercase().contains(&q)
                        || b.keys.iter().any(|k| k.to_lowercase().contains(&q))
                })
                .collect();
        }
        if self.selected >= self.filtered_indices.len() && !self.filtered_indices.is_empty() {
            self.selected = self.filtered_indices.len() - 1;
        }
    }

    pub fn check_conflicts(&mut self) {
        let mut conflicts = Vec::new();
        for i in 0..self.bindings.len() {
            for j in i + 1..self.bindings.len() {
                let keys_i: std::collections::HashSet<&String> = self.bindings[i].keys.iter().collect();
                let keys_j: std::collections::HashSet<&String> = self.bindings[j].keys.iter().collect();
                if keys_i.intersection(&keys_j).next().is_some() {
                    conflicts.push((i, j));
                }
            }
        }
        self.conflicts = conflicts;
    }

    pub fn save(&self) -> anyhow::Result<()> {
        let config_dir = dirs::config_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("rmtide");
        std::fs::create_dir_all(&config_dir)?;
        let path = config_dir.join("keymaps.toml");
        // Wrap in a table so TOML serializes cleanly
        #[derive(serde::Serialize)]
        struct KeymapsFile<'a> {
            bindings: &'a Vec<KeyBinding>,
        }
        let wrapper = KeymapsFile { bindings: &self.bindings };
        let toml_str = toml::to_string_pretty(&wrapper)
            .map_err(|e| anyhow::anyhow!("TOML serialize error: {}", e))?;
        std::fs::write(path, toml_str)?;
        Ok(())
    }

    pub fn selected_binding(&self) -> Option<&KeyBinding> {
        self.filtered_indices.get(self.selected).and_then(|&i| self.bindings.get(i))
    }

    pub fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    pub fn move_down(&mut self) {
        if self.selected + 1 < self.filtered_indices.len() {
            self.selected += 1;
        }
    }
}

impl Default for KeymapEditorState {
    fn default() -> Self {
        Self::new()
    }
}

pub struct KeymapEditorWidget<'a> {
    pub state: &'a KeymapEditorState,
}

impl Widget for KeymapEditorWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 10 || area.height < 8 {
            return;
        }

        let bg = Style::default().bg(Color::Rgb(14, 16, 22));
        let border_style = Style::default().fg(Color::Magenta);
        let title_style = Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD);
        let label_style = Style::default().fg(Color::DarkGray);
        let selected_style = Style::default().bg(Color::Rgb(40, 20, 50)).fg(Color::White);
        let normal_style = Style::default().fg(Color::Gray);
        let category_style = Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD);
        let key_style = Style::default().fg(Color::Yellow);
        let conflict_style = Style::default().fg(Color::Red);

        // Fill background
        for row in area.y..area.y + area.height {
            for col in area.x..area.x + area.width {
                buf.get_mut(col, row).set_char(' ').set_style(bg);
            }
        }

        // Border
        buf.get_mut(area.x, area.y).set_char('╔').set_style(border_style);
        buf.get_mut(area.x + area.width - 1, area.y).set_char('╗').set_style(border_style);
        for col in area.x + 1..area.x + area.width - 1 {
            buf.get_mut(col, area.y).set_char('═').set_style(border_style);
        }
        buf.get_mut(area.x, area.y + area.height - 1).set_char('╚').set_style(border_style);
        buf.get_mut(area.x + area.width - 1, area.y + area.height - 1).set_char('╝').set_style(border_style);
        for col in area.x + 1..area.x + area.width - 1 {
            buf.get_mut(col, area.y + area.height - 1).set_char('═').set_style(border_style);
        }
        for row in area.y + 1..area.y + area.height - 1 {
            buf.get_mut(area.x, row).set_char('║').set_style(border_style);
            buf.get_mut(area.x + area.width - 1, row).set_char('║').set_style(border_style);
        }

        // Title
        let title = " Keymap Editor ";
        let title_x = area.x + (area.width.saturating_sub(title.len() as u16)) / 2;
        for (i, ch) in title.chars().enumerate() {
            if title_x + i as u16 >= area.x + area.width - 1 { break; }
            buf.get_mut(title_x + i as u16, area.y).set_char(ch).set_style(title_style);
        }

        // Filter bar (row y+1)
        let filter_prompt = " Filter: ";
        for (i, ch) in filter_prompt.chars().enumerate() {
            let col = area.x + 1 + i as u16;
            if col >= area.x + area.width - 1 { break; }
            buf.get_mut(col, area.y + 1).set_char(ch).set_style(label_style);
        }
        let filter_display: String = self.state.filter.chars().take((area.width as usize).saturating_sub(12)).collect();
        let filter_style = Style::default().fg(Color::White).bg(Color::Rgb(22, 24, 34));
        for col in area.x + 1 + filter_prompt.len() as u16..area.x + area.width - 1 {
            buf.get_mut(col, area.y + 1).set_char(' ').set_style(filter_style);
        }
        for (i, ch) in filter_display.chars().enumerate() {
            let col = area.x + 1 + filter_prompt.len() as u16 + i as u16;
            if col >= area.x + area.width - 1 { break; }
            buf.get_mut(col, area.y + 1).set_char(ch).set_style(filter_style);
        }

        // Separator
        for col in area.x + 1..area.x + area.width - 1 {
            buf.get_mut(col, area.y + 2).set_char('─').set_style(border_style);
        }
        buf.get_mut(area.x, area.y + 2).set_char('╟').set_style(border_style);
        buf.get_mut(area.x + area.width - 1, area.y + 2).set_char('╢').set_style(border_style);

        // Split layout: left list (2/3), right details (1/3)
        let list_w = area.width * 2 / 3;
        let detail_x = area.x + list_w;

        // Vertical divider
        for row in area.y + 2..area.y + area.height - 1 {
            buf.get_mut(detail_x, row).set_char('│').set_style(border_style);
        }

        // Column headers
        let col_header = format!(" {:<20} {:<12} {}", "COMMAND", "CATEGORY", "KEYS");
        for (i, ch) in col_header.chars().take(list_w as usize - 2).enumerate() {
            let col = area.x + 1 + i as u16;
            if col >= detail_x { break; }
            buf.get_mut(col, area.y + 3).set_char(ch).set_style(label_style);
        }

        // Bindings list
        let list_start = area.y + 4;
        let list_height = (area.height as usize).saturating_sub(7);
        let scroll = if self.state.selected >= list_height {
            self.state.selected - list_height + 1
        } else {
            0
        };

        for (view_idx, &binding_idx) in self.state.filtered_indices.iter().enumerate().skip(scroll).take(list_height) {
            let ry = list_start + (view_idx - scroll) as u16;
            if ry >= area.y + area.height - 2 { break; }

            let binding = &self.state.bindings[binding_idx];
            let is_sel = view_idx == self.state.selected;
            let has_conflict = self.state.conflicts.iter().any(|(a, b)| *a == binding_idx || *b == binding_idx);

            let row_style = if is_sel { selected_style } else { normal_style };

            for col in area.x + 1..detail_x {
                buf.get_mut(col, ry).set_char(' ').set_style(row_style);
            }

            let cmd_short: String = binding.command.chars().take(19).collect();
            let cat_short: String = binding.category.chars().take(11).collect();
            let keys_str: String = binding.keys.join(", ");
            let keys_short: String = keys_str.chars().take(20).collect();

            let row_text = format!(" {:<20} {:<12} {}", cmd_short, cat_short, keys_short);
            for (i, ch) in row_text.chars().take((list_w as usize).saturating_sub(2)).enumerate() {
                let col = area.x + 1 + i as u16;
                if col >= detail_x { break; }
                let ch_style = if is_sel {
                    Style::default().fg(Color::White).bg(Color::Rgb(40,20,50))
                } else if has_conflict {
                    conflict_style
                } else {
                    normal_style
                };
                buf.get_mut(col, ry).set_char(ch).set_style(ch_style);
            }

            // Conflict indicator
            if has_conflict {
                let cx = detail_x.saturating_sub(4);
                if cx >= area.x + 1 {
                    buf.get_mut(cx, ry).set_char('⚠').set_style(conflict_style);
                }
            }
        }

        // Right detail panel
        let detail_area_x = detail_x + 1;
        let detail_area_w = area.width.saturating_sub(list_w + 2);

        if let Some(binding) = self.state.selected_binding() {
            let dy = area.y + 3;

            let detail_title = " Selected Command ";
            for (i, ch) in detail_title.chars().take(detail_area_w as usize) .enumerate() {
                let col = detail_area_x + i as u16;
                if col >= area.x + area.width - 1 { break; }
                buf.get_mut(col, dy).set_char(ch).set_style(label_style);
            }

            let cmd_label = format!(" Cmd:  {}", binding.command);
            for (i, ch) in cmd_label.chars().take(detail_area_w as usize).enumerate() {
                let col = detail_area_x + i as u16;
                if col >= area.x + area.width - 1 { break; }
                buf.get_mut(col, dy + 1).set_char(ch).set_style(
                    Style::default().fg(Color::White)
                );
            }

            let cat_label = format!(" Cat:  {}", binding.category);
            for (i, ch) in cat_label.chars().take(detail_area_w as usize).enumerate() {
                let col = detail_area_x + i as u16;
                if col >= area.x + area.width - 1 { break; }
                buf.get_mut(col, dy + 2).set_char(ch).set_style(category_style);
            }

            // Keys
            for col in detail_area_x..area.x + area.width - 1 {
                buf.get_mut(col, dy + 3).set_char(' ').set_style(bg);
            }
            let keys_hdr = " Keys: ";
            for (i, ch) in keys_hdr.chars().enumerate() {
                let col = detail_area_x + i as u16;
                if col >= area.x + area.width - 1 { break; }
                buf.get_mut(col, dy + 3).set_char(ch).set_style(label_style);
            }
            for (ki, key) in binding.keys.iter().enumerate() {
                let key_badge = format!("[{}] ", key);
                let kx = detail_area_x + keys_hdr.len() as u16 + ki as u16 * 10;
                for (i, ch) in key_badge.chars().take(10).enumerate() {
                    let col = kx + i as u16;
                    if col >= area.x + area.width - 1 { break; }
                    buf.get_mut(col, dy + 3).set_char(ch).set_style(key_style);
                }
            }

            // Description (wrapped)
            let desc_y = dy + 5;
            let desc_label = " Desc: ";
            for (i, ch) in desc_label.chars().enumerate() {
                let col = detail_area_x + i as u16;
                if col >= area.x + area.width - 1 { break; }
                buf.get_mut(col, desc_y).set_char(ch).set_style(label_style);
            }
            let desc_chars: Vec<char> = binding.description.chars().collect();
            let desc_w = (detail_area_w as usize).saturating_sub(desc_label.len() + 1);
            for (line_idx, chunk) in desc_chars.chunks(desc_w.max(1)).enumerate().take(3) {
                let ly = desc_y + line_idx as u16;
                if ly >= area.y + area.height - 2 { break; }
                let off = if line_idx == 0 { desc_label.len() as u16 } else { 0 };
                for (i, ch) in chunk.iter().enumerate() {
                    let col = detail_area_x + off + i as u16;
                    if col >= area.x + area.width - 1 { break; }
                    buf.get_mut(col, ly).set_char(*ch).set_style(normal_style);
                }
            }

            // Capturing mode
            if self.state.capturing {
                let cap_y = desc_y + 4;
                if cap_y < area.y + area.height - 2 {
                    let cap = " ► Press new key... ";
                    for (i, ch) in cap.chars().enumerate() {
                        let col = detail_area_x + i as u16;
                        if col >= area.x + area.width - 1 { break; }
                        buf.get_mut(col, cap_y).set_char(ch).set_style(
                            Style::default().fg(Color::Black).bg(Color::Yellow)
                        );
                    }
                }
            }

            // Conflict warnings
            let binding_idx = self.state.filtered_indices.get(self.state.selected).copied().unwrap_or(0);
            let conflicts: Vec<usize> = self.state.conflicts.iter()
                .filter_map(|(a, b)| {
                    if *a == binding_idx { Some(*b) }
                    else if *b == binding_idx { Some(*a) }
                    else { None }
                })
                .collect();

            if !conflicts.is_empty() {
                let cw_y = desc_y + 5;
                if cw_y < area.y + area.height - 2 {
                    let cw_label = " ⚠ Conflicts:";
                    for (i, ch) in cw_label.chars().enumerate() {
                        let col = detail_area_x + i as u16;
                        if col >= area.x + area.width - 1 { break; }
                        buf.get_mut(col, cw_y).set_char(ch).set_style(conflict_style);
                    }
                    for (ci, &conflict_idx) in conflicts.iter().enumerate().take(3) {
                        let cy = cw_y + 1 + ci as u16;
                        if cy >= area.y + area.height - 2 { break; }
                        if let Some(other) = self.state.bindings.get(conflict_idx) {
                            let ctext: String = format!("  {}", other.command).chars().take(detail_area_w as usize).collect();
                            for (i, ch) in ctext.chars().enumerate() {
                                let col = detail_area_x + i as u16;
                                if col >= area.x + area.width - 1 { break; }
                                buf.get_mut(col, cy).set_char(ch).set_style(conflict_style);
                            }
                        }
                    }
                }
            }
        }

        // Hint bar
        let hint = " [↑↓] navigate  [/] filter  [r] rebind  [d] delete  [Ctrl+s] save  [Esc] close";
        for (i, ch) in hint.chars().enumerate() {
            let col = area.x + 1 + i as u16;
            if col >= area.x + area.width - 1 { break; }
            buf.get_mut(col, area.y + area.height - 2).set_char(ch).set_style(label_style);
        }

        // Conflict count in status
        let conflict_count = self.state.conflicts.len();
        if conflict_count > 0 {
            let cstat = format!(" {} conflicts ", conflict_count);
            let cstat_x = area.x + area.width - 1 - cstat.len() as u16 - 1;
            for (i, ch) in cstat.chars().enumerate() {
                let col = cstat_x + i as u16;
                if col >= area.x + area.width - 1 { break; }
                buf.get_mut(col, area.y + area.height - 2).set_char(ch).set_style(conflict_style);
            }
        }
    }
}
