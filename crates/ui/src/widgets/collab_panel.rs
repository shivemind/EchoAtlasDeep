#![allow(dead_code, unused_imports, unused_variables)]
//! Collaborative editing scaffolding — Phase 12 Point 48.
//! WebRTC not yet implemented — scaffolding with CRDT-ready structures.
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::Widget,
};

#[derive(Clone, Debug)]
pub struct CollabParticipant {
    pub name: String,
    pub color: Color,
    pub cursor_file: Option<String>,
    pub cursor_line: Option<usize>,
    pub is_host: bool,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum CollabStatus {
    Disconnected,
    Hosting { port: u16 },
    Connected,
}

impl CollabStatus {
    pub fn label(&self) -> String {
        match self {
            CollabStatus::Disconnected => " ● Disconnected".to_string(),
            CollabStatus::Hosting { port } => format!(" ● Hosting on port {}", port),
            CollabStatus::Connected => " ● Connected".to_string(),
        }
    }

    pub fn color(&self) -> Color {
        match self {
            CollabStatus::Disconnected => Color::DarkGray,
            CollabStatus::Hosting { .. } => Color::Green,
            CollabStatus::Connected => Color::Cyan,
        }
    }
}

pub struct CollabState {
    pub open: bool,
    pub status: CollabStatus,
    pub session_id: Option<String>,
    pub participants: Vec<CollabParticipant>,
    pub chat_messages: Vec<(String, String)>,
    pub chat_input: String,
}

impl CollabState {
    pub fn new() -> Self {
        Self {
            open: false,
            status: CollabStatus::Disconnected,
            session_id: None,
            participants: Vec::new(),
            chat_messages: Vec::new(),
            chat_input: String::new(),
        }
    }

    /// Generate a session ID (UUID-based).
    pub fn generate_session_id(&mut self) {
        self.session_id = Some(uuid::Uuid::new_v4().to_string());
    }

    /// Start hosting a session.
    /// TODO: WebRTC — actually bind a WebRTC signaling server here.
    pub fn start_hosting(&mut self) {
        // TODO: WebRTC — replace with actual WebRTC data channel setup
        if self.session_id.is_none() {
            self.generate_session_id();
        }
        // Default port 7890 for the collab session
        // TODO: WebRTC — bind the actual transport layer
        self.status = CollabStatus::Hosting { port: 7890 };
        // Add self as first participant (host)
        self.participants.insert(0, CollabParticipant {
            name: "You (Host)".into(),
            color: Color::Green,
            cursor_file: None,
            cursor_line: None,
            is_host: true,
        });
    }

    /// Disconnect from the session.
    pub fn disconnect(&mut self) {
        // TODO: WebRTC — close data channels and peer connections
        self.status = CollabStatus::Disconnected;
        self.participants.clear();
        self.chat_messages.push((
            "System".into(),
            "Disconnected from collaborative session.".into(),
        ));
    }

    /// Add a message to the chat log.
    pub fn send_chat(&mut self, message: &str) {
        if !message.is_empty() {
            let name = self.participants.first()
                .map(|p| p.name.clone())
                .unwrap_or_else(|| "You".into());
            self.chat_messages.push((name, message.to_string()));
            self.chat_input.clear();
        }
    }
}

impl Default for CollabState {
    fn default() -> Self {
        Self::new()
    }
}

pub struct CollabPanelWidget<'a> {
    pub state: &'a CollabState,
}

