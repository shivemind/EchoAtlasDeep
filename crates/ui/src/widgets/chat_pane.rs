#![allow(dead_code, unused_imports, unused_variables)]
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::Widget,
};

use ai::chat::{ChatLine, ChatLineKind};

/// Render-ready snapshot of a ChatSession.
pub struct ChatDisplay<'a> {
    pub display_lines: &'a [ChatLine],
    pub scroll_offset: usize,
    pub input: &'a str,
    pub streaming: bool,
}

pub struct ChatPaneWidget<'a> {
    pub display: ChatDisplay<'a>,
    pub focused: bool,
}

impl<'a> Widget for ChatPaneWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.height < 3 {
            return;
        }

        // Reserve bottom 3 rows for input box
        let chat_area = Rect {
            height: area.height.saturating_sub(3),
            ..area
        };
        let input_area = Rect {
            y: area.y + area.height.saturating_sub(3),
            height: 3,
            ..area
        };

        render_chat_history(&self.display, chat_area, buf);
        render_input_box(&self.display, input_area, buf, self.focused);
    }
}

fn render_chat_history(display: &ChatDisplay<'_>, area: Rect, buf: &mut Buffer) {
    if area.height == 0 {
        return;
    }
    let visible = area.height as usize;
    let total = display.display_lines.len();

    let scroll = if display.scroll_offset == usize::MAX {
        total.saturating_sub(visible)
    } else {
        display.scroll_offset.min(total.saturating_sub(visible))
    };

    for (i, line) in display
        .display_lines
        .iter()
        .enumerate()
        .skip(scroll)
        .take(visible)
    {
        let row = area.y + (i - scroll) as u16;
        if row >= area.y + area.height {
            break;
        }

        let (fg, modifier) = match line.kind {
            ChatLineKind::UserMsg => (Color::Cyan, Modifier::BOLD),
            ChatLineKind::AssistantMsg => (Color::White, Modifier::empty()),
            ChatLineKind::AssistantStreaming => (Color::White, Modifier::DIM),
            ChatLineKind::SystemInfo => (Color::DarkGray, Modifier::ITALIC),
            ChatLineKind::CodeBlock => (Color::Green, Modifier::empty()),
            ChatLineKind::CodeBlockLang => (Color::Yellow, Modifier::BOLD),
            ChatLineKind::Error => (Color::Red, Modifier::BOLD),
        };

        let style = Style::default().fg(fg).add_modifier(modifier);
        let mut col = area.x + 1;
        for c in line.text.chars() {
            if col >= area.x + area.width - 1 {
                break;
            }
            buf[(col, row)].set_style(style).set_char(c);
            col += 1;
        }
    }
}

fn render_input_box(display: &ChatDisplay<'_>, area: Rect, buf: &mut Buffer, focused: bool) {
    let border_style = Style::default().fg(if focused {
        Color::Cyan
    } else {
        Color::DarkGray
    });
    let label = "─── Message ───";
    let mut col = area.x;
    for c in label.chars() {
        if col >= area.x + area.width {
            break;
        }
        buf[(col, area.y)].set_style(border_style).set_char(c);
        col += 1;
    }
    while col < area.x + area.width {
        buf[(col, area.y)].set_style(border_style).set_char('─');
        col += 1;
    }

    // Input text on row+1
    let text_row = area.y + 1;
    if text_row >= area.y + area.height {
        return;
    }
    let text_style = Style::default().fg(Color::White);
    let prompt = "> ";
    let mut x = area.x;
    for c in prompt.chars() {
        if x >= area.x + area.width {
            break;
        }
        buf[(x, text_row)]
            .set_style(Style::default().fg(Color::Cyan))
            .set_char(c);
        x += 1;
    }
    for c in display.input.chars() {
        if x >= area.x + area.width - 1 {
            break;
        }
        buf[(x, text_row)].set_style(text_style).set_char(c);
        x += 1;
    }
    // cursor
    if focused && x < area.x + area.width {
        buf[(x, text_row)]
            .set_style(Style::default().bg(Color::White).fg(Color::Black))
            .set_char(' ');
    }

    // Bottom border
    let bottom = area.y + 2;
    if bottom < area.y + area.height {
        for c in area.x..area.x + area.width {
            buf[(c, bottom)].set_style(border_style).set_char('─');
        }
    }
}
