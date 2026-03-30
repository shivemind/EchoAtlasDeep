#![allow(dead_code, unused_imports, unused_variables)]
//! Plugin marketplace TUI — Phase 12 Point 47.
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::Widget,
};

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct MarketplaceEntry {
    pub name: String,
    pub description: String,
    pub author: String,
    pub version: String,
    pub kind: String,
    pub stars: u32,
    pub installed: bool,
    pub url: String,
}

pub struct PluginMarketplaceState {
    pub open: bool,
    pub entries: Vec<MarketplaceEntry>,
    pub filtered: Vec<usize>,
    pub selected: usize,
    pub query: String,
    pub loading: bool,
    pub install_log: Vec<String>,
}

impl PluginMarketplaceState {
    pub fn new() -> Self {
        let mut s = Self {
            open: false,
            entries: Vec::new(),
            filtered: Vec::new(),
            selected: 0,
            query: String::new(),
            loading: false,
            install_log: Vec::new(),
        };
        s.load_builtin_registry();
        s
    }

    pub fn load_builtin_registry(&mut self) {
        self.entries = vec![
            MarketplaceEntry {
                name: "copilot-bridge".into(),
                description: "GitHub Copilot API bridge for rmtide. Provides completion suggestions from Copilot's model.".into(),
                author: "rmtide-org".into(),
                version: "0.3.1".into(),
                kind: "wasm".into(),
                stars: 1240,
                installed: false,
                url: "https://plugins.rmtide.dev/copilot-bridge".into(),
            },
            MarketplaceEntry {
                name: "rust-analyzer-extras".into(),
                description: "Extra rust-analyzer integrations: proc-macro expansion viewer, cargo-expand, and workspace symbols sidebar.".into(),
                author: "ferris".into(),
                version: "0.2.0".into(),
                kind: "lua".into(),
                stars: 890,
                installed: false,
                url: "https://plugins.rmtide.dev/rust-analyzer-extras".into(),
            },
            MarketplaceEntry {
                name: "prettier-format".into(),
                description: "Run Prettier on save for JS/TS/CSS/HTML/JSON/Markdown files. Configurable via .prettierrc.".into(),
                author: "prettier-team".into(),
                version: "1.0.4".into(),
                kind: "lua".into(),
                stars: 2100,
                installed: false,
                url: "https://plugins.rmtide.dev/prettier-format".into(),
            },
            MarketplaceEntry {
                name: "git-graph".into(),
                description: "Interactive git commit graph with branch visualization, cherry-pick, and rebase support.".into(),
                author: "mitsuhiko".into(),
                version: "0.5.2".into(),
                kind: "wasm".into(),
                stars: 1560,
                installed: false,
                url: "https://plugins.rmtide.dev/git-graph".into(),
            },
            MarketplaceEntry {
                name: "color-picker".into(),
                description: "Inline color picker for CSS, Tailwind, and SVG. Shows color swatches in the gutter.".into(),
                author: "color-tools".into(),
                version: "0.1.8".into(),
                kind: "lua".into(),
                stars: 430,
                installed: false,
                url: "https://plugins.rmtide.dev/color-picker".into(),
            },
            MarketplaceEntry {
                name: "lorem-ipsum".into(),
                description: "Generate Lorem Ipsum placeholder text. Commands: :Lorem, :LoremWords N, :LoremParagraphs N.".into(),
                author: "devtools-io".into(),
                version: "0.0.5".into(),
                kind: "lua".into(),
                stars: 210,
                installed: false,
                url: "https://plugins.rmtide.dev/lorem-ipsum".into(),
            },
            MarketplaceEntry {
                name: "csv-viewer".into(),
                description: "Render CSV/TSV files as aligned tables with sorting, filtering, and column stats.".into(),
                author: "data-lab".into(),
                version: "0.4.0".into(),
                kind: "wasm".into(),
                stars: 780,
                installed: false,
                url: "https://plugins.rmtide.dev/csv-viewer".into(),
            },
            MarketplaceEntry {
                name: "markdown-preview".into(),
                description: "Live Markdown preview in a side pane. Renders headings, code blocks, tables, and images.".into(),
                author: "md-tools".into(),
                version: "1.2.1".into(),
                kind: "wasm".into(),
                stars: 1890,
                installed: false,
                url: "https://plugins.rmtide.dev/markdown-preview".into(),
            },
        ];
        self.apply_filter();
    }

