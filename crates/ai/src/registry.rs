#![allow(dead_code, unused_imports, unused_variables)]
use std::collections::HashMap;
use std::sync::Arc;

use parking_lot::RwLock;
use tracing::warn;

use crate::backend::{AiBackend, AiChunk, CompletionOptions, Message, ModelInfo};

/// Wraps a backend in an Arc for shared ownership.
pub type BoxedBackend = Arc<dyn AiBackend>;

pub struct BackendRegistry {
    backends: HashMap<String, BoxedBackend>,
    active: RwLock<String>,
    active_model: RwLock<Option<String>>,
    // token usage tracking
    total_input_tokens: RwLock<u64>,
    total_output_tokens: RwLock<u64>,
    last_input_tokens: RwLock<u32>,
    last_output_tokens: RwLock<u32>,
}

impl BackendRegistry {
    pub fn new() -> Self {
        Self {
            backends: HashMap::new(),
            active: RwLock::new(String::new()),
            active_model: RwLock::new(None),
            total_input_tokens: RwLock::new(0),
            total_output_tokens: RwLock::new(0),
            last_input_tokens: RwLock::new(0),
            last_output_tokens: RwLock::new(0),
        }
    }

    pub fn register(&mut self, backend: BoxedBackend) {
        let name = backend.name().to_string();
        if self.active.read().is_empty() {
            *self.active.write() = name.clone();
        }
        self.backends.insert(name, backend);
    }

    pub fn set_active(&self, name: &str) -> bool {
        if self.backends.contains_key(name) {
            *self.active.write() = name.to_string();
            *self.active_model.write() = None;
            true
        } else {
            false
        }
    }

    pub fn set_active_model(&self, model: &str) {
        *self.active_model.write() = Some(model.to_string());
    }

    pub fn active_name(&self) -> String {
        self.active.read().clone()
    }

    pub fn active_model(&self) -> Option<String> {
        self.active_model.read().clone()
    }

    pub fn active_display(&self) -> String {
        let name = self.active.read().clone();
        if let Some(model) = self.active_model.read().clone() {
            format!("{name}:{model}")
        } else {
            name
        }
    }

    pub fn get_active(&self) -> Option<BoxedBackend> {
        let name = self.active.read().clone();
        self.backends.get(&name).cloned()
    }

    pub fn get(&self, name: &str) -> Option<BoxedBackend> {
        self.backends.get(name).cloned()
    }

    pub fn all_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self.backends.keys().cloned().collect();
        names.sort();
        names
    }

    pub async fn list_all_models(&self) -> Vec<(String, Vec<ModelInfo>)> {
        let mut result = Vec::new();
        for (name, backend) in &self.backends {
            match backend.list_models().await {
                Ok(models) => result.push((name.clone(), models)),
                Err(e) => warn!("Failed to list models for {name}: {e}"),
            }
        }
        result
    }

    /// Track token usage from completed chunks.
    pub fn record_usage(&self, input: Option<u32>, output: Option<u32>) {
        if let Some(n) = input {
            *self.last_input_tokens.write() = n;
            *self.total_input_tokens.write() += n as u64;
        }
        if let Some(n) = output {
            *self.last_output_tokens.write() = n;
            *self.total_output_tokens.write() += n as u64;
        }
    }

    pub fn last_usage(&self) -> (u32, u32) {
        (*self.last_input_tokens.read(), *self.last_output_tokens.read())
    }

    pub fn total_usage(&self) -> (u64, u64) {
        (*self.total_input_tokens.read(), *self.total_output_tokens.read())
    }

    /// Create a clone of this registry (sharing the same Arc<dyn AiBackend> instances).
    pub fn clone_registry(&self) -> BackendRegistry {
        let active = self.active.read().clone();
        let active_model = self.active_model.read().clone();
        let reg = BackendRegistry {
            backends: self.backends.clone(),
            active: parking_lot::RwLock::new(active),
            active_model: parking_lot::RwLock::new(active_model),
            total_input_tokens: parking_lot::RwLock::new(*self.total_input_tokens.read()),
            total_output_tokens: parking_lot::RwLock::new(*self.total_output_tokens.read()),
            last_input_tokens: parking_lot::RwLock::new(*self.last_input_tokens.read()),
            last_output_tokens: parking_lot::RwLock::new(*self.last_output_tokens.read()),
        };
        reg
    }
}

impl Default for BackendRegistry {
    fn default() -> Self {
        Self::new()
    }
}
