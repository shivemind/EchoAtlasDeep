/// VtProcessor: consumes VtEvent values and mutates ScreenBuffer.
/// Implements VT100 / VT220 / xterm escape sequences.
use tracing::{debug, trace, warn};

use crate::screen::{Cell, ScreenBuffer};
use crate::vt::color::Color;
use crate::vt::attrs::Attrs;
use crate::vt::parser::VtEvent;

// ─── Cursor ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, Default)]
pub struct Cursor {
    pub row: usize,
    pub col: usize,
}

// ─── Processor state ─────────────────────────────────────────────────────────

pub struct VtProcessor {
    pub cursor: Cursor,
    pub saved_cursor: Cursor,

    /// Current SGR foreground / background.
    pub fg: Color,
    pub bg: Color,
    pub attrs: Attrs,

    /// Scroll region (inclusive rows).
    pub scroll_top: usize,
    pub scroll_bottom: usize,

    /// Whether the alternate screen is active.
    pub alt_screen: bool,

    /// Bracketed paste mode.
    pub bracketed_paste: bool,

    /// Mouse reporting mode (0 = off).
    pub mouse_mode: u8,

    /// Window title.
    pub title: String,
}

impl VtProcessor {
    pub fn new(rows: usize) -> Self {
        Self {
            cursor: Cursor::default(),
            saved_cursor: Cursor::default(),
            fg: Color::Default,
            bg: Color::Default,
            attrs: Attrs::empty(),
            scroll_top: 0,
            scroll_bottom: rows.saturating_sub(1),
            alt_screen: false,
            bracketed_paste: false,
            mouse_mode: 0,
            title: String::new(),
        }
    }

    /// Process a batch of VtEvents against the given screen buffer.
    pub fn process(&mut self, events: &[VtEvent], buf: &mut ScreenBuffer) {
        for event in events {
            self.handle(event, buf);
        }
    }

    fn handle(&mut self, event: &VtEvent, buf: &mut ScreenBuffer) {
        match event {
            VtEvent::Print(ch) => self.print(*ch, buf),
            VtEvent::Execute(byte) => self.execute(*byte, buf),
            VtEvent::CsiDispatch { params, intermediates, action, .. } => {
                self.csi(params, intermediates, *action, buf);
            }
            VtEvent::OscDispatch { params, .. } => self.osc(params),
            VtEvent::EscDispatch { intermediates, byte, .. } => {
                self.esc(intermediates, *byte, buf);
            }
            VtEvent::Hook { .. } | VtEvent::Put(_) | VtEvent::Unhook => {
                // DCS — not yet implemented.
            }
        }
    }

    // ─── Print ───────────────────────────────────────────────────────────────

    fn print(&mut self, ch: char, buf: &mut ScreenBuffer) {
        let width = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(1);
        if self.cursor.col >= buf.cols() {
            self.cursor.col = 0;
            self.advance_row(buf);
        }
        let cell = Cell {
            ch,
            fg: self.fg,
            bg: self.bg,
            attrs: self.attrs,
        };
        buf.set(self.cursor.row, self.cursor.col, cell);
        // For wide chars, fill the next cell with a placeholder.
        if width == 2 && self.cursor.col + 1 < buf.cols() {
            let wide_cell = Cell {
                ch: ' ',
                fg: self.fg,
                bg: self.bg,
                attrs: self.attrs | Attrs::WIDE,
            };
            buf.set(self.cursor.row, self.cursor.col + 1, wide_cell);
        }
        self.cursor.col += width;
    }

    // ─── C0/C1 control ───────────────────────────────────────────────────────

    fn execute(&mut self, byte: u8, buf: &mut ScreenBuffer) {
        match byte {
            0x07 => { /* BEL — ignore */ }
            0x08 => { // BS
                if self.cursor.col > 0 { self.cursor.col -= 1; }
            }
            0x09 => { // HT (tab)
                let next = (self.cursor.col / 8 + 1) * 8;
                self.cursor.col = next.min(buf.cols() - 1);
            }
            0x0A | 0x0B | 0x0C => self.advance_row(buf), // LF/VT/FF
            0x0D => self.cursor.col = 0,                   // CR
            _ => trace!("Unhandled execute: 0x{byte:02X}"),
        }
    }

