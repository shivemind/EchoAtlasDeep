#![allow(dead_code, unused_imports, unused_variables)]
//! Prompt library picker widget.
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Widget},
};

use ai::prompt_library::{PromptLibrary, PromptTemplate};

pub struct PromptLibraryState {
    pub open: bool,
    pub library: PromptLibrary,
    pub search_query: String,
    pub selected: usize,
    pub edit_mode: bool,
    pub edit_buffer: String,
}

impl PromptLibraryState {
    pub fn new() -> Self {
        Self {
            open: false,
            library: PromptLibrary::load(None),
            search_query: String::new(),
            selected: 0,
            edit_mode: false,
            edit_buffer: String::new(),
        }
    }

    /// Get the currently filtered list of templates.
    pub fn filtered_templates(&self) -> Vec<&PromptTemplate> {
        if self.search_query.is_empty() {
            self.library.templates.iter().collect()
        } else {
            self.library.search(&self.search_query)
        }
    }

    /// Move selection up.
    pub fn select_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    /// Move selection down.
    pub fn select_down(&mut self) {
        let count = self.filtered_templates().len();
        if count > 0 && self.selected < count - 1 {
            self.selected += 1;
        }
    }

    /// Get the currently selected template.
    pub fn selected_template(&self) -> Option<&PromptTemplate> {
        self.filtered_templates().into_iter().nth(self.selected)
    }
}

impl Default for PromptLibraryState {
    fn default() -> Self {
        Self::new()
    }
}

pub struct PromptLibraryWidget<'a> {
    pub state: &'a PromptLibraryState,
}

impl<'a> Widget for PromptLibraryWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let block = Block::default()
            .title(" Prompt Library ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Magenta));
        let inner = block.inner(area);
        block.render(area, buf);

        if inner.height == 0 || inner.width == 0 {
            return;
        }

        let mut y = inner.y;
        let max_y = inner.y + inner.height;

        // ── Search bar ───────────────────────────────────────────────────────
        if y < max_y {
            let search_label = format!("Search: {}_", self.state.search_query);
            render_text(buf, inner.x, y, inner.width, &search_label,
                Style::default().fg(Color::White));
            y += 1;
        }

        // ── Separator ────────────────────────────────────────────────────────
        if y < max_y {
            for i in 0..inner.width {
                buf[(inner.x + i, y)].set_char('─')
                    .set_style(Style::default().fg(Color::DarkGray));
            }
            y += 1;
        }

        // ── Template list ────────────────────────────────────────────────────
        let templates = self.state.filtered_templates();
        if templates.is_empty() && y < max_y {
            render_text(buf, inner.x, y, inner.width, "  No templates found.",
                Style::default().fg(Color::DarkGray));
            y += 1;
        }

        // Split view: list on left, preview on right
        let list_w = (inner.width / 2).max(20).min(40);
        let preview_x = inner.x + list_w + 1;
        let preview_w = inner.width.saturating_sub(list_w + 2);
        let list_start_y = y;

        // Vertical separator
        for row in y..max_y {
            if inner.x + list_w < inner.right() {
                buf[(inner.x + list_w, row)].set_char('│')
                    .set_style(Style::default().fg(Color::DarkGray));
            }
        }

        // List column
        for (idx, template) in templates.iter().enumerate() {
            if y >= max_y { break; }
            let is_selected = idx == self.state.selected;
            let style = if is_selected {
                Style::default().fg(Color::Black).bg(Color::Magenta)
            } else {
                Style::default().fg(Color::White)
            };
            let tags_str = if template.tags.is_empty() {
                String::new()
            } else {
                format!(" [{}]", template.tags.join(","))
            };
            let label = format!("  {}{}", template.name, tags_str);
            render_text(buf, inner.x, y, list_w, &label, style);
            y += 1;
        }

        // Preview column — show selected template details
        if let Some(template) = self.state.selected_template() {
            let mut py = list_start_y;

            // Name
            if py < max_y && preview_w > 0 {
                render_text(buf, preview_x, py, preview_w,
                    &format!("Name: {}", template.name),
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));
                py += 1;
            }

            // Description
            if py < max_y && preview_w > 0 {
                let desc = format!("Desc: {}", template.description);
                render_text(buf, preview_x, py, preview_w, &desc,
                    Style::default().fg(Color::White));
                py += 1;
            }

            // Variables
            if py < max_y && preview_w > 0 {
                let vars = format!("Vars: {}", template.variables.join(", "));
                render_text(buf, preview_x, py, preview_w, &vars,
                    Style::default().fg(Color::Cyan));
                py += 1;
            }

            // Separator
            if py < max_y && preview_w > 0 {
                for i in 0..preview_w {
                    buf[(preview_x + i, py)].set_char('─')
                        .set_style(Style::default().fg(Color::DarkGray));
                }
                py += 1;
            }

            // Template preview (first few lines)
            for line in template.template.lines() {
                if py >= max_y { break; }
                render_text(buf, preview_x, py, preview_w, line,
                    Style::default().fg(Color::DarkGray));
                py += 1;
            }
        }

        // ── Footer help ──────────────────────────────────────────────────────
        if max_y > inner.y + 2 {
            let help = "Enter:use  Tab:edit  Esc:close";
            render_text(buf, inner.x, max_y - 1, inner.width, help,
                Style::default().fg(Color::DarkGray));
        }
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────────

fn render_text(buf: &mut Buffer, x: u16, y: u16, width: u16, text: &str, style: Style) {
    let chars: Vec<char> = text.chars().collect();
    for (i, &ch) in chars.iter().enumerate() {
        let cx = x + i as u16;
        if cx >= x + width { break; }
        buf[(cx, y)].set_char(ch).set_style(style);
    }
}
