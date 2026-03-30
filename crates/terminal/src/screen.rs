/// Screen buffer: 2D grid of Cell structs with dirty-row tracking.
use bitvec::vec::BitVec;
use serde::{Deserialize, Serialize};

use crate::vt::color::Color;
use crate::vt::attrs::Attrs;

// ─── Cell ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cell {
    pub ch: char,
    pub fg: Color,
    pub bg: Color,
    pub attrs: Attrs,
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            ch: ' ',
            fg: Color::Default,
            bg: Color::Default,
            attrs: Attrs::empty(),
        }
    }
}

// ─── Scrollback ───────────────────────────────────────────────────────────────

pub struct Scrollback {
    lines: std::collections::VecDeque<Vec<Cell>>,
    max: usize,
}

impl Scrollback {
    pub fn new(max: usize) -> Self {
        Self { lines: Default::default(), max }
    }

    pub fn push(&mut self, line: Vec<Cell>) {
        if self.lines.len() >= self.max {
            self.lines.pop_front();
        }
        self.lines.push_back(line);
    }

    pub fn len(&self) -> usize {
        self.lines.len()
    }

    pub fn get(&self, idx: usize) -> Option<&Vec<Cell>> {
        self.lines.get(idx)
    }
}

// ─── ScreenBuffer ────────────────────────────────────────────────────────────

pub struct ScreenBuffer {
    cols: usize,
    rows: usize,
    /// Flat `Vec<Cell>` — index with `row * cols + col`.
    cells: Vec<Cell>,
    /// Dirty flags — one bit per row.
    dirty: BitVec,
    pub scrollback: Scrollback,
}

impl ScreenBuffer {
    pub fn new(cols: usize, rows: usize, scrollback_max: usize) -> Self {
        let n = cols * rows;
        Self {
            cols,
            rows,
            cells: vec![Cell::default(); n],
            dirty: BitVec::repeat(true, rows), // all dirty on first paint
            scrollback: Scrollback::new(scrollback_max),
        }
    }

    pub fn cols(&self) -> usize { self.cols }
    pub fn rows(&self) -> usize { self.rows }

    /// Resize the buffer, preserving content where possible.
    pub fn resize(&mut self, cols: usize, rows: usize) {
        let mut new_cells = vec![Cell::default(); cols * rows];
        for r in 0..rows.min(self.rows) {
            for c in 0..cols.min(self.cols) {
                new_cells[r * cols + c] = self.cells[r * self.cols + c].clone();
            }
        }
        self.cols = cols;
        self.rows = rows;
        self.cells = new_cells;
        self.dirty = BitVec::repeat(true, rows);
    }

    // ─── Cell access ─────────────────────────────────────────────────────────

    #[inline]
    pub fn get(&self, row: usize, col: usize) -> &Cell {
        &self.cells[row * self.cols + col]
    }

    #[inline]
    pub fn set(&mut self, row: usize, col: usize, cell: Cell) {
        self.cells[row * self.cols + col] = cell;
        self.dirty.set(row, true);
    }

    pub fn get_mut(&mut self, row: usize, col: usize) -> &mut Cell {
        self.dirty.set(row, true);
        &mut self.cells[row * self.cols + col]
    }

    // ─── Erase operations ────────────────────────────────────────────────────

    pub fn erase_row(&mut self, row: usize) {
        let start = row * self.cols;
        let end = start + self.cols;
        for cell in &mut self.cells[start..end] {
            *cell = Cell::default();
        }
        self.dirty.set(row, true);
    }

    pub fn erase_range(&mut self, row: usize, col_start: usize, col_end: usize) {
        for c in col_start..col_end.min(self.cols) {
            self.cells[row * self.cols + c] = Cell::default();
        }
        self.dirty.set(row, true);
    }

    pub fn erase_all(&mut self) {
        self.cells.iter_mut().for_each(|c| *c = Cell::default());
        self.dirty.fill(true);
    }

    // ─── Scroll ──────────────────────────────────────────────────────────────

    /// Scroll lines [top, bottom) up by `count` lines, saving scrolled-off lines.
    pub fn scroll_up(&mut self, top: usize, bottom: usize, count: usize) {
        for _ in 0..count.min(bottom - top) {
            // Save the top line to scrollback.
            let row = self.cells[top * self.cols..(top + 1) * self.cols].to_vec();
            self.scrollback.push(row);

            // Shift rows up.
            for r in top..bottom - 1 {
                for c in 0..self.cols {
                    let below = self.cells[(r + 1) * self.cols + c].clone();
                    self.cells[r * self.cols + c] = below;
                }
                self.dirty.set(r, true);
            }
            // Blank the bottom row.
            self.erase_row(bottom - 1);
        }
    }

    pub fn scroll_down(&mut self, top: usize, bottom: usize, count: usize) {
        for _ in 0..count.min(bottom - top) {
            for r in (top + 1..bottom).rev() {
                for c in 0..self.cols {
                    let above = self.cells[(r - 1) * self.cols + c].clone();
                    self.cells[r * self.cols + c] = above;
                }
                self.dirty.set(r, true);
            }
            self.erase_row(top);
        }
    }

    // ─── Dirty tracking ──────────────────────────────────────────────────────

    pub fn is_dirty(&self, row: usize) -> bool {
        self.dirty[row]
    }

    pub fn clear_dirty(&mut self) {
        self.dirty.fill(false);
    }

    pub fn mark_all_dirty(&mut self) {
        self.dirty.fill(true);
    }
}
