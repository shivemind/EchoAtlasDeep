#![allow(dead_code)]
//! Per-file diagnostic store.
use std::collections::HashMap;

use parking_lot::RwLock;

use crate::types::{Diagnostic, DiagnosticSeverity};

/// Thread-safe store of diagnostics keyed by file URI.
pub struct DiagnosticsStore {
    inner: RwLock<HashMap<String, FileDiagnostics>>,
}

#[derive(Debug, Clone, Default)]
pub struct FileDiagnostics {
    pub version: Option<i64>,
    pub items: Vec<Diagnostic>,
}

impl FileDiagnostics {
    pub fn errors(&self) -> usize {
        self.items
            .iter()
            .filter(|d| d.severity == Some(DiagnosticSeverity::Error))
            .count()
    }

    pub fn warnings(&self) -> usize {
        self.items
            .iter()
            .filter(|d| d.severity == Some(DiagnosticSeverity::Warning))
            .count()
    }

    /// All diagnostics starting on the given (0-indexed) line.
    pub fn on_line(&self, line: u32) -> Vec<&Diagnostic> {
        self.items
            .iter()
            .filter(|d| d.range.start.line == line)
            .collect()
    }
}

impl DiagnosticsStore {
    pub fn new() -> Self {
        Self {
            inner: RwLock::new(HashMap::new()),
        }
    }

    pub fn update(&self, uri: &str, version: Option<i64>, items: Vec<Diagnostic>) {
        let mut map = self.inner.write();
        map.insert(
            uri.to_string(),
            FileDiagnostics { version, items },
        );
    }

    pub fn get(&self, uri: &str) -> Option<FileDiagnostics> {
        self.inner.read().get(uri).cloned()
    }

    pub fn all(&self) -> Vec<(String, FileDiagnostics)> {
        self.inner
            .read()
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }

    pub fn total_errors(&self) -> usize {
        self.inner.read().values().map(|f| f.errors()).sum()
    }

    pub fn total_warnings(&self) -> usize {
        self.inner.read().values().map(|f| f.warnings()).sum()
    }
}

impl Default for DiagnosticsStore {
    fn default() -> Self {
        Self::new()
    }
}
