#![allow(dead_code, unused_imports, unused_variables)]
use std::path::PathBuf;
use parking_lot::RwLock;
use tracing::{error, info, warn};

const RELOAD_SCRIPT: &str = r#"<script>/* rmtide live reload */</script>"#;

pub struct LiveServer {
    pub root: PathBuf,
    pub port: u16,
    pub running: RwLock<bool>,
    pub url: RwLock<Option<String>>,
    shutdown_tx: RwLock<Option<tokio::sync::oneshot::Sender<()>>>,
}

impl LiveServer {
    pub fn new(root: PathBuf, port: u16) -> Self {
        Self {
            root,
            port,
            running: RwLock::new(false),
            url: RwLock::new(None),
            shutdown_tx: RwLock::new(None),
        }
    }

    /// Spawn tokio TcpListener serving files from root.
    /// For .html files, inject a WebSocket reload <script> before </body>.
    /// For other files, serve bytes with guessed Content-Type.
    pub async fn start(&self) -> anyhow::Result<()> {
        if *self.running.read() {
            return Ok(());
        }

        let addr = format!("127.0.0.1:{}", self.port);
        let listener = tokio::net::TcpListener::bind(&addr).await?;
        let url = format!("http://{}", addr);

        *self.running.write() = true;
        *self.url.write() = Some(url.clone());

        info!("Live server started at {}", url);

        let root = self.root.clone();
        let (shutdown_tx, mut shutdown_rx) = tokio::sync::oneshot::channel::<()>();
        *self.shutdown_tx.write() = Some(shutdown_tx);

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    result = listener.accept() => {
                        match result {
                            Ok((stream, peer)) => {
                                let root_clone = root.clone();
                                tokio::spawn(async move {
                                    if let Err(e) = handle_connection(stream, root_clone).await {
                                        warn!("Live server connection error from {}: {}", peer, e);
                                    }
                                });
                            }
                            Err(e) => {
                                error!("Live server accept error: {}", e);
                                break;
                            }
                        }
                    }
                    _ = &mut shutdown_rx => {
                        info!("Live server shutting down");
                        break;
                    }
                }
            }
        });

        Ok(())
    }

    pub fn stop(&self) {
        *self.running.write() = false;
        *self.url.write() = None;
        let mut tx_guard = self.shutdown_tx.write();
        if let Some(tx) = tx_guard.take() {
            let _ = tx.send(());
        }
    }

    pub fn is_running(&self) -> bool {
        *self.running.read()
    }

    pub fn get_url(&self) -> Option<String> {
        self.url.read().clone()
    }
}

async fn handle_connection(
    mut stream: tokio::net::TcpStream,
    root: PathBuf,
) -> anyhow::Result<()> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let mut buf = vec![0u8; 4096];
    let n = stream.read(&mut buf).await?;
    if n == 0 {
        return Ok(());
    }

    let request = String::from_utf8_lossy(&buf[..n]);
    let request_line = request.lines().next().unwrap_or("");

    // Parse: "GET /path HTTP/1.1"
    let parts: Vec<&str> = request_line.splitn(3, ' ').collect();
    if parts.len() < 2 {
        send_response(&mut stream, 400, "Bad Request", "text/plain", b"Bad Request").await?;
        return Ok(());
    }

    let method = parts[0];
    let path_str = parts[1];

    if method != "GET" && method != "HEAD" {
        send_response(&mut stream, 405, "Method Not Allowed", "text/plain", b"Method Not Allowed").await?;
        return Ok(());
    }

    // Sanitize path
    let path_str = path_str.split('?').next().unwrap_or("/");
    let path_str = if path_str == "/" { "/index.html" } else { path_str };
    let relative = path_str.trim_start_matches('/');

    // Prevent directory traversal
    if relative.contains("..") {
        send_response(&mut stream, 403, "Forbidden", "text/plain", b"Forbidden").await?;
        return Ok(());
    }

    let file_path = root.join(relative);

    if !file_path.exists() || !file_path.is_file() {
        let body = format!("Not Found: {}", path_str);
        send_response(&mut stream, 404, "Not Found", "text/plain", body.as_bytes()).await?;
        return Ok(());
    }

    let content_type = guess_content_type(&file_path);
    let mut body = std::fs::read(&file_path)?;

    // Inject reload script into HTML
    if content_type == "text/html" {
        let mut html = String::from_utf8_lossy(&body).into_owned();
        if let Some(pos) = html.find("</body>") {
            html.insert_str(pos, RELOAD_SCRIPT);
            body = html.into_bytes();
        }
    }

    if method == "HEAD" {
        let header = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            content_type,
            body.len()
        );
        stream.write_all(header.as_bytes()).await?;
    } else {
        send_response(&mut stream, 200, "OK", content_type, &body).await?;
    }

    Ok(())
}

async fn send_response(
    stream: &mut tokio::net::TcpStream,
    status: u16,
    status_text: &str,
    content_type: &str,
    body: &[u8],
) -> anyhow::Result<()> {
    use tokio::io::AsyncWriteExt;
    let header = format!(
        "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        status,
        status_text,
        content_type,
        body.len()
    );
    stream.write_all(header.as_bytes()).await?;
    stream.write_all(body).await?;
    Ok(())
}

fn guess_content_type(path: &std::path::Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()) {
        Some("html") | Some("htm") => "text/html",
        Some("css") => "text/css",
        Some("js") | Some("mjs") => "application/javascript",
        Some("json") => "application/json",
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("svg") => "image/svg+xml",
        Some("ico") => "image/x-icon",
        Some("wasm") => "application/wasm",
        Some("txt") | Some("md") => "text/plain",
        Some("xml") => "application/xml",
        Some("pdf") => "application/pdf",
        Some("woff") => "font/woff",
        Some("woff2") => "font/woff2",
        Some("ttf") => "font/ttf",
        _ => "application/octet-stream",
    }
}
