#![allow(dead_code, unused_imports, unused_variables)]
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::Widget,
};

#[derive(Clone, Copy, PartialEq)]
pub enum DbPanelTab {
    Schema,
    Query,
    Results,
}

pub struct DbPanelState {
    pub open: bool,
    pub tab: DbPanelTab,
    pub selected_connection: usize,
    pub tables: Vec<String>,
    pub selected_table: usize,
    pub query_buf: String,
    pub last_result: Option<runner::DbResult>,
    pub result_scroll: usize,
    pub query_history_idx: Option<usize>,
}

impl DbPanelState {
    pub fn new() -> Self {
        Self {
            open: false,
            tab: DbPanelTab::Schema,
            selected_connection: 0,
            tables: Vec::new(),
            selected_table: 0,
            query_buf: String::new(),
            last_result: None,
            result_scroll: 0,
            query_history_idx: None,
        }
    }
}

pub struct DbPanelWidget<'a> {
    pub state: &'a DbPanelState,
    pub client: &'a runner::DbClient,
}

impl<'a> Widget for DbPanelWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let connections = self.client.connections.read();

        // Background
        let bg_style = Style::default().bg(Color::Rgb(14, 12, 22));
        for y in area.y..area.y + area.height {
            for x in area.x..area.x + area.width {
                buf.get_mut(x, y).set_char(' ').set_style(bg_style);
            }
        }

        if area.height < 5 || area.width < 30 {
            return;
        }

        // Border
        let border_style = Style::default().fg(Color::Magenta);
        buf.get_mut(area.x, area.y).set_char('┌').set_style(border_style);
        buf.get_mut(area.x + area.width - 1, area.y).set_char('┐').set_style(border_style);
        buf.get_mut(area.x, area.y + area.height - 1).set_char('└').set_style(border_style);
        buf.get_mut(area.x + area.width - 1, area.y + area.height - 1).set_char('┘').set_style(border_style);
        for x in (area.x + 1)..(area.x + area.width - 1) {
            buf.get_mut(x, area.y).set_char('─').set_style(border_style);
            buf.get_mut(x, area.y + area.height - 1).set_char('─').set_style(border_style);
        }
        for y in (area.y + 1)..(area.y + area.height - 1) {
            buf.get_mut(area.x, y).set_char('│').set_style(border_style);
            buf.get_mut(area.x + area.width - 1, y).set_char('│').set_style(border_style);
        }

        // Tab bar
        let tabs = [" Schema ", " Query ", " Results "];
        let tab_y = area.y;
        let mut tab_x = area.x + 2;
        for (i, tab_label) in tabs.iter().enumerate() {
            let current_tab = match i {
                0 => DbPanelTab::Schema,
                1 => DbPanelTab::Query,
                2 => DbPanelTab::Results,
                _ => DbPanelTab::Schema,
            };
            let is_active = self.state.tab == current_tab;
            let style = if is_active {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Magenta)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::DarkGray)
            };
            for ch in tab_label.chars() {
                if tab_x >= area.x + area.width - 1 {
                    break;
                }
                buf.get_mut(tab_x, tab_y).set_char(ch).set_style(style);
                tab_x += 1;
            }
        }

        let content_area = Rect {
            x: area.x + 1,
            y: area.y + 1,
            width: area.width.saturating_sub(2),
            height: area.height.saturating_sub(3),
        };

        // Connection selector (always visible at top of content)
        if content_area.height > 0 {
            let conn_label = if connections.is_empty() {
                " No connections — add one with :DB connect ".to_string()
            } else {
                let active = connections.get(self.state.selected_connection);
                if let Some(conn) = active {
                    format!(
                        " [{}] {} ({}) ",
                        self.state.selected_connection + 1,
                        conn.name,
                        conn.kind.label()
                    )
                } else {
                    " (no connection selected) ".to_string()
                }
            };
            let conn_style = Style::default().fg(Color::Cyan);
            for (i, ch) in conn_label.chars().enumerate() {
                let x = content_area.x + i as u16;
                if x >= content_area.x + content_area.width {
                    break;
                }
                buf.get_mut(x, content_area.y)
                    .set_char(ch)
                    .set_style(conn_style);
            }
        }

        let body_area = Rect {
            y: content_area.y + 1,
            height: content_area.height.saturating_sub(1),
            ..content_area
        };

        match self.state.tab {
            DbPanelTab::Schema => self.render_schema(body_area, buf),
            DbPanelTab::Query => self.render_query(body_area, buf),
            DbPanelTab::Results => self.render_results(body_area, buf),
        }

        // Key hints
        let hints = " [1]=Schema  [2]=Query  [3]=Results  [Enter]=Run Query  [c]=Connect  [q]=Close ";
        let hints_y = area.y + area.height - 1;
        for (i, ch) in hints.chars().enumerate() {
            let x = area.x + 1 + i as u16;
            if x >= area.x + area.width - 1 {
                break;
            }
            buf.get_mut(x, hints_y)
                .set_char(ch)
                .set_style(Style::default().fg(Color::DarkGray));
        }
    }
}

