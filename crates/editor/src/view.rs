#![allow(dead_code)]
use core::ids::BufferId;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CursorPos {
    pub line: usize,
    pub col: usize,
}

#[derive(Debug, Clone, Copy)]
pub struct Selection {
    pub anchor: CursorPos,
    pub active: CursorPos,
}

pub struct EditorView {
    pub buffer_id: BufferId,
    pub cursor: CursorPos,
    pub selection: Option<Selection>,
    pub scroll_row: usize,
    pub scroll_col: usize,
    /// Where visual selection started (set on entering Visual mode).
    pub visual_anchor: Option<CursorPos>,
    /// Additional cursors (multi-cursor).
    pub extra_cursors: Vec<CursorPos>,
}

impl EditorView {
    pub fn new(buffer_id: BufferId) -> Self {
        Self {
            buffer_id,
            cursor: CursorPos::default(),
            selection: None,
            scroll_row: 0,
            scroll_col: 0,
            visual_anchor: None,
            extra_cursors: Vec::new(),
        }
    }

    /// Ensure the cursor is within the viewport (rows x cols).
    pub fn scroll_to_cursor(&mut self, viewport_rows: usize, viewport_cols: usize) {
        if self.cursor.line < self.scroll_row {
            self.scroll_row = self.cursor.line;
        } else if self.cursor.line >= self.scroll_row + viewport_rows {
            self.scroll_row = self.cursor.line.saturating_sub(viewport_rows - 1);
        }
        if self.cursor.col < self.scroll_col {
            self.scroll_col = self.cursor.col;
        } else if self.cursor.col >= self.scroll_col + viewport_cols {
            self.scroll_col = self.cursor.col.saturating_sub(viewport_cols - 1);
        }
    }

    /// Returns the sorted (start, end) of the current visual selection,
    /// or None if not in visual mode.
    pub fn visual_range(&self) -> Option<(CursorPos, CursorPos)> {
        let anchor = self.visual_anchor?;
        let active = self.cursor;
        // Sort: start is the earlier position
        if anchor.line < active.line || (anchor.line == active.line && anchor.col <= active.col) {
            Some((anchor, active))
        } else {
            Some((active, anchor))
        }
    }

    pub fn add_cursor(&mut self, pos: CursorPos) {
        if !self.extra_cursors.iter().any(|c| c.line == pos.line && c.col == pos.col) {
            self.extra_cursors.push(pos);
        }
    }

    pub fn clear_extra_cursors(&mut self) {
        self.extra_cursors.clear();
    }
}
