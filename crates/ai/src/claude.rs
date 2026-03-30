#![allow(dead_code, unused_imports, unused_variables)]
use std::pin::Pin;
use futures::{Stream, StreamExt};
use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tracing::{debug, warn};

use crate::backend::{AiBackend, AiChunk, CompletionOptions, Message, ModelInfo, Role};

pub struct ClaudeBackend {
    api_key: Option<String>,
    http: reqwest::Client,
}

impl ClaudeBackend {
    pub fn new(api_key: Option<String>) -> Self {
        Self {
            api_key,
            http: reqwest::Client::new(),
        }
    }

    /// Try to find the `claude` CLI binary.
    fn cli_available() -> bool {
        which("claude").is_some()
    }

    /// Stream via the claude CLI.
    async fn stream_via_cli(
        messages: Vec<Message>,
        opts: &CompletionOptions,
    ) -> anyhow::Result<Pin<Box<dyn Stream<Item = anyhow::Result<AiChunk>> + Send>>> {
        let prompt = messages_to_prompt(&messages);
        let model = opts.model.clone().unwrap_or_else(|| "claude-sonnet-4-6".to_string());
        let max_tokens = opts.max_tokens.unwrap_or(4096).to_string();

        let mut cmd = Command::new("claude");
        cmd.args(["--output-format", "stream-json", "--model", &model, "--max-tokens", &max_tokens]);
        if let Some(system) = &opts.system {
            cmd.args(["--system-prompt", system]);
        }
        cmd.arg("--print");
        cmd.arg(&prompt);
        cmd.stdout(std::process::Stdio::piped())
           .stderr(std::process::Stdio::null())
           .stdin(std::process::Stdio::null());

        let mut child = cmd.spawn()?;
        let stdout = child.stdout.take().unwrap();
        let reader = BufReader::new(stdout);

        let stream = tokio_stream::wrappers::LinesStream::new(reader.lines())
            .filter_map(|line_result| async move {
                let line = line_result.ok()?;
                let trimmed = line.trim().to_string();
                if trimmed.is_empty() {
                    return None;
                }
                let v: Value = serde_json::from_str(&trimmed).ok()?;
                parse_claude_cli_chunk(&v)
            })
            .map(|chunk| Ok(chunk));

        tokio::spawn(async move {
            let _ = child.wait().await;
        });

        Ok(Box::pin(stream))
    }

    /// Stream via Anthropic REST API.
    async fn stream_via_api(
        &self,
        messages: Vec<Message>,
        opts: &CompletionOptions,
    ) -> anyhow::Result<Pin<Box<dyn Stream<Item = anyhow::Result<AiChunk>> + Send>>> {
        let api_key = self.api_key.clone()
            .unwrap_or_else(|| std::env::var("ANTHROPIC_API_KEY").unwrap_or_default());

        if api_key.is_empty() {
            return Err(anyhow::anyhow!("No ANTHROPIC_API_KEY set and claude CLI not found"));
        }

        let model = opts.model.clone().unwrap_or_else(|| "claude-sonnet-4-6".to_string());
        let max_tokens = opts.max_tokens.unwrap_or(4096);

        let mut body = json!({
            "model": model,
            "max_tokens": max_tokens,
            "stream": true,
            "messages": messages_to_api_format(&messages),
        });
        if let Some(system) = &opts.system {
            body["system"] = json!(system);
        }
        if let Some(temp) = opts.temperature {
            body["temperature"] = json!(temp);
        }

        let resp = self.http
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("Anthropic API error {status}: {text}"));
        }

        let stream = resp
            .bytes_stream()
            .map(|r| r.map_err(anyhow::Error::from))
            .flat_map(|bytes_result| {
                match bytes_result {
                    Err(e) => futures::stream::iter(vec![Err(e)]),
                    Ok(bytes) => {
                        let lines_text = String::from_utf8_lossy(&bytes).to_string();
                        let chunks: Vec<anyhow::Result<AiChunk>> = lines_text
                            .lines()
                            .filter_map(|line| {
                                let line = line.trim();
                                if let Some(data) = line.strip_prefix("data: ") {
                                    if data == "[DONE]" {
                                        return None;
                                    }
                                    let v: Value = serde_json::from_str(data).ok()?;
                                    parse_anthropic_sse_chunk(&v).map(Ok)
                                } else {
                                    None
                                }
                            })
                            .collect();
                        futures::stream::iter(chunks)
                    }
                }
            });

        Ok(Box::pin(stream))
    }
}

