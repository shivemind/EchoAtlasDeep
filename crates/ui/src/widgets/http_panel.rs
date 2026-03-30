#![allow(dead_code, unused_imports, unused_variables)]
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::Widget,
};

#[derive(Clone, Copy, PartialEq)]
pub enum HttpPanelTab {
    Collections,
    Request,
    Response,
}

#[derive(Clone, Copy, PartialEq)]
pub enum HttpRequestField {
    Url,
    Body,
    Headers,
}

pub struct HttpPanelState {
    pub open: bool,
    pub tab: HttpPanelTab,
    pub selected_collection: usize,
    pub selected_request: usize,
    pub current_request: runner::HttpRequest,
    pub last_response: Option<runner::HttpResponse>,
    pub active_field: HttpRequestField,
    pub sending: bool,
}

impl HttpPanelState {
    pub fn new() -> Self {
        Self {
            open: false,
            tab: HttpPanelTab::Collections,
            selected_collection: 0,
            selected_request: 0,
            current_request: runner::HttpRequest::default(),
            last_response: None,
            active_field: HttpRequestField::Url,
            sending: false,
        }
    }
}

pub struct HttpPanelWidget<'a> {
    pub state: &'a HttpPanelState,
    pub client: &'a runner::HttpClient,
}

impl<'a> Widget for HttpPanelWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let collections = self.client.get_collections();

        // Background
        let bg_style = Style::default().bg(Color::Rgb(12, 16, 22));
        for y in area.y..area.y + area.height {
            for x in area.x..area.x + area.width {
                buf.get_mut(x, y).set_char(' ').set_style(bg_style);
            }
        }

        if area.height < 5 || area.width < 30 {
            return;
        }

        // Border
        let border_style = Style::default().fg(Color::Blue);
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
        let tabs = [" Collections ", " Request ", " Response "];
        let tab_y = area.y;
        let mut tab_x = area.x + 2;
        for (i, tab_label) in tabs.iter().enumerate() {
            let current_tab = match i {
                0 => HttpPanelTab::Collections,
                1 => HttpPanelTab::Request,
                2 => HttpPanelTab::Response,
                _ => HttpPanelTab::Collections,
            };
            let is_active = self.state.tab == current_tab;
            let style = if is_active {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Blue)
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

        // Sending indicator
        if self.state.sending {
            let indicator = " [SENDING...] ";
            let ind_x = area.x + area.width.saturating_sub(indicator.len() as u16 + 2);
            for (i, ch) in indicator.chars().enumerate() {
                let x = ind_x + i as u16;
                if x >= area.x + area.width - 1 {
                    break;
                }
                buf.get_mut(x, tab_y)
                    .set_char(ch)
                    .set_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));
            }
        }

        let content_area = Rect {
            x: area.x + 1,
            y: area.y + 1,
            width: area.width.saturating_sub(2),
            height: area.height.saturating_sub(3),
        };

        match self.state.tab {
            HttpPanelTab::Collections => {
                self.render_collections(content_area, buf, &collections);
            }
            HttpPanelTab::Request => {
                self.render_request(content_area, buf);
            }
            HttpPanelTab::Response => {
                self.render_response(content_area, buf);
            }
        }

        // Key hints
        let hints = " [1]=Collections  [2]=Request  [3]=Response  [Enter]=Send  [s]=Save  [q]=Close ";
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

