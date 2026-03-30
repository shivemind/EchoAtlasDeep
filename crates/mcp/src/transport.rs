#![allow(dead_code, unused_imports, unused_variables)]
use anyhow::{anyhow, Context};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use super::protocol::{RpcRequest, RpcResponse};

/// Read one Content-Length framed JSON-RPC message from `reader`.
pub async fn read_message<R: tokio::io::AsyncRead + Unpin>(
    reader: &mut BufReader<R>,
) -> anyhow::Result<RpcRequest> {
    // Read headers
    let mut content_length: Option<usize> = None;
    loop {
        let mut line = String::new();
        let n = reader.read_line(&mut line).await?;
        if n == 0 {
            return Err(anyhow!("EOF reading MCP message"));
        }
        let line = line.trim_end_matches(['\r', '\n']);
        if line.is_empty() {
            break; // blank line separates headers from body
        }
        if let Some(rest) = line.strip_prefix("Content-Length: ") {
            content_length = Some(rest.trim().parse::<usize>()
                .context("invalid Content-Length")?);
        }
    }
    let len = content_length.ok_or_else(|| anyhow!("Missing Content-Length"))?;
    let mut body = vec![0u8; len];
    reader.read_exact(&mut body).await?;
    let req: RpcRequest = serde_json::from_slice(&body)
        .context("failed to parse JSON-RPC request")?;
    Ok(req)
}

/// Write one Content-Length framed JSON-RPC response to `writer`.
pub async fn write_response<W: tokio::io::AsyncWrite + Unpin>(
    writer: &mut W,
    response: &RpcResponse,
) -> anyhow::Result<()> {
    let body = serde_json::to_vec(response)?;
    let header = format!("Content-Length: {}\r\n\r\n", body.len());
    writer.write_all(header.as_bytes()).await?;
    writer.write_all(&body).await?;
    writer.flush().await?;
    Ok(())
}

/// Write a JSON-RPC notification (no id) to `writer`.
pub async fn write_notification<W: tokio::io::AsyncWrite + Unpin>(
    writer: &mut W,
    method: &str,
    params: serde_json::Value,
) -> anyhow::Result<()> {
    let notif = serde_json::json!({
        "jsonrpc": "2.0",
        "method": method,
        "params": params
    });
    let body = serde_json::to_vec(&notif)?;
    let header = format!("Content-Length: {}\r\n\r\n", body.len());
    writer.write_all(header.as_bytes()).await?;
    writer.write_all(&body).await?;
    writer.flush().await?;
    Ok(())
}
