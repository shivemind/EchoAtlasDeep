#![allow(dead_code, unused_imports, unused_variables)]
//! AI backend integrations — Phase 4 (Points 31–40).
pub mod backend;
pub mod chat;
pub mod claude;
pub mod context;
pub mod gemini;
pub mod ghost;
pub mod ollama;
pub mod openai;
pub mod registry;
// Phase 8 — BYOK, Spend, Approvals, Fallback, Cache
pub mod keyring;
pub mod spend;
pub mod approval;
pub mod fallback;
pub mod cache;
// Phase 9 — Agent, Memory, Prompt Library
pub mod agent;
pub mod agent_memory;
pub mod prompt_library;

pub use backend::{AiBackend, AiChunk, CompletionOptions, Message, ModelInfo, Role};
pub use chat::ChatSession;
pub use context::EditorContext;
pub use ghost::GhostText;
pub use registry::BackendRegistry;
// Phase 8 re-exports
pub use keyring::{KeyVault, KeyId, KeyMeta};
pub use spend::{SpendTracker, ModelPricing, pricing_table, LatencyTier};
pub use approval::{ApprovalQueue, ApprovalModalState, ActionKind, ApprovalDecision};
pub use fallback::{FallbackChain, parse_chain};
pub use cache::ResponseCache;
// Phase 9 re-exports
pub use agent::{AgentSession, AgentStatus, AgentUpdate, AgentStep, ToolCall, SubAgentInfo, spawn_agent_loop};
pub use agent_memory::{AgentMemory, MemoryEntry};
pub use prompt_library::{PromptLibrary, PromptTemplate};

use std::sync::Arc;

/// Build a BackendRegistry from config values.
pub fn build_registry(
    anthropic_key: Option<String>,
    google_key: Option<String>,
    openai_key: Option<String>,
    active_backend: &str,
) -> BackendRegistry {
    let mut reg = BackendRegistry::new();
    reg.register(Arc::new(claude::ClaudeBackend::new(anthropic_key)));
    reg.register(Arc::new(gemini::GeminiBackend::new(google_key)));
    reg.register(Arc::new(openai::OpenAiBackend::new(openai_key)));
    reg.register(Arc::new(ollama::OllamaBackend::local()));
    reg.set_active(active_backend);
    reg
}
