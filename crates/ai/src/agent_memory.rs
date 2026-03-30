#![allow(dead_code, unused_imports, unused_variables)]
//! JSON-backed persistent memory for agent sessions.
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub key: String,
    pub value: String,
    pub updated_at: u64,
}

impl MemoryEntry {
    fn new(key: &str, value: &str) -> Self {
        let updated_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        Self {
            key: key.to_string(),
            value: value.to_string(),
            updated_at,
        }
    }
}

pub struct AgentMemory {
    entries: RwLock<HashMap<String, MemoryEntry>>,
    db_path: PathBuf,
}

impl AgentMemory {
    /// Load from `.rmtide/agent-memory.json`, creating an empty store if not present.
    pub fn new(workspace_root: &Path) -> Self {
        let db_path = workspace_root.join(".rmtide").join("agent-memory.json");
        let entries = Self::load_from_disk(&db_path);
        Self {
            entries: RwLock::new(entries),
            db_path,
        }
    }

    fn load_from_disk(path: &Path) -> HashMap<String, MemoryEntry> {
        if let Ok(json) = std::fs::read_to_string(path) {
            if let Ok(map) = serde_json::from_str::<HashMap<String, MemoryEntry>>(&json) {
                return map;
            }
        }
        HashMap::new()
    }

    /// Store a key-value pair.
    pub fn set(&self, key: &str, value: &str) {
        let entry = MemoryEntry::new(key, value);
        self.entries.write().insert(key.to_string(), entry);
    }

    /// Retrieve a value by key.
    pub fn get(&self, key: &str) -> Option<String> {
        self.entries.read().get(key).map(|e| e.value.clone())
    }

    /// Delete a key.
    pub fn delete(&self, key: &str) {
        self.entries.write().remove(key);
    }

    /// List all entries sorted by updated_at descending.
    pub fn list(&self) -> Vec<MemoryEntry> {
        let mut entries: Vec<MemoryEntry> = self.entries.read().values().cloned().collect();
        entries.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        entries
    }

    /// Flush current state to disk.
    pub fn flush(&self) -> anyhow::Result<()> {
        if let Some(parent) = self.db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let map = self.entries.read().clone();
        let json = serde_json::to_string_pretty(&map)?;
        std::fs::write(&self.db_path, json)?;
        Ok(())
    }

    /// Format entries as a string suitable for system prompt injection.
    /// Truncates to `max_chars` if needed.
    pub fn as_context(&self, max_chars: usize) -> String {
        let entries = self.list();
        if entries.is_empty() {
            return String::new();
        }
        let mut buf = String::from("Agent Memory:\n");
        for e in &entries {
            let line = format!("  {}: {}\n", e.key, e.value);
            if buf.len() + line.len() > max_chars {
                buf.push_str("  ...(truncated)\n");
                break;
            }
            buf.push_str(&line);
        }
        buf
    }
}