#[async_trait::async_trait]
impl AiBackend for ClaudeBackend {
    fn name(&self) -> &str {
        "claude"
    }

    fn default_model(&self) -> &str {
        "claude-sonnet-4-6"
    }

    async fn health_check(&self) -> bool {
        Self::cli_available()
            || self.api_key.is_some()
            || std::env::var("ANTHROPIC_API_KEY").is_ok()
    }

    async fn list_models(&self) -> anyhow::Result<Vec<ModelInfo>> {
        Ok(vec![
            ModelInfo {
                id: "claude-opus-4-6".into(),
                name: "Claude Opus 4.6".into(),
                context_window: Some(200_000),
            },
            ModelInfo {
                id: "claude-sonnet-4-6".into(),
                name: "Claude Sonnet 4.6".into(),
                context_window: Some(200_000),
            },
            ModelInfo {
                id: "claude-haiku-4-5-20251001".into(),
                name: "Claude Haiku 4.5".into(),
                context_window: Some(200_000),
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

// ── Helpers ──────────────────────────────────────────────────────────────────

fn which(binary: &str) -> Option<std::path::PathBuf> {
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
                return Some(p);
            }
        }
    }
    None
}

fn messages_to_prompt(messages: &[Message]) -> String {
    messages
        .iter()
        .map(|m| match m.role {
            Role::System => format!("[System]: {}", m.content),
            Role::User => m.content.clone(),
            Role::Assistant => format!("[Assistant]: {}", m.content),
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn messages_to_api_format(messages: &[Message]) -> Value {
    let filtered: Vec<Value> = messages
        .iter()
        .filter(|m| m.role != Role::System)
        .map(|m| {
            json!({
                "role": if m.role == Role::User { "user" } else { "assistant" },
                "content": m.content,
            })
        })
        .collect();
    json!(filtered)
}

fn parse_claude_cli_chunk(v: &Value) -> Option<AiChunk> {
    match v["type"].as_str()? {
        "content_block_delta" => {
            let text = v["delta"]["text"].as_str()?.to_string();
            Some(AiChunk {
                text,
                is_final: false,
                input_tokens: None,
                output_tokens: None,
            })
        }
        "message_stop" => Some(AiChunk {
            text: String::new(),
            is_final: true,
            input_tokens: None,
            output_tokens: None,
        }),
        "message_delta" => {
            let out = v["usage"]["output_tokens"].as_u64().map(|n| n as u32);
            Some(AiChunk {
                text: String::new(),
                is_final: false,
                input_tokens: None,
                output_tokens: out,
            })
        }
        _ => None,
    }
}

fn parse_anthropic_sse_chunk(v: &Value) -> Option<AiChunk> {
    match v["type"].as_str()? {
        "content_block_delta" => {
            let text = v["delta"]["text"].as_str()?.to_string();
            Some(AiChunk {
                text,
                is_final: false,
                input_tokens: None,
                output_tokens: None,
            })
        }
        "message_stop" => Some(AiChunk {
            text: String::new(),
            is_final: true,
            input_tokens: None,
            output_tokens: None,
        }),
        "message_delta" => {
            let out = v["usage"]["output_tokens"].as_u64().map(|n| n as u32);
            Some(AiChunk {
                text: String::new(),
                is_final: false,
                input_tokens: None,
                output_tokens: out,
            })
        }
        _ => None,
    }
}