impl<'a> DbPanelWidget<'a> {
    fn render_schema(&self, area: Rect, buf: &mut Buffer) {
        if self.state.tables.is_empty() {
            let msg = " No tables loaded — press [Enter] on Schema tab to fetch ";
            for (i, ch) in msg.chars().enumerate() {
                let x = area.x + i as u16;
                if x >= area.x + area.width {
                    break;
                }
                buf.get_mut(x, area.y)
                    .set_char(ch)
                    .set_style(Style::default().fg(Color::DarkGray));
            }
            return;
        }

        let label = format!(" Tables ({}) ", self.state.tables.len());
        let label_style = Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD);
        for (i, ch) in label.chars().enumerate() {
            let x = area.x + i as u16;
            if x >= area.x + area.width {
                break;
            }
            buf.get_mut(x, area.y).set_char(ch).set_style(label_style);
        }

        for (i, table_name) in self.state.tables.iter().enumerate() {
            let row_y = area.y + 1 + i as u16;
            if row_y >= area.y + area.height {
                break;
            }
            let is_sel = i == self.state.selected_table;
            let bg = if is_sel { Color::Rgb(25, 15, 35) } else { Color::Reset };
            let style = Style::default().bg(bg).fg(if is_sel { Color::Magenta } else { Color::White });
            let row = format!("  {}", table_name);
            for (j, ch) in row.chars().enumerate() {
                let x = area.x + j as u16;
                if x >= area.x + area.width {
                    break;
                }
                buf.get_mut(x, row_y).set_char(ch).set_style(style);
            }
        }
    }

    fn render_query(&self, area: Rect, buf: &mut Buffer) {
        // Query buffer display
        let query_label = " SQL Query: ";
        let label_style = Style::default().fg(Color::Yellow);
        for (i, ch) in query_label.chars().enumerate() {
            let x = area.x + i as u16;
            if x >= area.x + area.width {
                break;
            }
            buf.get_mut(x, area.y).set_char(ch).set_style(label_style);
        }

        let query_bg = Style::default().bg(Color::Rgb(18, 18, 30)).fg(Color::White);
        let query_area_y = area.y + 1;
        let query_height = (area.height / 3).max(3).min(10);

        for row in 0..query_height {
            let y = query_area_y + row;
            if y >= area.y + area.height {
                break;
            }
            for x in area.x..(area.x + area.width) {
                buf.get_mut(x, y).set_char(' ').set_style(query_bg);
            }
        }

        // Render query lines
        for (line_idx, line) in self.state.query_buf.lines().enumerate() {
            let y = query_area_y + line_idx as u16;
            if y >= query_area_y + query_height {
                break;
            }
            for (j, ch) in line.chars().enumerate() {
                let x = area.x + j as u16;
                if x >= area.x + area.width {
                    break;
                }
                buf.get_mut(x, y).set_char(ch).set_style(query_bg);
            }
        }

        // Cursor position
        let cursor_y = query_area_y + self.state.query_buf.lines().count() as u16;
        if cursor_y < query_area_y + query_height && cursor_y < area.y + area.height {
            let cursor_x = area.x + self.state.query_buf.lines().last().map(|l| l.len() as u16).unwrap_or(0);
            if cursor_x < area.x + area.width {
                buf.get_mut(cursor_x, cursor_y)
                    .set_char('_')
                    .set_style(Style::default().fg(Color::White).add_modifier(Modifier::SLOW_BLINK));
            }
        }

        // Hint
        let hint_y = query_area_y + query_height + 1;
        if hint_y < area.y + area.height {
            let hint = " Press Ctrl+Enter or [Enter] in normal mode to execute ";
            for (i, ch) in hint.chars().enumerate() {
                let x = area.x + i as u16;
                if x >= area.x + area.width {
                    break;
                }
                buf.get_mut(x, hint_y)
                    .set_char(ch)
                    .set_style(Style::default().fg(Color::DarkGray));
            }
        }
    }

    fn render_results(&self, area: Rect, buf: &mut Buffer) {
        let Some(result) = &self.state.last_result else {
            let msg = " No results yet — run a query first ";
            for (i, ch) in msg.chars().enumerate() {
                let x = area.x + i as u16;
                if x >= area.x + area.width {
                    break;
                }
                buf.get_mut(x, area.y)
                    .set_char(ch)
                    .set_style(Style::default().fg(Color::DarkGray));
            }
            return;
        };

        if let Some(ref err) = result.error {
            let err_line = format!(" Error: {} ", err);
            for (i, ch) in err_line.chars().enumerate() {
                let x = area.x + i as u16;
                if x >= area.x + area.width {
                    break;
                }
                buf.get_mut(x, area.y)
                    .set_char(ch)
                    .set_style(Style::default().fg(Color::Red));
            }
            return;
        }

        // Result info
        let info = format!(
            " {} rows × {} cols — {}ms ",
            result.rows.len(),
            result.columns.len(),
            result.duration_ms
        );
        for (i, ch) in info.chars().enumerate() {
            let x = area.x + i as u16;
            if x >= area.x + area.width {
                break;
            }
            buf.get_mut(x, area.y)
                .set_char(ch)
                .set_style(Style::default().fg(Color::Green));
        }

        if result.columns.is_empty() && result.rows.is_empty() {
            return;
        }

        // Calculate column widths
        let col_count = result.columns.len().max(
            result.rows.first().map(|r| r.len()).unwrap_or(0)
        );
        let col_w = if col_count > 0 {
            (area.width as usize / col_count).max(8).min(30)
        } else {
            15
        };

        // Header row
        let header_y = area.y + 1;
        if header_y < area.y + area.height {
            let header_style = Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD);
            for (j, col_name) in result.columns.iter().enumerate() {
                let x = area.x + (j * col_w) as u16;
                if x >= area.x + area.width {
                    break;
                }
                let col_str: String = col_name.chars().take(col_w - 1).collect();
                let padded = format!("{:<width$}", col_str, width = col_w);
                for (k, ch) in padded.chars().enumerate() {
                    let cx = x + k as u16;
                    if cx >= area.x + area.width {
                        break;
                    }
                    buf.get_mut(cx, header_y).set_char(ch).set_style(header_style);
                }
            }
        }

        // Separator
        let sep_y = area.y + 2;
        if sep_y < area.y + area.height {
            for x in area.x..(area.x + area.width) {
                buf.get_mut(x, sep_y)
                    .set_char('─')
                    .set_style(Style::default().fg(Color::DarkGray));
            }
        }

        // Data rows
        let data_y_start = area.y + 3;
        let scroll = self.state.result_scroll;
        for (row_idx, row) in result.rows.iter().skip(scroll).enumerate() {
            let row_y = data_y_start + row_idx as u16;
            if row_y >= area.y + area.height {
                break;
            }
            let row_bg = if row_idx % 2 == 0 { Color::Reset } else { Color::Rgb(18, 18, 28) };
            let row_style = Style::default().bg(row_bg).fg(Color::White);

            for (j, cell) in row.iter().enumerate() {
                let x = area.x + (j * col_w) as u16;
                if x >= area.x + area.width {
                    break;
                }
                let cell_str: String = cell.chars().take(col_w - 1).collect();
                let padded = format!("{:<width$}", cell_str, width = col_w);
                for (k, ch) in padded.chars().enumerate() {
                    let cx = x + k as u16;
                    if cx >= area.x + area.width {
                        break;
                    }
                    buf.get_mut(cx, row_y).set_char(ch).set_style(row_style);
                }
            }
        }
    }
}