    fn advance_row(&mut self, buf: &mut ScreenBuffer) {
        if self.cursor.row >= self.scroll_bottom {
            buf.scroll_up(self.scroll_top, self.scroll_bottom + 1, 1);
        } else {
            self.cursor.row += 1;
        }
    }

    // ─── CSI sequences ───────────────────────────────────────────────────────

    fn csi(&mut self, params: &[Vec<u16>], intermediates: &[u8], action: char, buf: &mut ScreenBuffer) {
        let p = |idx: usize, default: u16| -> u16 {
            params.get(idx).and_then(|v| v.first()).copied()
                .filter(|&v| v != 0)
                .unwrap_or(default)
        };

        match action {
            // Cursor Up/Down/Forward/Back
            'A' => self.cursor.row = self.cursor.row.saturating_sub(p(0, 1) as usize),
            'B' => self.cursor.row = (self.cursor.row + p(0, 1) as usize).min(buf.rows() - 1),
            'C' => self.cursor.col = (self.cursor.col + p(0, 1) as usize).min(buf.cols() - 1),
            'D' => self.cursor.col = self.cursor.col.saturating_sub(p(0, 1) as usize),

            // Cursor Position / Horizontal Vertical Position
            'H' | 'f' => {
                let row = p(0, 1).saturating_sub(1) as usize;
                let col = p(1, 1).saturating_sub(1) as usize;
                self.cursor.row = row.min(buf.rows() - 1);
                self.cursor.col = col.min(buf.cols() - 1);
            }

            // Erase in Display
            'J' => {
                let n = p(0, 0);
                match n {
                    0 => {
                        buf.erase_range(self.cursor.row, self.cursor.col, buf.cols());
                        for r in self.cursor.row + 1..buf.rows() { buf.erase_row(r); }
                    }
                    1 => {
                        for r in 0..self.cursor.row { buf.erase_row(r); }
                        buf.erase_range(self.cursor.row, 0, self.cursor.col + 1);
                    }
                    2 | 3 => buf.erase_all(),
                    _ => {}
                }
            }

            // Erase in Line
            'K' => {
                let n = p(0, 0);
                match n {
                    0 => buf.erase_range(self.cursor.row, self.cursor.col, buf.cols()),
                    1 => buf.erase_range(self.cursor.row, 0, self.cursor.col + 1),
                    2 => buf.erase_row(self.cursor.row),
                    _ => {}
                }
            }

            // Erase Characters
            'X' => {
                let n = p(0, 1) as usize;
                buf.erase_range(self.cursor.row, self.cursor.col, self.cursor.col + n);
            }

            // Insert/Delete Lines
            'L' => buf.scroll_down(self.cursor.row, self.scroll_bottom + 1, p(0, 1) as usize),
            'M' => buf.scroll_up(self.cursor.row, self.scroll_bottom + 1, p(0, 1) as usize),

            // Set Scrolling Region (DECSTBM)
            'r' => {
                let top = p(0, 1).saturating_sub(1) as usize;
                let bottom = p(1, buf.rows() as u16).saturating_sub(1) as usize;
                self.scroll_top = top;
                self.scroll_bottom = bottom.min(buf.rows() - 1);
                self.cursor = Cursor::default();
            }

            // SGR — Select Graphic Rendition
            'm' => self.sgr(params),

            // Save / Restore cursor (ANSI)
            's' => self.saved_cursor = self.cursor,
            'u' => self.cursor = self.saved_cursor,

            // DECSC / DECRC via ESC 7/8 handled in esc()

            // DEC private mode set/reset
            'h' if intermediates == b"?" => self.dec_mode(params, true),
            'l' if intermediates == b"?" => self.dec_mode(params, false),

            // Cursor column (absolute)
            'G' => self.cursor.col = p(0, 1).saturating_sub(1) as usize,

            // Cursor row (absolute)
            'd' => self.cursor.row = p(0, 1).saturating_sub(1) as usize,

            other => trace!("Unhandled CSI: {other:?} params={params:?}"),
        }
    }

