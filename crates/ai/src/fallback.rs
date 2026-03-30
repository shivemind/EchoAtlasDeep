#![allow(dead_code, unused_imports, unused_variables)]
//! Multi-provider fallback chains for resilient AI requests.
use std::sync::Arc;
use tracing::{info, warn};

use super::backend::{AiBackend, AiChunk, CompletionOptions, Message};
use super::registry::BackendRegistry;

/// A fallback chain: try each backend in order until one succeeds.
pub struct FallbackChain {
    /// Ordered list of backend names to try.
    pub chain: Vec<String>,
    /// Maximum retries per backend.
    pub max_retries: usize,
}

impl FallbackChain {
    pub fn new(chain: Vec<String>) -> Self {
        Self { chain, max_retries: 2 }
    }

    /// Execute with fallback. Returns (backend_name_used, stream).
    pub async fn execute(
        &self,
        registry: &BackendRegistry,
        messages: Vec<Message>,
        opts: CompletionOptions,
    ) -> anyhow::Result<(String, String)> {
        let mut last_err = anyhow::anyhow!("No backends in fallback chain");

        for backend_name in &self.chain {
            let backend = match registry.get(backend_name) {
                Some(b) => b,
                None => {
                    warn!("Fallback: backend '{backend_name}' not registered, skipping");
                    continue;
                }
            };

            // Quick health check
            if !backend.health_check().await {
                warn!("Fallback: '{backend_name}' health check failed, skipping");
                continue;
            }

            match backend.complete(messages.clone(), opts.clone()).await {
                Ok(text) => {
                    if backend_name != &self.chain[0] {
                        info!("Fallback: used '{backend_name}' after primary failed");
                    }
                    return Ok((backend_name.clone(), text));
                }
                Err(e) => {
                    warn!("Fallback: '{backend_name}' error: {e}");
                    last_err = e;
                }
            }
        }

        Err(last_err)
    }
}

/// Parse a fallback chain from config string like "claude,openai,ollama".
pub fn parse_chain(s: &str) -> Vec<String> {
    s.split(',').map(|p| p.trim().to_string()).filter(|s| !s.is_empty()).collect()
}
