#![allow(dead_code)]
//! Multi-cursor support. The primary cursor is in EditorView; this tracks extras.
use crate::view::CursorPos;

pub struct MultiCursor {
    pub extras: Vec<CursorPos>,
}

impl MultiCursor {
    pub fn new() -> Self {
        Self { extras: Vec::new() }
    }

    pub fn add(&mut self, pos: CursorPos) {
        if !self.extras.iter().any(|c| c.line == pos.line && c.col == pos.col) {
            self.extras.push(pos);
        }
    }

    pub fn remove_at(&mut self, index: usize) {
        if index < self.extras.len() {
            self.extras.remove(index);
        }
    }

    pub fn clear(&mut self) {
        self.extras.clear();
    }

    /// Iterate all extra cursor positions.
    pub fn iter(&self) -> impl Iterator<Item = &CursorPos> {
        self.extras.iter()
    }

    pub fn len(&self) -> usize {
        self.extras.len()
    }

    pub fn is_empty(&self) -> bool {
        self.extras.is_empty()
    }
}

impl Default for MultiCursor {
    fn default() -> Self {
        Self::new()
    }
}
