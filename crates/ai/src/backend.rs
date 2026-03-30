#![allow(dead_code, unused_imports, unused_variables)]
use std::pin::Pin;
use futures::Stream;

/// A chunk of streamed text from an AI model.
#[derive(Debug, Clone)]
pub struct AiChunk {
    pub text: String,
    pub is_final: bool,
    pub input_tokens: Option<u32>,
    pub output_tokens: Option<u32>,
}

/// A message in a conversation.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Message {
    pub role: Role,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Role {
    System,
    User,
    Assistant,
}

/// Options for a completion request.
#[derive(Debug, Clone)]
pub struct CompletionOptions {
    pub model: Option<String>,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
    pub system: Option<String>,
}

impl Default for CompletionOptions {
    fn default() -> Self {
        Self {
            model: None,
            max_tokens: Some(4096),
            temperature: None,
            system: None,
        }
    }
}

/// Info about a single model.
#[derive(Debug, Clone)]
pub struct ModelInfo {
    pub id: String,
    pub name: String,
    pub context_window: Option<u32>,
}

/// The core AI backend trait. Each backend implements this.
#[async_trait::async_trait]
pub trait AiBackend: Send + Sync {
    fn name(&self) -> &str;
    fn default_model(&self) -> &str;

    async fn list_models(&self) -> anyhow::Result<Vec<ModelInfo>>;
    async fn health_check(&self) -> bool;

    /// Stream a completion. Returns a stream of AiChunks.
    async fn stream_completion(
        &self,
        messages: Vec<Message>,
        opts: CompletionOptions,
    ) -> anyhow::Result<Pin<Box<dyn Stream<Item = anyhow::Result<AiChunk>> + Send>>>;

    /// Convenience: collect all chunks into a single string.
    async fn complete(
        &self,
        messages: Vec<Message>,
        opts: CompletionOptions,
    ) -> anyhow::Result<String> {
        use futures::StreamExt;
        let mut stream = self.stream_completion(messages, opts).await?;
        let mut result = String::new();
        while let Some(chunk) = stream.next().await {
            result.push_str(&chunk?.text);
        }
        Ok(result)
    }
}