impl<'a> HttpPanelWidget<'a> {
    fn render_collections(
        &self,
        area: Rect,
        buf: &mut Buffer,
        collections: &[runner::HttpCollection],
    ) {
        let left_w = (area.width / 3).max(15);
        let divider_x = area.x + left_w;

        // Divider
        for y in area.y..(area.y + area.height) {
            buf.get_mut(divider_x, y)
                .set_char('│')
                .set_style(Style::default().fg(Color::DarkGray));
        }

        // Collections list
        if collections.is_empty() {
            let msg = " No collections ";
            for (i, ch) in msg.chars().enumerate() {
                let x = area.x + i as u16;
                if x >= divider_x {
                    break;
                }
                buf.get_mut(x, area.y)
                    .set_char(ch)
                    .set_style(Style::default().fg(Color::DarkGray));
            }
        } else {
            for (i, col) in collections.iter().enumerate() {
                let col_y = area.y + (i * 2) as u16;
                if col_y >= area.y + area.height {
                    break;
                }
                let is_sel = i == self.state.selected_collection;
                let bg = if is_sel { Color::Rgb(20, 20, 40) } else { Color::Reset };
                let style = Style::default().bg(bg).fg(if is_sel { Color::Blue } else { Color::Gray });
                let col_name: String = col.name.chars().take((left_w - 2) as usize).collect();
                for (j, ch) in col_name.chars().enumerate() {
                    let x = area.x + j as u16;
                    if x >= divider_x {
                        break;
                    }
                    buf.get_mut(x, col_y).set_char(ch).set_style(style);
                }

                // Requests in this collection
                for (ri, req) in col.requests.iter().enumerate() {
                    let req_y = col_y + 1 + ri as u16;
                    if req_y >= area.y + area.height {
                        break;
                    }
                    let is_req_sel = is_sel && ri == self.state.selected_request;
                    let req_bg = if is_req_sel { Color::Rgb(25, 25, 50) } else { Color::Reset };
                    let req_text = format!("  {} {}", req.method, req.name.chars().take(20).collect::<String>());
                    let method_color = method_color(&req.method);
                    for (j, ch) in req_text.chars().enumerate() {
                        let x = area.x + j as u16;
                        if x >= divider_x {
                            break;
                        }
                        let style = Style::default().bg(req_bg).fg(if j < 5 { method_color } else { Color::White });
                        buf.get_mut(x, req_y).set_char(ch).set_style(style);
                    }
                }
            }
        }

        // Right: selected request preview
        let right_x = divider_x + 1;
        let right_w = area.width.saturating_sub(left_w + 1);
        if let Some(col) = collections.get(self.state.selected_collection) {
            if let Some(req) = col.requests.get(self.state.selected_request) {
                let method_str = format!("{} {}", req.method, req.url);
                let style = Style::default().fg(Color::White).add_modifier(Modifier::BOLD);
                for (i, ch) in method_str.chars().enumerate() {
                    let x = right_x + i as u16;
                    if x >= right_x + right_w {
                        break;
                    }
                    buf.get_mut(x, area.y).set_char(ch).set_style(style);
                }
            }
        }
    }

    fn render_request(&self, area: Rect, buf: &mut Buffer) {
        let req = &self.state.current_request;

        // Method + URL row
        let method_str = format!("{}", req.method);
        let method_color_val = method_color(&req.method);
        let is_url_active = self.state.active_field == HttpRequestField::Url;

        let method_label = format!("[{}]", method_str);
        let method_style = Style::default().fg(method_color_val).add_modifier(Modifier::BOLD);
        for (i, ch) in method_label.chars().enumerate() {
            let x = area.x + i as u16;
            if x >= area.x + area.width {
                break;
            }
            buf.get_mut(x, area.y).set_char(ch).set_style(method_style);
        }

        let url_x = area.x + method_label.len() as u16 + 1;
        let url_style = if is_url_active {
            Style::default().fg(Color::White).bg(Color::Rgb(20, 20, 40))
        } else {
            Style::default().fg(Color::White)
        };
        for (i, ch) in req.url.chars().enumerate() {
            let x = url_x + i as u16;
            if x >= area.x + area.width {
                break;
            }
            buf.get_mut(x, area.y).set_char(ch).set_style(url_style);
        }

        // Headers
        let headers_y = area.y + 2;
        if headers_y < area.y + area.height {
            let header_label = "Headers:";
            let h_style = Style::default().fg(Color::Yellow);
            for (i, ch) in header_label.chars().enumerate() {
                let x = area.x + i as u16;
                if x >= area.x + area.width {
                    break;
                }
                buf.get_mut(x, headers_y).set_char(ch).set_style(h_style);
            }
            for (j, (name, value)) in req.headers.iter().enumerate() {
                let h_y = headers_y + 1 + j as u16;
                if h_y >= area.y + area.height {
                    break;
                }
                let header_line = format!("  {}: {}", name, value);
                let style = Style::default().fg(Color::Gray);
                for (i, ch) in header_line.chars().enumerate() {
                    let x = area.x + i as u16;
                    if x >= area.x + area.width {
                        break;
                    }
                    buf.get_mut(x, h_y).set_char(ch).set_style(style);
                }
            }
        }

        // Body
        let body_offset = 3 + req.headers.len() as u16;
        let body_y = area.y + body_offset;
        if body_y < area.y + area.height {
            let is_body_active = self.state.active_field == HttpRequestField::Body;
            let body_label = "Body:";
            let bl_style = Style::default().fg(Color::Yellow);
            for (i, ch) in body_label.chars().enumerate() {
                let x = area.x + i as u16;
                if x >= area.x + area.width {
                    break;
                }
                buf.get_mut(x, body_y).set_char(ch).set_style(bl_style);
            }

            if let Some(body) = &req.body {
                let body_content_y = body_y + 1;
                if body_content_y < area.y + area.height {
                    let body_bg = if is_body_active { Color::Rgb(15, 20, 30) } else { Color::Reset };
                    for (i, ch) in body.chars().take(area.width as usize).enumerate() {
                        let x = area.x + i as u16;
                        if x >= area.x + area.width {
                            break;
                        }
                        buf.get_mut(x, body_content_y)
                            .set_char(ch)
                            .set_style(Style::default().bg(body_bg).fg(Color::White));
                    }
                }
            } else {
                let empty = " (no body) ";
                let empty_y = body_y + 1;
                if empty_y < area.y + area.height {
                    for (i, ch) in empty.chars().enumerate() {
                        let x = area.x + i as u16;
                        if x >= area.x + area.width {
                            break;
                        }
                        buf.get_mut(x, empty_y)
                            .set_char(ch)
                            .set_style(Style::default().fg(Color::DarkGray));
                    }
                }
            }
        }
    }

