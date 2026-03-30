#![allow(dead_code, unused_imports, unused_variables)]
use std::pin::Pin;
use futures::{Stream, StreamExt};
use serde_json::{json, Value};

use crate::backend::{AiBackend, AiChunk, CompletionOptions, Message, ModelInfo, Role};

pub struct OpenAiBackend {
    api_key: Option<String>,
    base_url: String,
    http: reqwest::Client,
}

impl OpenAiBackend {
    pub fn new(api_key: Option<String>) -> Self {
        Self {
            api_key,
            base_url: "https://api.openai.com/v1".into(),
            http: reqwest::Client::new(),
        }
    }

    /// Custom base URL (for compatible APIs like Azure OpenAI, local proxies).
    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }
}

#[async_trait::async_trait]
impl AiBackend for OpenAiBackend {
    fn name(&self) -> &str {
        "openai"
    }

    fn default_model(&self) -> &str {
        "gpt-4o"
    }

    async fn health_check(&self) -> bool {
        self.api_key.is_some() || std::env::var("OPENAI_API_KEY").is_ok()
    }

    async fn list_models(&self) -> anyhow::Result<Vec<ModelInfo>> {
        Ok(vec![
            ModelInfo {
                id: "gpt-4o".into(),
                name: "GPT-4o".into(),
                context_window: Some(128_000),
            },
            ModelInfo {
                id: "gpt-4o-mini".into(),
                name: "GPT-4o Mini".into(),
                context_window: Some(128_000),
            },
            ModelInfo {
                id: "o1".into(),
                name: "o1".into(),
                context_window: Some(200_000),
            },
            ModelInfo {
                id: "o3-mini".into(),
                name: "o3-mini".into(),
                context_window: Some(200_000),
            },
        ])
    }

    async fn stream_completion(
        &self,
        messages: Vec<Message>,
        opts: CompletionOptions,
    ) -> anyhow::Result<Pin<Box<dyn Stream<Item = anyhow::Result<AiChunk>> + Send>>> {
        let api_key = self.api_key.clone()
            .or_else(|| std::env::var("OPENAI_API_KEY").ok())
            .ok_or_else(|| anyhow::anyhow!("No OPENAI_API_KEY set"))?;

        let model = opts
            .model
            .clone()
            .unwrap_or_else(|| self.default_model().to_string());
        let max_tokens = opts.max_tokens.unwrap_or(4096);

        let api_messages: Vec<Value> = messages
            .iter()
            .map(|m| {
                json!({
                    "role": match m.role {
                        Role::System    => "system",
                        Role::User      => "user",
                        Role::Assistant => "assistant",
                    },
                    "content": m.content,
                })
            })
            .collect();

        let mut body = json!({
            "model": model,
            "messages": api_messages,
            "max_tokens": max_tokens,
            "stream": true,
        });
        if let Some(temp) = opts.temperature {
            body["temperature"] = json!(temp);
        }

        let url = format!("{}/chat/completions", self.base_url);
        let resp = self.http
            .post(&url)
            .bearer_auth(&api_key)
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("OpenAI API error {status}: {text}"));
        }

        let stream = resp
            .bytes_stream()
            .map(|r| r.map_err(anyhow::Error::from))
            .flat_map(|bytes_result| match bytes_result {
                Err(e) => futures::stream::iter(vec![Err(e)]),
                Ok(bytes) => {
                    let text = String::from_utf8_lossy(&bytes).to_string();
                    let chunks: Vec<anyhow::Result<AiChunk>> = text
                        .lines()
                        .filter_map(|line| {
                            let data = line.strip_prefix("data: ")?;
                            if data == "[DONE]" {
                                return None;
                            }
                            let v: Value = serde_json::from_str(data).ok()?;
                            let delta = &v["choices"][0]["delta"];
                            let text = delta["content"].as_str()?.to_string();
                            let is_final = v["choices"][0]["finish_reason"].is_string();
                            let out_tokens =
                                v["usage"]["completion_tokens"].as_u64().map(|n| n as u32);
                            Some(Ok(AiChunk {
                                text,
                                is_final,
                                input_tokens: None,
                                output_tokens: out_tokens,
                            }))
                        })
                        .collect();
                    futures::stream::iter(chunks)
                }
            });

        Ok(Box::pin(stream))
    }
}
