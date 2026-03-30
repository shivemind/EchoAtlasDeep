#![allow(dead_code)]
//! Buffer search state and match finding.
use regex::Regex;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchDir {
    Forward,
    Backward,
}

#[derive(Debug, Clone, Copy)]
pub struct SearchMatch {
    pub line: usize,
    pub start_col: usize,
    pub end_col: usize,
}

pub struct SearchState {
    pub pattern: String,
    pub dir: SearchDir,
    pub matches: Vec<SearchMatch>,
    pub current: usize,
    regex: Option<Regex>,
}

impl SearchState {
    pub fn new() -> Self {
        Self {
            pattern: String::new(),
            dir: SearchDir::Forward,
            matches: Vec::new(),
            current: 0,
            regex: None,
        }
    }

    /// Set a new search pattern and direction.
    pub fn set_pattern(&mut self, pattern: String, dir: SearchDir) {
        self.pattern = pattern.clone();
        self.dir = dir;
        self.current = 0;
        self.regex = Regex::new(&pattern).ok();
    }

    /// Find all matches in the given lines, stores them in self.matches.
    pub fn find_all(&mut self, lines: &[&str]) {
        self.matches.clear();
        let re = match &self.regex {
            Some(r) => r,
            None => return,
        };
        for (line_idx, line) in lines.iter().enumerate() {
            for m in re.find_iter(line) {
                // Compute col offsets in chars (not bytes) for display.
                let start_col = line[..m.start()].chars().count();
                let end_col = line[..m.end()].chars().count();
                self.matches.push(SearchMatch {
                    line: line_idx,
                    start_col,
                    end_col,
                });
            }
        }
    }

    /// Get the next match after (current_line, current_col), wrapping around.
    pub fn next_match(&mut self, current_line: usize, current_col: usize) -> Option<SearchMatch> {
        if self.matches.is_empty() {
            return None;
        }
        // Find first match after current position
        let pos = self.matches.iter().position(|m| {
            m.line > current_line || (m.line == current_line && m.start_col > current_col)
        });
        let idx = pos.unwrap_or(0); // wrap to beginning
        self.current = idx;
        Some(self.matches[idx])
    }

    /// Get the previous match before current position, wrapping.
    pub fn prev_match(&mut self, current_line: usize, current_col: usize) -> Option<SearchMatch> {
        if self.matches.is_empty() {
            return None;
        }
        // Find last match before current position
        let pos = self.matches.iter().rposition(|m| {
            m.line < current_line || (m.line == current_line && m.start_col < current_col)
        });
        let idx = pos.unwrap_or(self.matches.len() - 1); // wrap to end
        self.current = idx;
        Some(self.matches[idx])
    }

    pub fn current_match(&self) -> Option<SearchMatch> {
        self.matches.get(self.current).copied()
    }

    pub fn matches_on_line(&self, line: usize) -> impl Iterator<Item = &SearchMatch> {
        self.matches.iter().filter(move |m| m.line == line)
    }
}

impl Default for SearchState {
    fn default() -> Self {
        Self::new()
    }
}
