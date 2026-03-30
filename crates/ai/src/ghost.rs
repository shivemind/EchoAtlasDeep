#![allow(dead_code, unused_imports, unused_variables)]
use std::time::{Duration, Instant};

/// State for inline ghost-text completion.
pub struct GhostText {
    pub suggestion: Option<String>,
    pub last_keystroke: Instant,
    pub pending: bool,
    pub trigger_delay: Duration,
}

impl GhostText {
    pub fn new() -> Self {
        Self {
            suggestion: None,
            last_keystroke: Instant::now(),
            pending: false,
            trigger_delay: Duration::from_millis(800),
        }
    }

    pub fn on_keystroke(&mut self) {
        self.last_keystroke = Instant::now();
        self.suggestion = None;
        self.pending = false;
    }

    pub fn should_trigger(&self) -> bool {
        !self.pending
            && self.suggestion.is_none()
            && self.last_keystroke.elapsed() >= self.trigger_delay
    }

    pub fn set_suggestion(&mut self, text: String) {
        self.suggestion = Some(text);
        self.pending = false;
    }

    pub fn accept_full(&mut self) -> Option<String> {
        self.pending = false;
        self.suggestion.take()
    }

    pub fn accept_word(&mut self) -> Option<String> {
        let s = self.suggestion.as_mut()?;
        let end = s
            .find(|c: char| c.is_whitespace())
            .map(|i| i + 1)
            .unwrap_or(s.len());
        let word = s[..end].to_string();
        *s = s[end..].to_string();
        if s.is_empty() {
            self.suggestion = None;
        }
        Some(word)
    }

    pub fn dismiss(&mut self) {
        self.suggestion = None;
        self.pending = false;
    }
}

impl Default for GhostText {
    fn default() -> Self {
        Self::new()
    }
}
