use thiserror::Error;

#[derive(Debug, Error)]
pub enum EchoError {
    #[error("PTY error: {0}")]
    Pty(String),

    #[error("VT parse error: {0}")]
    Vt(String),

    #[error("LSP error: {0}")]
    Lsp(String),

    #[error("MCP error: {0}")]
    Mcp(String),

    #[error("AI backend error: {0}")]
    AiBackend(String),

    #[error("Editor error: {0}")]
    Editor(String),

    #[error("Git error: {0}")]
    Git(String),

    #[error("Config error: {0}")]
    Config(String),

    #[error("Plugin error: {0}")]
    Plugin(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serde(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, EchoError>;
