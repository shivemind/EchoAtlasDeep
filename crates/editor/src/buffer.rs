#![allow(dead_code)]
//! Piece-table text buffer.
use core::ids::BufferId;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineEnding { Lf, CrLf, Cr }

/// A span in either the original file data or the append buffer.
#[derive(Debug, Clone)]
struct Piece {
    in_add: bool,    // false = original, true = add buffer
    start:  usize,
    length: usize,
}

pub struct PieceTable {
    original: Vec<u8>,
    add:      Vec<u8>,
    pieces:   Vec<Piece>,
    /// Byte offset of each line start (index 0 = byte 0).
    line_index: Vec<usize>,
}

impl PieceTable {
    pub fn from_bytes(data: Vec<u8>) -> Self {
        let len = data.len();
        let mut pt = Self {
            original: data,
            add: Vec::new(),
            pieces: vec![Piece { in_add: false, start: 0, length: len }],
            line_index: Vec::new(),
        };
        pt.rebuild_line_index();
        pt
    }

    pub fn empty() -> Self {
        Self::from_bytes(Vec::new())
    }

    pub fn len(&self) -> usize {
        self.pieces.iter().map(|p| p.length).sum()
    }

    pub fn is_empty(&self) -> bool { self.len() == 0 }

    pub fn line_count(&self) -> usize {
        self.line_index.len()
    }

    /// Byte offset of line `line` (0-indexed).
    pub fn line_start(&self, line: usize) -> usize {
        self.line_index.get(line).copied().unwrap_or(self.len())
    }

    /// Which line contains byte offset `offset`.
    pub fn line_at_offset(&self, offset: usize) -> usize {
        match self.line_index.binary_search(&offset) {
            Ok(i)  => i,
            Err(i) => i.saturating_sub(1),
        }
    }

    /// Insert `text` at byte `offset`.
    pub fn insert(&mut self, offset: usize, text: &[u8]) {
        if text.is_empty() { return; }
        let add_start = self.add.len();
        self.add.extend_from_slice(text);
        self.insert_piece_at(offset, Piece { in_add: true, start: add_start, length: text.len() });
        self.rebuild_line_index();
    }

    /// Delete bytes in range `[start, end)`.
    pub fn delete(&mut self, start: usize, end: usize) {
        if start >= end { return; }
        self.delete_range(start, end);
        self.rebuild_line_index();
    }

    /// Collect all bytes into a Vec (expensive — use only for save/LSP sync).
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.len());
        for piece in &self.pieces {
            let src = if piece.in_add { &self.add } else { &self.original };
            out.extend_from_slice(&src[piece.start..piece.start + piece.length]);
        }
        out
    }

    /// Collect as String (UTF-8 lossy).
    pub fn to_string_lossy(&self) -> String {
        String::from_utf8_lossy(&self.to_bytes()).into_owned()
    }

    // ─── Internal ────────────────────────────────────────────────────────────

    fn byte_src(&self, piece: &Piece) -> &[u8] {
        if piece.in_add { &self.add } else { &self.original }
    }

    fn insert_piece_at(&mut self, offset: usize, new_piece: Piece) {
        let mut remaining = offset;
        let mut idx = 0;
        while idx < self.pieces.len() {
            let len = self.pieces[idx].length;
            if remaining < len {
                // Split piece[idx] at `remaining`.
                let left = Piece {
                    in_add: self.pieces[idx].in_add,
                    start:  self.pieces[idx].start,
                    length: remaining,
                };
                let right = Piece {
                    in_add: self.pieces[idx].in_add,
                    start:  self.pieces[idx].start + remaining,
                    length: len - remaining,
                };
                self.pieces.splice(idx..=idx, [left, new_piece, right]);
                return;
            } else if remaining == len {
                self.pieces.insert(idx + 1, new_piece);
                return;
            }
            remaining -= len;
            idx += 1;
        }
        self.pieces.push(new_piece);
    }

    fn delete_range(&mut self, start: usize, end: usize) {
        let mut new_pieces: Vec<Piece> = Vec::new();
        let mut pos = 0usize;
        for piece in &self.pieces {
            let piece_end = pos + piece.length;
            if piece_end <= start || pos >= end {
                // Outside deletion range — keep.
                new_pieces.push(piece.clone());
            } else {
                // Partially or fully inside.
                let keep_before = start.saturating_sub(pos).min(piece.length);
                let keep_after_start = end.saturating_sub(pos).min(piece.length);
                if keep_before > 0 {
                    new_pieces.push(Piece {
                        in_add: piece.in_add,
                        start:  piece.start,
                        length: keep_before,
                    });
                }
                if keep_after_start < piece.length {
                    new_pieces.push(Piece {
                        in_add: piece.in_add,
                        start:  piece.start + keep_after_start,
                        length: piece.length - keep_after_start,
                    });
                }
            }
            pos = piece_end;
        }
        self.pieces = new_pieces;
    }

    /// Get the content of a single line as a String (without trailing newline).
    pub fn line_content(&self, line: usize) -> String {
        let start = self.line_start(line);
        let end = if line + 1 < self.line_index.len() {
            self.line_index[line + 1]
        } else {
            self.len()
        };
        let bytes = self.bytes_in_range(start, end);
        let s = String::from_utf8_lossy(&bytes).into_owned();
        // Strip trailing newline(s)
        let s = s.trim_end_matches('\n').trim_end_matches('\r');
        s.to_string()
    }

    /// Get a single char at byte offset `offset`.
    pub fn char_at(&self, offset: usize) -> Option<char> {
        // Get a few bytes around offset and decode
        let bytes = self.bytes_in_range(offset, (offset + 4).min(self.len()));
        std::str::from_utf8(&bytes).ok().and_then(|s| s.chars().next())
    }

    /// Convert (line, col) where col is a char column to a byte offset.
    pub fn line_col_to_offset(&self, line: usize, col: usize) -> usize {
        let line_start = self.line_start(line);
        let line_text = self.line_content(line);
        let byte_col: usize = line_text.chars().take(col).map(|c| c.len_utf8()).sum();
        line_start + byte_col
    }

    /// Collect bytes in range [start, end).
    pub fn bytes_in_range(&self, start: usize, end: usize) -> Vec<u8> {
        if start >= end {
            return Vec::new();
        }
        let mut result = Vec::with_capacity(end - start);
        let mut pos = 0usize;
        for piece in &self.pieces {
            let piece_end = pos + piece.length;
            if piece_end <= start {
                pos = piece_end;
                continue;
            }
            if pos >= end {
                break;
            }
            let src = if piece.in_add { &self.add } else { &self.original };
            let p_start = piece.start + start.saturating_sub(pos);
            let p_end = piece.start + (end.min(piece_end) - pos);
            result.extend_from_slice(&src[p_start..p_end]);
            pos = piece_end;
        }
        result
    }

    fn rebuild_line_index(&mut self) {
        let mut new_index = vec![0usize];
        let mut offset = 0usize;
        for piece in &self.pieces {
            let src = self.byte_src(piece);
            let slice = &src[piece.start..piece.start + piece.length];
            for &b in slice {
                offset += 1;
                if b == b'\n' {
                    new_index.push(offset);
                }
            }
        }
        self.line_index = new_index;
    }
}