    fn sgr(&mut self, params: &[Vec<u16>]) {
        let mut i = 0;
        while i < params.len() {
            let code = params[i].first().copied().unwrap_or(0);
            match code {
                0 => {
                    self.fg = Color::Default;
                    self.bg = Color::Default;
                    self.attrs = Attrs::empty();
                }
                1 => self.attrs |= Attrs::BOLD,
                2 => self.attrs |= Attrs::DIM,
                3 => self.attrs |= Attrs::ITALIC,
                4 => self.attrs |= Attrs::UNDERLINE,
                5 => self.attrs |= Attrs::BLINK,
                6 => self.attrs |= Attrs::BLINK_RAPID,
                7 => self.attrs |= Attrs::REVERSE,
                8 => self.attrs |= Attrs::HIDDEN,
                9 => self.attrs |= Attrs::STRIKETHROUGH,
                22 => self.attrs &= !(Attrs::BOLD | Attrs::DIM),
                23 => self.attrs &= !Attrs::ITALIC,
                24 => self.attrs &= !Attrs::UNDERLINE,
                25 => self.attrs &= !(Attrs::BLINK | Attrs::BLINK_RAPID),
                27 => self.attrs &= !Attrs::REVERSE,
                28 => self.attrs &= !Attrs::HIDDEN,
                29 => self.attrs &= !Attrs::STRIKETHROUGH,
                30..=37 => self.fg = Color::Indexed(code as u8 - 30),
                38 => {
                    if let Some(color) = self.parse_extended_color(params, &mut i) {
                        self.fg = color;
                    }
                }
                39 => self.fg = Color::Default,
                40..=47 => self.bg = Color::Indexed(code as u8 - 40),
                48 => {
                    if let Some(color) = self.parse_extended_color(params, &mut i) {
                        self.bg = color;
                    }
                }
                49 => self.bg = Color::Default,
                90..=97  => self.fg = Color::Indexed(code as u8 - 90 + 8),
                100..=107 => self.bg = Color::Indexed(code as u8 - 100 + 8),
                _ => {}
            }
            i += 1;
        }
    }

    fn parse_extended_color(&self, params: &[Vec<u16>], i: &mut usize) -> Option<Color> {
        let next = params.get(*i + 1)?;
        match next.first().copied()? {
            2 => {
                // 38;2;R;G;B
                let r = params.get(*i + 2)?.first().copied()? as u8;
                let g = params.get(*i + 3)?.first().copied()? as u8;
                let b = params.get(*i + 4)?.first().copied()? as u8;
                *i += 4;
                Some(Color::Rgb(r, g, b))
            }
            5 => {
                // 38;5;index
                let idx = params.get(*i + 2)?.first().copied()? as u8;
                *i += 2;
                Some(Color::Indexed(idx))
            }
            _ => None,
        }
    }

    fn dec_mode(&mut self, params: &[Vec<u16>], enable: bool) {
        for p in params {
            match p.first().copied().unwrap_or(0) {
                1    => { /* DECCKM application cursor — pass through */ }
                7    => { /* Auto-wrap */ }
                12   => { /* Cursor blink */ }
                25   => { /* Cursor visibility */ }
                1000 | 1001 | 1002 | 1003 => {
                    self.mouse_mode = if enable { p[0] as u8 } else { 0 };
                }
                1004 => { /* Focus events */ }
                1049 => { /* Alternate screen — handled by session */ }
                2004 => self.bracketed_paste = enable,
                mode => debug!("Unhandled DEC mode: {mode} enable={enable}"),
            }
        }
    }

    // ─── OSC sequences ───────────────────────────────────────────────────────

    fn osc(&mut self, params: &[Vec<u8>]) {
        let cmd = params.first().and_then(|p| p.first()).copied().unwrap_or(255);
        match cmd {
            b'0' | b'2' => {
                if let Some(title_bytes) = params.get(1) {
                    self.title = String::from_utf8_lossy(title_bytes).into_owned();
                }
            }
            b'8' => { /* Hyperlinks — future */ }
            _ => {}
        }
    }

    // ─── ESC sequences ───────────────────────────────────────────────────────

    fn esc(&mut self, _intermediates: &[u8], byte: u8, _buf: &mut ScreenBuffer) {
        match byte {
            b'7' => self.saved_cursor = self.cursor, // DECSC
            b'8' => self.cursor = self.saved_cursor, // DECRC
            b'M' => { /* Reverse index — scroll down if at top */ }
            _ => trace!("Unhandled ESC: 0x{byte:02X}"),
        }
    }
}
