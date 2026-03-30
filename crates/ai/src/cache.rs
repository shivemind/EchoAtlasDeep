#![allow(dead_code, unused_imports, unused_variables)]
//! Offline response cache — stores AI responses indexed by prompt hash.
use std::collections::HashMap;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedResponse {
    pub prompt_hash: u64,
    pub provider: String,
    pub model: String,
    pub response: String,
    pub timestamp: u64,
    pub input_tokens: u64,
    pub output_tokens: u64,
}

pub struct ResponseCache {
    entries: RwLock<HashMap<u64, CachedResponse>>,
    pub offline_mode: RwLock<bool>,
    pub max_entries: usize,
    cache_path: Option<std::path::PathBuf>,
}

impl ResponseCache {
    pub fn new() -> Self {
        let cache_path = dirs::data_local_dir()
            .map(|d| d.join("rmtide").join("response_cache.json"));
        let mut cache = Self {
            entries: RwLock::new(HashMap::new()),
            offline_mode: RwLock::new(false),
            max_entries: 500,
            cache_path,
        };
        cache.load_from_disk();
        cache
    }

    pub fn is_offline(&self) -> bool {
        *self.offline_mode.read()
    }

    pub fn toggle_offline(&self) -> bool {
        let mut mode = self.offline_mode.write();
        *mode = !*mode;
        *mode
    }

    /// Hash a prompt for cache key.
    pub fn hash_prompt(provider: &str, model: &str, messages: &str) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        provider.hash(&mut hasher);
        model.hash(&mut hasher);
        messages.hash(&mut hasher);
        std::hash::Hasher::finish(&hasher)
    }

    /// Look up a cached response.
    pub fn get(&self, hash: u64) -> Option<CachedResponse> {
        self.entries.read().get(&hash).cloned()
    }

    /// Store a response in the cache.
    pub fn store(&self, resp: CachedResponse) {
        let mut entries = self.entries.write();
        // Evict oldest if at capacity
        if entries.len() >= self.max_entries {
            if let Some((&oldest_key, _)) = entries.iter()
                .min_by_key(|(_, v)| v.timestamp)
            {
                entries.remove(&oldest_key);
            }
        }
        entries.insert(resp.prompt_hash, resp);
        drop(entries);
        self.save_to_disk();
    }

    fn load_from_disk(&mut self) {
        if let Some(ref path) = self.cache_path {
            if path.exists() {
                if let Ok(text) = std::fs::read_to_string(path) {
                    if let Ok(map) = serde_json::from_str::<HashMap<u64, CachedResponse>>(&text) {
                        *self.entries.write() = map;
                    }
                }
            }
        }
    }

    fn save_to_disk(&self) {
        if let Some(ref path) = self.cache_path {
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let entries = self.entries.read();
            if let Ok(json) = serde_json::to_string(&*entries) {
                let _ = std::fs::write(path, json);
            }
        }
    }

    pub fn size(&self) -> usize {
        self.entries.read().len()
    }
}

impl Default for ResponseCache {
    fn default() -> Self { Self::new() }
}
