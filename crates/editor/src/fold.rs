#![allow(dead_code)]
//! Code folding state.

#[derive(Debug, Clone)]
pub struct FoldRange {
    pub start_line: usize,
    pub end_line: usize,
    pub folded: bool,
}

pub struct FoldState {
    ranges: Vec<FoldRange>,
}

impl FoldState {
    pub fn new() -> Self {
        Self { ranges: Vec::new() }
    }

    /// Add a fold range.
    pub fn add_range(&mut self, start_line: usize, end_line: usize) {
        if start_line < end_line {
            self.ranges.push(FoldRange { start_line, end_line, folded: false });
            self.ranges.sort_by_key(|r| r.start_line);
        }
    }

    /// Toggle fold at the range containing `line`.
    pub fn toggle(&mut self, line: usize) {
        if let Some(r) = self.range_containing_mut(line) {
            r.folded = !r.folded;
        }
    }

    /// Fold the range containing `line`.
    pub fn fold(&mut self, line: usize) {
        if let Some(r) = self.range_containing_mut(line) {
            r.folded = true;
        }
    }

    /// Unfold the range containing `line`.
    pub fn unfold(&mut self, line: usize) {
        if let Some(r) = self.range_containing_mut(line) {
            r.folded = false;
        }
    }

    pub fn fold_all(&mut self) {
        for r in &mut self.ranges {
            r.folded = true;
        }
    }

    pub fn unfold_all(&mut self) {
        for r in &mut self.ranges {
            r.folded = false;
        }
    }

    /// Returns true if `line` is hidden (inside a folded range, not the start line).
    pub fn is_folded(&self, line: usize) -> bool {
        self.ranges.iter().any(|r| r.folded && line > r.start_line && line <= r.end_line)
    }

    /// True if `line` is the start of a folded range.
    pub fn is_fold_start(&self, line: usize) -> bool {
        self.ranges.iter().any(|r| r.folded && r.start_line == line)
    }

    /// Number of visible lines given total buffer lines.
    pub fn visible_line_count(&self, total_lines: usize) -> usize {
        let mut hidden = 0usize;
        for r in &self.ranges {
            if r.folded {
                // Lines [start_line+1 .. end_line] are hidden
                let count = r.end_line.saturating_sub(r.start_line).min(total_lines.saturating_sub(r.start_line + 1));
                hidden += count;
            }
        }
        total_lines.saturating_sub(hidden)
    }

    /// Map a visual line index to a buffer line index.
    pub fn visual_to_buffer_line(&self, visual_line: usize, total_lines: usize) -> usize {
        let mut visual = 0usize;
        let mut buffer = 0usize;
        while buffer < total_lines {
            if visual == visual_line {
                return buffer;
            }
            visual += 1;
            if self.is_folded(buffer + 1) {
                // Skip the hidden lines of the fold
                if let Some(r) = self.ranges.iter().find(|r| r.folded && r.start_line == buffer) {
                    buffer = r.end_line + 1;
                    continue;
                }
            }
            buffer += 1;
        }
        buffer.min(total_lines.saturating_sub(1))
    }

    fn range_containing_mut(&mut self, line: usize) -> Option<&mut FoldRange> {
        self.ranges.iter_mut().find(|r| r.start_line <= line && line <= r.end_line)
    }

    pub fn ranges(&self) -> &[FoldRange] {
        &self.ranges
    }
}

impl Default for FoldState {
    fn default() -> Self {
        Self::new()
    }
}