    fn render_response(&self, area: Rect, buf: &mut Buffer) {
        let Some(resp) = &self.state.last_response else {
            let msg = " No response yet — press Enter to send ";
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

        // Status line
        let status_color = status_code_color(resp.status);
        let status_line = format!(
            "HTTP {} — {}ms — {} bytes",
            resp.status, resp.duration_ms, resp.size_bytes
        );
        let status_style = Style::default().fg(status_color).add_modifier(Modifier::BOLD);
        for (i, ch) in status_line.chars().enumerate() {
            let x = area.x + i as u16;
            if x >= area.x + area.width {
                break;
            }
            buf.get_mut(x, area.y).set_char(ch).set_style(status_style);
        }

        // Response headers (compact)
        let mut row = 2u16;
        let header_label = "Response Headers:";
        if area.y + row < area.y + area.height {
            for (i, ch) in header_label.chars().enumerate() {
                let x = area.x + i as u16;
                if x >= area.x + area.width {
                    break;
                }
                buf.get_mut(x, area.y + row)
                    .set_char(ch)
                    .set_style(Style::default().fg(Color::Yellow));
            }
            row += 1;
        }

        for (name, value) in resp.headers.iter().take(5) {
            if area.y + row >= area.y + area.height {
                break;
            }
            let h_line = format!("  {}: {}", name, value.chars().take(50).collect::<String>());
            for (i, ch) in h_line.chars().enumerate() {
                let x = area.x + i as u16;
                if x >= area.x + area.width {
                    break;
                }
                buf.get_mut(x, area.y + row)
                    .set_char(ch)
                    .set_style(Style::default().fg(Color::Gray));
            }
            row += 1;
        }

        // Body preview
        row += 1;
        if area.y + row < area.y + area.height {
            let body_label = "Body:";
            for (i, ch) in body_label.chars().enumerate() {
                let x = area.x + i as u16;
                if x >= area.x + area.width {
                    break;
                }
                buf.get_mut(x, area.y + row)
                    .set_char(ch)
                    .set_style(Style::default().fg(Color::Yellow));
            }
            row += 1;
        }

        // Pretty-print JSON if possible
        let body_display = if let Ok(val) = serde_json::from_str::<serde_json::Value>(&resp.body) {
            serde_json::to_string_pretty(&val).unwrap_or_else(|_| resp.body.clone())
        } else {
            resp.body.clone()
        };

        for (line_idx, line) in body_display.lines().enumerate() {
            if area.y + row >= area.y + area.height {
                break;
            }
            for (i, ch) in line.chars().enumerate() {
                let x = area.x + i as u16;
                if x >= area.x + area.width {
                    break;
                }
                buf.get_mut(x, area.y + row)
                    .set_char(ch)
                    .set_style(Style::default().fg(Color::White));
            }
            row += 1;
        }
    }
}

fn method_color(method: &runner::HttpMethod) -> Color {
    use runner::HttpMethod;
    match method {
        HttpMethod::Get => Color::Green,
        HttpMethod::Post => Color::Yellow,
        HttpMethod::Put => Color::Blue,
        HttpMethod::Patch => Color::Cyan,
        HttpMethod::Delete => Color::Red,
        HttpMethod::Head => Color::Magenta,
        HttpMethod::Options => Color::Gray,
    }
}

fn status_code_color(status: u16) -> Color {
    match status {
        200..=299 => Color::Green,
        300..=399 => Color::Cyan,
        400..=499 => Color::Yellow,
        500..=599 => Color::Red,
        _ => Color::Gray,
    }
}