// ─── EditorBuffer ────────────────────────────────────────────────────────────

pub struct EditorBuffer {
    pub id: BufferId,
    pub path: Option<PathBuf>,
    pub text: PieceTable,
    pub line_ending: LineEnding,
    pub dirty: bool,
}

impl EditorBuffer {
    pub fn new(id: BufferId) -> Self {
        Self {
            id,
            path: None,
            text: PieceTable::empty(),
            line_ending: LineEnding::Lf,
            dirty: false,
        }
    }

    pub fn from_file(id: BufferId, path: PathBuf, data: Vec<u8>) -> Self {
        let line_ending = detect_line_ending(&data);
        Self {
            id,
            path: Some(path),
            text: PieceTable::from_bytes(data),
            line_ending,
            dirty: false,
        }
    }

    pub fn insert(&mut self, offset: usize, text: &str) {
        self.text.insert(offset, text.as_bytes());
        self.dirty = true;
    }

    pub fn delete(&mut self, start: usize, end: usize) {
        self.text.delete(start, end);
        self.dirty = true;
    }

    pub fn line_count(&self) -> usize {
        self.text.line_count()
    }

    pub fn line_content(&self, line: usize) -> String {
        self.text.line_content(line)
    }

    pub fn get_line_text(&self, line: usize) -> String {
        self.text.line_content(line)
    }

    pub fn insert_at_line_col(&mut self, line: usize, col: usize, text: &str) {
        let offset = self.text.line_col_to_offset(line, col);
        self.insert(offset, text);
    }

    /// Delete from (start_line, start_col) to (end_line, end_col) exclusive.
    /// Returns the deleted text.
    pub fn delete_range_line_col(
        &mut self,
        start_line: usize,
        start_col: usize,
        end_line: usize,
        end_col: usize,
    ) -> String {
        let start = self.text.line_col_to_offset(start_line, start_col);
        let end = self.text.line_col_to_offset(end_line, end_col);
        if start >= end {
            return String::new();
        }
        let bytes = self.text.bytes_in_range(start, end);
        let deleted = String::from_utf8_lossy(&bytes).into_owned();
        self.delete(start, end);
        deleted
    }
}

fn detect_line_ending(data: &[u8]) -> LineEnding {
    for i in 0..data.len() {
        if data[i] == b'\r' {
            if data.get(i + 1) == Some(&b'\n') { return LineEnding::CrLf; }
            return LineEnding::Cr;
        }
        if data[i] == b'\n' { return LineEnding::Lf; }
    }
    LineEnding::Lf
}
