#![allow(dead_code)]
//! JSON-RPC 2.0 framing with Content-Length headers (LSP transport).
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};

#[derive(Debug, Serialize, Deserialize)]
pub struct RpcMessage {
    pub jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Value>, // present on requests + responses
    #[serde(skip_serializing_if = "Option::is_none")]
    pub method: Option<String>, // present on requests + notifications
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<RpcError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcError {
    pub code: i64,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl RpcMessage {
    pub fn request(id: u64, method: &str, params: Value) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            id: Some(Value::Number(id.into())),
            method: Some(method.into()),
            params: Some(params),
            result: None,
            error: None,
        }
    }

    pub fn notification(method: &str, params: Value) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            id: None,
            method: Some(method.into()),
            params: Some(params),
            result: None,
            error: None,
        }
    }

    pub fn is_notification(&self) -> bool {
        self.id.is_none() && self.method.is_some()
    }

    pub fn is_response(&self) -> bool {
        self.id.is_some() && self.method.is_none()
    }
}

/// Read one LSP message from an async buffered reader.
/// Returns `None` on clean EOF.
pub async fn read_message<R: AsyncReadExt + Unpin>(
    reader: &mut BufReader<R>,
) -> anyhow::Result<Option<RpcMessage>> {
    let mut content_length: Option<usize> = None;

    // Read headers until blank line
    loop {
        let mut line = String::new();
        let n = reader.read_line(&mut line).await?;
        if n == 0 {
            return Ok(None); // EOF
        }
        let trimmed = line.trim_end_matches(|c| c == '\r' || c == '\n');
        if trimmed.is_empty() {
            break;
        }
        if let Some(val) = trimmed.strip_prefix("Content-Length: ") {
            content_length = Some(val.trim().parse()?);
        }
        // Ignore Content-Type and other headers
    }

    let len = content_length
        .ok_or_else(|| anyhow::anyhow!("Missing Content-Length header"))?;
    let mut buf = vec![0u8; len];
    reader.read_exact(&mut buf).await?;
    let msg: RpcMessage = serde_json::from_slice(&buf)?;
    Ok(Some(msg))
}

/// Write one LSP message with Content-Length framing.
pub async fn write_message<W: AsyncWriteExt + Unpin>(
    writer: &mut W,
    msg: &RpcMessage,
) -> anyhow::Result<()> {
    let body = serde_json::to_string(msg)?;
    let header = format!("Content-Length: {}\r\n\r\n", body.len());
    writer.write_all(header.as_bytes()).await?;
    writer.write_all(body.as_bytes()).await?;
    writer.flush().await?;
    Ok(())
}
