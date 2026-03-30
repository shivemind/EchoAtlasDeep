/// Layered configuration: built-ins < system < user < project.
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub shell: String,
    pub scrollback_lines: usize,
    pub fps: u64,
    pub theme: String,
    pub ai: AiConfig,
    pub mcp: McpConfig,
    pub editor: EditorConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiConfig {
    /// Active backend: "claude" | "gemini" | "codex"
    pub backend: String,
    pub anthropic_api_key: Option<String>,
    pub google_api_key: Option<String>,
    pub openai_api_key: Option<String>,
    pub max_context_tokens: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpConfig {
    pub bind_addr: String,
    pub port: Option<u16>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditorConfig {
    pub tab_width: usize,
    pub expand_tabs: bool,
    pub line_numbers: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            shell: default_shell(),
            scrollback_lines: 10_000,
            fps: 60,
            theme: "catppuccin-mocha".into(),
            ai: AiConfig {
                backend: "claude".into(),
                anthropic_api_key: None,
                google_api_key: None,
                openai_api_key: None,
                max_context_tokens: 100_000,
            },
            mcp: McpConfig {
                bind_addr: "127.0.0.1".into(),
                port: None,
            },
            editor: EditorConfig {
                tab_width: 4,
                expand_tabs: true,
                line_numbers: true,
            },
        }
    }
}

impl Config {
    /// Load configuration from all layers, merging in order.
    pub fn load() -> Result<Self> {
        let mut cfg = Config::default();

        // User config: ~/.config/rmtide/config.toml
        if let Some(config_dir) = dirs::config_dir() {
            let user_path = config_dir.join("rmtide").join("config.toml");
            if user_path.exists() {
                let text = std::fs::read_to_string(&user_path)?;
                let user: toml::Value = toml::from_str(&text)?;
                cfg.merge_toml(&user);
            }
        }

        // Project config: .rmtide.toml in current dir.
        let project_path = PathBuf::from(".rmtide.toml");
        if project_path.exists() {
            let text = std::fs::read_to_string(&project_path)?;
            let project: toml::Value = toml::from_str(&text)?;
            cfg.merge_toml(&project);
        }

        Ok(cfg)
    }

    fn merge_toml(&mut self, val: &toml::Value) {
        // Simple key-by-key merge for top-level scalar fields.
        if let Some(t) = val.get("theme").and_then(|v| v.as_str()) {
            self.theme = t.to_string();
        }
        if let Some(s) = val.get("shell").and_then(|v| v.as_str()) {
            self.shell = s.to_string();
        }
        if let Some(n) = val.get("scrollback_lines").and_then(|v| v.as_integer()) {
            self.scrollback_lines = n as usize;
        }
        if let Some(n) = val.get("fps").and_then(|v| v.as_integer()) {
            self.fps = n as u64;
        }
        if let Some(ai) = val.get("ai") {
            if let Some(b) = ai.get("backend").and_then(|v| v.as_str()) {
                self.ai.backend = b.to_string();
            }
            if let Some(k) = ai.get("anthropic_api_key").and_then(|v| v.as_str()) {
                self.ai.anthropic_api_key = Some(k.to_string());
            }
            if let Some(k) = ai.get("google_api_key").and_then(|v| v.as_str()) {
                self.ai.google_api_key = Some(k.to_string());
            }
            if let Some(k) = ai.get("openai_api_key").and_then(|v| v.as_str()) {
                self.ai.openai_api_key = Some(k.to_string());
            }
        }
    }
}

fn default_shell() -> String {
    if cfg!(windows) {
        std::env::var("COMSPEC").unwrap_or_else(|_| "cmd.exe".into())
    } else {
        std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".into())
    }
}
