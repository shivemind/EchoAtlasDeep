#![allow(dead_code, unused_imports, unused_variables)]
use std::pin::Pin;
use futures::{Stream, StreamExt};
use serde_json::{json, Value};

use crate::backend::{AiBackend, AiChunk, CompletionOptions, Message, ModelInfo, Role};

pub struct OllamaBackend {
    base_url: String,
    http: reqwest::Client,
}

impl OllamaBackend {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            http: reqwest::Client::new(),
        }
    }

    pub fn local() -> Self {
        Self::new("http://localhost:11434")
    }
}

#[async_trait::async_trait]
impl AiBackend for OllamaBackend {
    fn name(&self) -> &str {
        "ollama"
    }

    fn default_model(&self) -> &str {
        "llama3"
    }

    async fn health_check(&self) -> bool {
        let url = format!("{}/api/tags", self.base_url);
        self.http
            .get(&url)
            .timeout(std::time::Duration::from_secs(2))
            .send()
            .await
            .is_ok()
    }

    async fn list_models(&self) -> anyhow::Result<Vec<ModelInfo>> {
        let url = format!("{}/api/tags", self.base_url);
        let resp: Value = self.http.get(&url).send().await?.json().await?;
        let models: Vec<ModelInfo> = resp["models"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .filter_map(|m| {
                let id = m["name"].as_str()?.to_string();
                Some(ModelInfo {
                    name: id.clone(),
                    id,
                    context_window: None,
                })
            })
            .collect();
        Ok(models)
    }

    async fn stream_completion(
        &self,
        messages: Vec<Message>,
        opts: CompletionOptions,
    ) -> anyhow::Result<Pin<Box<dyn Stream<Item = anyhow::Result<AiChunk>> + Send>>> {
        let model = opts
            .model
            .clone()
            .unwrap_or_else(|| self.default_model().to_string());
        let url = format!("{}/api/chat", self.base_url);

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

        let body = json!({
            "model": model,
            "messages": api_messages,
            "stream": true,
        });

        let resp = self.http.post(&url).json(&body).send().await?;
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("Ollama error {status}: {text}"));
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
                            let v: Value = serde_json::from_str(line).ok()?;
                            let content = v["message"]["content"].as_str()?.to_string();
                            let done = v["done"].as_bool().unwrap_or(false);
                            let out_tokens = v["eval_count"].as_u64().map(|n| n as u32);
                            Some(Ok(AiChunk {
                                text: content,
                                is_final: done,
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
