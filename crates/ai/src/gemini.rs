#![allow(dead_code, unused_imports, unused_variables)]
use std::pin::Pin;
use futures::{Stream, StreamExt};
use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

use crate::backend::{AiBackend, AiChunk, CompletionOptions, Message, ModelInfo, Role};

pub struct GeminiBackend {
    api_key: Option<String>,
    http: reqwest::Client,
}

impl GeminiBackend {
    pub fn new(api_key: Option<String>) -> Self {
        Self {
            api_key,
            http: reqwest::Client::new(),
        }
    }

    fn cli_available() -> bool {
        which("gemini")
    }

    async fn stream_via_cli(
        messages: Vec<Message>,
        opts: &CompletionOptions,
    ) -> anyhow::Result<Pin<Box<dyn Stream<Item = anyhow::Result<AiChunk>> + Send>>> {
        let prompt = messages
            .iter()
            .map(|m| m.content.clone())
            .collect::<Vec<_>>()
            .join("\n\n");
        let model = opts.model.clone().unwrap_or_else(|| "gemini-2.0-flash".to_string());

        let mut cmd = Command::new("gemini");
        cmd.args(["-m", &model, "-p", &prompt]);
        cmd.stdout(std::process::Stdio::piped())
           .stderr(std::process::Stdio::null())
           .stdin(std::process::Stdio::null());

        let mut child = cmd.spawn()?;
        let stdout = child.stdout.take().unwrap();
        let reader = BufReader::new(stdout);

        let stream = tokio_stream::wrappers::LinesStream::new(reader.lines())
            .map(|r| r.map_err(anyhow::Error::from))
            .map(|line_result| {
                line_result.map(|line| AiChunk {
                    text: line,
                    is_final: false,
                    input_tokens: None,
                    output_tokens: None,
                })
            });

        tokio::spawn(async move {
            let _ = child.wait().await;
        });
        Ok(Box::pin(stream))
    }

    async fn stream_via_api(
        &self,
        messages: Vec<Message>,
        opts: &CompletionOptions,
    ) -> anyhow::Result<Pin<Box<dyn Stream<Item = anyhow::Result<AiChunk>> + Send>>> {
        let api_key = self.api_key.clone()
            .or_else(|| std::env::var("GEMINI_API_KEY").ok())
            .ok_or_else(|| anyhow::anyhow!("No GEMINI_API_KEY and gemini CLI not found"))?;

        let model = opts.model.clone().unwrap_or_else(|| "gemini-2.0-flash".to_string());
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{model}:streamGenerateContent?alt=sse&key={api_key}"
        );

        let contents: Vec<Value> = messages
            .iter()
            .filter(|m| m.role != Role::System)
            .map(|m| {
                json!({
                    "role": if m.role == Role::User { "user" } else { "model" },
                    "parts": [{"text": m.content}],
                })
            })
            .collect();

        let mut body = json!({ "contents": contents });
        if let Some(system) = &opts.system {
            body["systemInstruction"] = json!({ "parts": [{"text": system}] });
        }
        if let Some(max_t) = opts.max_tokens {
            body["generationConfig"] = json!({ "maxOutputTokens": max_t });
        }

        let resp = self.http.post(&url).json(&body).send().await?;
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("Gemini API error {status}: {text}"));
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
                            let v: Value = serde_json::from_str(data).ok()?;
                            let text = v["candidates"][0]["content"]["parts"][0]["text"]
                                .as_str()?
                                .to_string();
                            Some(Ok(AiChunk {
                                text,
                                is_final: false,
                                input_tokens: None,
                                output_tokens: None,
                            }))
                        })
                        .collect();
                    futures::stream::iter(chunks)
                }
            });

        Ok(Box::pin(stream))
    }
}

#[async_trait::async_trait]
impl AiBackend for GeminiBackend {
    fn name(&self) -> &str {
        "gemini"
    }

    fn default_model(&self) -> &str {
        "gemini-2.0-flash"
    }

    async fn health_check(&self) -> bool {
        Self::cli_available()
            || self.api_key.is_some()
            || std::env::var("GEMINI_API_KEY").is_ok()
    }

    async fn list_models(&self) -> anyhow::Result<Vec<ModelInfo>> {
        Ok(vec![
            ModelInfo {
                id: "gemini-2.0-flash".into(),
                name: "Gemini 2.0 Flash".into(),
                context_window: Some(1_048_576),
            },
            ModelInfo {
                id: "gemini-2.0-pro".into(),
                name: "Gemini 2.0 Pro".into(),
                context_window: Some(2_097_152),
            },
            ModelInfo {
                id: "gemini-1.5-pro".into(),
                name: "Gemini 1.5 Pro".into(),
                context_window: Some(2_097_152),
            },
            ModelInfo {
                id: "gemini-1.5-flash".into(),
                name: "Gemini 1.5 Flash".into(),
                context_window: Some(1_048_576),
            },
        ])
    }

    async fn stream_completion(
        &self,
        messages: Vec<Message>,
        opts: CompletionOptions,
    ) -> anyhow::Result<Pin<Box<dyn Stream<Item = anyhow::Result<AiChunk>> + Send>>> {
        if Self::cli_available() {
            Self::stream_via_cli(messages, &opts).await
        } else {
            self.stream_via_api(messages, &opts).await
        }
    }
}

fn which(binary: &str) -> bool {
    let exts: &[&str] = if cfg!(windows) {
        &[".exe", ".cmd", ".bat", ""]
    } else {
        &[""]
    };
    for dir in std::env::var("PATH")
        .unwrap_or_default()
        .split(if cfg!(windows) { ';' } else { ':' })
    {
        for ext in exts {
            let p = std::path::Path::new(dir).join(format!("{binary}{ext}"));
            if p.exists() {
                return true;
            }
        }
    }
    false
}