    pub fn filter(&mut self) {
        self.apply_filter();
    }

    fn apply_filter(&mut self) {
        if self.query.is_empty() {
            self.filtered = (0..self.entries.len()).collect();
        } else {
            let q = self.query.to_lowercase();
            self.filtered = (0..self.entries.len())
                .filter(|&i| {
                    let e = &self.entries[i];
                    e.name.to_lowercase().contains(&q)
                        || e.description.to_lowercase().contains(&q)
                        || e.author.to_lowercase().contains(&q)
                        || e.kind.to_lowercase().contains(&q)
                })
                .collect();
        }
        if self.selected >= self.filtered.len() && !self.filtered.is_empty() {
            self.selected = self.filtered.len() - 1;
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

    pub fn selected_entry(&self) -> Option<&MarketplaceEntry> {
        self.filtered.get(self.selected).and_then(|&i| self.entries.get(i))
    }
}

impl Default for PluginMarketplaceState {
    fn default() -> Self {
        Self::new()
    }
}

pub struct PluginMarketplaceWidget<'a> {
    pub state: &'a PluginMarketplaceState,
}

impl Widget for PluginMarketplaceWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 10 || area.height < 8 {
            return;
        }

        let w = (area.width * 9 / 10).max(80).min(area.width);
        let h = (area.height * 9 / 10).max(24).min(area.height);
        let x = area.x + (area.width.saturating_sub(w)) / 2;
        let y = area.y + (area.height.saturating_sub(h)) / 2;