impl Widget for CollabPanelWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 10 || area.height < 8 {
            return;
        }

        let w = (area.width * 2 / 3).max(60).min(area.width);
        let h = (area.height * 3 / 4).max(20).min(area.height);
        let x = area.x + (area.width.saturating_sub(w)) / 2;
        let y = area.y + (area.height.saturating_sub(h)) / 2;

        let bg = Style::default().bg(Color::Rgb(12, 16, 22));
        let border_style = Style::default().fg(Color::Cyan);
        let title_style = Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD);
        let label_style = Style::default().fg(Color::DarkGray);
        let status_color = self.state.status.color();

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
        let title = " Collaborative Editing (Experimental) ";
        let title_x = x + (w.saturating_sub(title.len() as u16)) / 2;
        for (i, ch) in title.chars().enumerate() {
            if title_x + i as u16 >= x + w - 1 { break; }
            buf.get_mut(title_x + i as u16, y).set_char(ch).set_style(title_style);
        }

        // Status row
        let status_label = self.state.status.label();
        for (i, ch) in status_label.chars().take((w as usize).saturating_sub(2)).enumerate() {
            let col = x + 1 + i as u16;
            if col >= x + w - 1 { break; }
            buf.get_mut(col, y + 1).set_char(ch).set_style(
                Style::default().fg(status_color).add_modifier(Modifier::BOLD)
            );
        }

        // Session ID
        if let Some(ref sid) = self.state.session_id {
            let id_line = format!(" Session: {}", sid);
            for (i, ch) in id_line.chars().take((w as usize).saturating_sub(2)).enumerate() {
                let col = x + 1 + i as u16;
                if col >= x + w - 1 { break; }
                buf.get_mut(col, y + 2).set_char(ch).set_style(
                    Style::default().fg(Color::Yellow)
                );
            }
        } else {
            let no_session = " No active session. Press [h] to host or [j] to join.";
            for (i, ch) in no_session.chars().take((w as usize).saturating_sub(2)).enumerate() {
                let col = x + 1 + i as u16;
                if col >= x + w - 1 { break; }
                buf.get_mut(col, y + 2).set_char(ch).set_style(label_style);
            }
        }

        // Separator
        for col in x + 1..x + w - 1 {
            buf.get_mut(col, y + 3).set_char('─').set_style(border_style);
        }
        buf.get_mut(x, y + 3).set_char('╟').set_style(border_style);
        buf.get_mut(x + w - 1, y + 3).set_char('╢').set_style(border_style);

        // Participants section
        let part_header = " Participants:";
        for (i, ch) in part_header.chars().enumerate() {
            let col = x + 1 + i as u16;
            if col >= x + w - 1 { break; }
            buf.get_mut(col, y + 4).set_char(ch).set_style(label_style);
        }

        let max_participants = 5usize;
        for (pi, participant) in self.state.participants.iter().enumerate().take(max_participants) {
            let py = y + 5 + pi as u16;
            if py >= y + h - 8 { break; }

            let host_badge = if participant.is_host { " [host]" } else { "" };
            let cursor_info = match (&participant.cursor_file, participant.cursor_line) {
                (Some(f), Some(l)) => format!(" @ {}:{}", f, l),
                _ => String::new(),
            };
            let part_line = format!("  ● {}{}{}", participant.name, host_badge, cursor_info);
            for (i, ch) in part_line.chars().take((w as usize).saturating_sub(4)).enumerate() {
                let col = x + 1 + i as u16;
                if col >= x + w - 1 { break; }
                buf.get_mut(col, py).set_char(ch).set_style(
                    Style::default().fg(participant.color)
                );
            }
        }

        if self.state.participants.is_empty() {
            let empty = "  No participants yet.";
            for (i, ch) in empty.chars().enumerate() {
                let col = x + 1 + i as u16;
                if col >= x + w - 1 { break; }
                buf.get_mut(col, y + 5).set_char(ch).set_style(label_style);
            }
        }

        // Chat section separator
        let chat_sep_y = y + h - 9;
        if chat_sep_y > y + 8 {
            for col in x + 1..x + w - 1 {
                buf.get_mut(col, chat_sep_y).set_char('─').set_style(border_style);
            }
            buf.get_mut(x, chat_sep_y).set_char('╟').set_style(border_style);
            buf.get_mut(x + w - 1, chat_sep_y).set_char('╢').set_style(border_style);

            let chat_label = " Chat:";
            for (i, ch) in chat_label.chars().enumerate() {
                let col = x + 1 + i as u16;
                if col >= x + w - 1 { break; }
                buf.get_mut(col, chat_sep_y).set_char(ch).set_style(label_style);
            }

            // Chat messages
            let chat_area_start = chat_sep_y + 1;
            let chat_area_h = 5usize;
            let msgs_to_show: Vec<&(String, String)> = self.state.chat_messages
                .iter()
                .rev()
                .take(chat_area_h)
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect();

            for (mi, (name, msg)) in msgs_to_show.iter().enumerate() {
                let my = chat_area_start + mi as u16;
                if my >= y + h - 3 { break; }
                let msg_line = format!(" <{}> {}", name, msg);
                let name_color = if name == "System" { Color::Yellow } else { Color::Cyan };
                for (i, ch) in msg_line.chars().take((w as usize).saturating_sub(4)).enumerate() {
                    let col = x + 1 + i as u16;
                    if col >= x + w - 1 { break; }
                    let ch_style = if i < name.len() + 3 {
                        Style::default().fg(name_color)
                    } else {
                        Style::default().fg(Color::White)
                    };
                    buf.get_mut(col, my).set_char(ch).set_style(ch_style);
                }
            }

            // Chat input box
            let input_y = y + h - 3;
            for col in x + 1..x + w - 1 {
                buf.get_mut(col, input_y).set_char(' ').set_style(
                    Style::default().bg(Color::Rgb(20, 24, 32))
                );
            }
            let input_prompt = " > ";
            for (i, ch) in input_prompt.chars().enumerate() {
                let col = x + 1 + i as u16;
                if col >= x + w - 1 { break; }
                buf.get_mut(col, input_y).set_char(ch).set_style(
                    Style::default().fg(Color::Cyan).bg(Color::Rgb(20, 24, 32))
                );
            }
            for (i, ch) in self.state.chat_input.chars().take((w as usize).saturating_sub(6)).enumerate() {
                let col = x + 1 + input_prompt.len() as u16 + i as u16;
                if col >= x + w - 1 { break; }
                buf.get_mut(col, input_y).set_char(ch).set_style(
                    Style::default().fg(Color::White).bg(Color::Rgb(20, 24, 32))
                );
            }
        }

        // TODO: WebRTC note
        let webrtc_note = " [TODO: WebRTC transport — scaffolding only] ";
        let note_x = x + w - 1 - webrtc_note.len() as u16 - 1;
        if note_x > x + 1 {
            for (i, ch) in webrtc_note.chars().enumerate() {
                let col = note_x + i as u16;
                if col >= x + w - 1 { break; }
                buf.get_mut(col, y + 1).set_char(ch).set_style(
                    Style::default().fg(Color::DarkGray)
                );
            }
        }

        // Hint bar
        let hint = " [h] host  [j] join  [d] disconnect  [Tab] chat  [Esc] close";
        for (i, ch) in hint.chars().enumerate() {
            let col = x + 1 + i as u16;
            if col >= x + w - 1 { break; }
            buf.get_mut(col, y + h - 2).set_char(ch).set_style(label_style);
        }
    }
}