        let bg = Style::default().bg(Color::Rgb(14, 16, 26));
        let border_style = Style::default().fg(Color::Cyan);
        let title_style = Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD);
        let label_style = Style::default().fg(Color::DarkGray);
        let selected_style = Style::default().bg(Color::Rgb(20, 30, 50)).fg(Color::White);
        let normal_style = Style::default().fg(Color::Gray);
        let installed_style = Style::default().fg(Color::Green);
        let wasm_style = Style::default().fg(Color::Blue);
        let lua_style = Style::default().fg(Color::Yellow);

        // Fill background
        for row in y..y + h {
            for col in x..x + w {
                buf.get_mut(col, row).set_char(' ').set_style(bg);
            }
        }

        // Border
        buf.get_mut(x, y).set_char('╔').set_style(border_style);
        buf.get_mut(x + w - 1, y).set_char('╗').set_style(border_style);
        for col in x + 1..x + w - 1 {
            buf.get_mut(col, y).set_char('═').set_style(border_style);
        }
        buf.get_mut(x, y + h - 1).set_char('╚').set_style(border_style);
        buf.get_mut(x + w - 1, y + h - 1).set_char('╝').set_style(border_style);
        for col in x + 1..x + w - 1 {
            buf.get_mut(col, y + h - 1).set_char('═').set_style(border_style);
        }
        for row in y + 1..y + h - 1 {
            buf.get_mut(x, row).set_char('║').set_style(border_style);
            buf.get_mut(x + w - 1, row).set_char('║').set_style(border_style);
        }

        // Title
        let title = " Plugin Marketplace ";
        let title_x = x + (w.saturating_sub(title.len() as u16)) / 2;
        for (i, ch) in title.chars().enumerate() {
            if title_x + i as u16 >= x + w - 1 { break; }
            buf.get_mut(title_x + i as u16, y).set_char(ch).set_style(title_style);
        }

        // Search bar (row y+1)
        let search_prompt = " Search: ";
        for (i, ch) in search_prompt.chars().enumerate() {
            let col = x + 1 + i as u16;
            if col >= x + w - 1 { break; }
            buf.get_mut(col, y + 1).set_char(ch).set_style(label_style);
        }
        let query_style = Style::default().fg(Color::White).bg(Color::Rgb(22, 26, 38));
        for col in x + 1 + search_prompt.len() as u16..x + w - 1 {
            buf.get_mut(col, y + 1).set_char(' ').set_style(query_style);
        }
        for (i, ch) in self.state.query.chars().take((w as usize).saturating_sub(12)).enumerate() {
            let col = x + 1 + search_prompt.len() as u16 + i as u16;
            if col >= x + w - 1 { break; }
            buf.get_mut(col, y + 1).set_char(ch).set_style(query_style);
        }

        // Separator
        for col in x + 1..x + w - 1 {
            buf.get_mut(col, y + 2).set_char('─').set_style(border_style);
        }
        buf.get_mut(x, y + 2).set_char('╟').set_style(border_style);
        buf.get_mut(x + w - 1, y + 2).set_char('╢').set_style(border_style);

        // Split: left list (55%), right details (45%)
        let list_w = w * 55 / 100;
        let detail_x = x + list_w;

        // Vertical divider
        for row in y + 2..y + h - 1 {
            buf.get_mut(detail_x, row).set_char('│').set_style(border_style);
        }

        // Loading indicator
        if self.state.loading {
            let loading = " ⠙ Loading registry...";
            for (i, ch) in loading.chars().enumerate() {
                let col = x + 1 + i as u16;
                if col >= detail_x { break; }
                buf.get_mut(col, y + 3).set_char(ch).set_style(Style::default().fg(Color::Yellow));
            }
        }

        // Plugin list
        let list_start = y + 3;
        let list_height = (h as usize).saturating_sub(6);
        let scroll = if self.state.selected >= list_height {
            self.state.selected - list_height + 1
        } else {
            0
        };

        for (view_idx, &entry_idx) in self.state.filtered.iter().enumerate().skip(scroll).take(list_height) {
            let ry = list_start + (view_idx - scroll) as u16;
            if ry >= y + h - 2 { break; }

            let entry = &self.state.entries[entry_idx];
            let is_sel = view_idx == self.state.selected;
            let row_style = if is_sel { selected_style } else { normal_style };

            for col in x + 1..detail_x {
                buf.get_mut(col, ry).set_char(' ').set_style(row_style);
            }

            // Installed badge
            let installed_badge = if entry.installed { "✓ " } else { "  " };
            let badge_style = if entry.installed {
                Style::default().fg(Color::Green).bg(if is_sel { Color::Rgb(20,30,50) } else { Color::Reset })
            } else {
                row_style
            };
            for (i, ch) in installed_badge.chars().enumerate() {
                let col = x + 1 + i as u16;
                if col >= detail_x { break; }
                buf.get_mut(col, ry).set_char(ch).set_style(badge_style);
            }

            // Kind badge
            let kind_badge = format!("[{}]", entry.kind.to_uppercase());
            let kind_style_here = if entry.kind == "wasm" {
                Style::default().fg(Color::Blue).bg(if is_sel { Color::Rgb(20,30,50) } else { Color::Reset })
            } else {
                Style::default().fg(Color::Yellow).bg(if is_sel { Color::Rgb(20,30,50) } else { Color::Reset })
            };
            for (i, ch) in kind_badge.chars().enumerate() {
                let col = x + 3 + i as u16;
                if col >= detail_x { break; }
                buf.get_mut(col, ry).set_char(ch).set_style(kind_style_here);
            }

            // Name
            let name_x = x + 9;
            let name_w = (list_w as usize).saturating_sub(30);
            for (i, ch) in entry.name.chars().take(name_w).enumerate() {
                let col = name_x + i as u16;
                if col >= detail_x { break; }
                buf.get_mut(col, ry).set_char(ch).set_style(
                    Style::default().fg(if is_sel { Color::White } else { Color::LightCyan })
                        .bg(if is_sel { Color::Rgb(20,30,50) } else { Color::Reset })
                );
            }

            // Stars
            let stars_str = format!("★{}", entry.stars);
            let stars_x = detail_x.saturating_sub(stars_str.len() as u16 + 1);
            for (i, ch) in stars_str.chars().enumerate() {
                let col = stars_x + i as u16;
                if col >= detail_x { break; }
                buf.get_mut(col, ry).set_char(ch).set_style(
                    Style::default().fg(Color::Yellow).bg(if is_sel { Color::Rgb(20,30,50) } else { Color::Reset })
                );
            }
        }

        // Right detail panel
        let detail_area_x = detail_x + 1;
        let detail_area_w = w.saturating_sub(list_w + 2);

        if let Some(entry) = self.state.selected_entry() {
            let dy = y + 3;

            // Plugin name
            let name_style = Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD);
            for (i, ch) in entry.name.chars().take(detail_area_w as usize).enumerate() {
                let col = detail_area_x + 1 + i as u16;
                if col >= x + w - 1 { break; }
                buf.get_mut(col, dy).set_char(ch).set_style(name_style);
            }

            // Author & version
            let meta = format!(" by {} — v{}", entry.author, entry.version);
            for (i, ch) in meta.chars().take(detail_area_w as usize).enumerate() {
                let col = detail_area_x + i as u16;
                if col >= x + w - 1 { break; }
                buf.get_mut(col, dy + 1).set_char(ch).set_style(label_style);
            }

            // Kind + Stars
            let kind_line = format!(" {} — ★{} stars", entry.kind.to_uppercase(), entry.stars);
            for (i, ch) in kind_line.chars().take(detail_area_w as usize).enumerate() {
                let col = detail_area_x + i as u16;
                if col >= x + w - 1 { break; }
                buf.get_mut(col, dy + 2).set_char(ch).set_style(
                    if entry.kind == "wasm" { wasm_style } else { lua_style }
                );
            }

            // Separator
            for col in detail_area_x..x + w - 1 {
                buf.get_mut(col, dy + 3).set_char('─').set_style(label_style);
            }

            // Description wrapped
            let desc_chars: Vec<char> = entry.description.chars().collect();
            let desc_w = (detail_area_w as usize).saturating_sub(2);
            for (chunk_idx, chunk) in desc_chars.chunks(desc_w.max(1)).enumerate().take(5) {
                let ly = dy + 4 + chunk_idx as u16;
                if ly >= y + h - 4 { break; }
                for (i, ch) in chunk.iter().enumerate() {
                    let col = detail_area_x + 1 + i as u16;
                    if col >= x + w - 1 { break; }
                    buf.get_mut(col, ly).set_char(*ch).set_style(normal_style);
                }
            }

            // URL
            let url_y = y + h - 5;
            if url_y > dy + 8 {
                let url_display: String = entry.url.chars().take(detail_area_w as usize).collect();
                for (i, ch) in url_display.chars().enumerate() {
                    let col = detail_area_x + i as u16;
                    if col >= x + w - 1 { break; }
                    buf.get_mut(col, url_y).set_char(ch).set_style(
                        Style::default().fg(Color::Blue).add_modifier(Modifier::UNDERLINED)
                    );
                }
            }

            // Install status
            let status_y = y + h - 4;
            if status_y > dy + 4 {
                let (status_str, status_style) = if entry.installed {
                    (" ✓ Installed  [u] update  [x] uninstall", Style::default().fg(Color::Green))
                } else {
                    (" [Enter] Install plugin", Style::default().fg(Color::Cyan))
                };
                for (i, ch) in status_str.chars().take(detail_area_w as usize).enumerate() {
                    let col = detail_area_x + i as u16;
                    if col >= x + w - 1 { break; }
                    buf.get_mut(col, status_y).set_char(ch).set_style(status_style);
                }
            }
        }

        // Install log
        if !self.state.install_log.is_empty() {
            let log_start = y + h - 3;
            for col in x + 1..x + w - 1 {
                buf.get_mut(col, log_start).set_char('─').set_style(border_style);
            }
            buf.get_mut(x, log_start).set_char('╟').set_style(border_style);
            buf.get_mut(x + w - 1, log_start).set_char('╢').set_style(border_style);

            if let Some(last_log) = self.state.install_log.last() {
                let display: String = last_log.chars().take((w as usize).saturating_sub(4)).collect();
                for (i, ch) in display.chars().enumerate() {
                    let col = x + 2 + i as u16;
                    if col >= x + w - 1 { break; }
                    buf.get_mut(col, log_start + 1).set_char(ch).set_style(
                        Style::default().fg(Color::DarkGray)
                    );
                }
            }
        }

        // Hint bar
        let hint = " [↑↓] navigate  [/] search  [Enter] install  [Esc] close";
        for (i, ch) in hint.chars().enumerate() {
            let col = x + 1 + i as u16;
            if col >= x + w - 1 { break; }
            buf.get_mut(col, y + h - 2).set_char(ch).set_style(label_style);
        }

        // Count info
        let count_str = format!(" {}/{} plugins", self.state.filtered.len(), self.state.entries.len());
        let count_x = x + w - 1 - count_str.len() as u16 - 1;
        for (i, ch) in count_str.chars().enumerate() {
            let col = count_x + i as u16;
            if col >= x + w - 1 { break; }
            buf.get_mut(col, y + h - 2).set_char(ch).set_style(label_style);
        }
    }
}
