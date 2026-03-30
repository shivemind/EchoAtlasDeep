#![allow(dead_code, unused_imports, unused_variables)]
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Instant;
use parking_lot::RwLock;
use tracing::{error, info, warn};

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Patch,
    Delete,
    Head,
    Options,
}

impl std::fmt::Display for HttpMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let s = match self {
            HttpMethod::Get => "GET",
            HttpMethod::Post => "POST",
            HttpMethod::Put => "PUT",
            HttpMethod::Patch => "PATCH",
            HttpMethod::Delete => "DELETE",
            HttpMethod::Head => "HEAD",
            HttpMethod::Options => "OPTIONS",
        };
        write!(f, "{}", s)
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct HttpRequest {
    pub name: String,
    pub method: HttpMethod,
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: Option<String>,
    pub variables: HashMap<String, String>,
}

impl Default for HttpRequest {
    fn default() -> Self {
        Self {
            name: String::from("New Request"),
            method: HttpMethod::Get,
            url: String::new(),
            headers: Vec::new(),
            body: None,
            variables: HashMap::new(),
        }
    }
}

impl HttpRequest {
    /// Interpolate {{variable}} placeholders with values.
    pub fn render_url(&self, vars: &HashMap<String, String>) -> String {
        let mut url = self.url.clone();
        // Merge request-level variables and passed-in vars (passed-in take precedence)
        let mut merged = self.variables.clone();
        merged.extend(vars.iter().map(|(k, v)| (k.clone(), v.clone())));

        for (key, value) in &merged {
            let placeholder = format!("{{{{{}}}}}", key);
            url = url.replace(&placeholder, value);
        }
        url
    }

    pub fn render_body(&self, vars: &HashMap<String, String>) -> Option<String> {
        let body = self.body.as_ref()?;
        let mut result = body.clone();
        let mut merged = self.variables.clone();
        merged.extend(vars.iter().map(|(k, v)| (k.clone(), v.clone())));
        for (key, value) in &merged {
            let placeholder = format!("{{{{{}}}}}", key);
            result = result.replace(&placeholder, value);
        }
        Some(result)
    }
}

#[derive(Clone, Debug)]
pub struct HttpResponse {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: String,
    pub duration_ms: u64,
    pub size_bytes: usize,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct HttpCollection {
    pub name: String,
    pub requests: Vec<HttpRequest>,
}

pub struct HttpClient {
    client: reqwest::Client,
    pub collections: RwLock<Vec<HttpCollection>>,
    collection_dir: PathBuf,
    pub history: RwLock<Vec<(HttpRequest, HttpResponse)>>,
}

impl HttpClient {
    pub fn new(workspace_root: &std::path::Path) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        let collection_dir = workspace_root.join(".rmtide").join("http");

        Self {
            client,
            collections: RwLock::new(Vec::new()),
            collection_dir,
            history: RwLock::new(Vec::new()),
        }
    }

    pub fn load_collections(&self) -> anyhow::Result<()> {
        if !self.collection_dir.exists() {
            return Ok(());
        }

        let mut collections = self.collections.write();
        collections.clear();

        for entry in walkdir::WalkDir::new(&self.collection_dir)
            .max_depth(1)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("json") {
                if let Ok(content) = std::fs::read_to_string(path) {
                    if let Ok(col) = serde_json::from_str::<HttpCollection>(&content) {
                        collections.push(col);
                    }
                }
            }
        }

        Ok(())
    }

    pub fn save_collection(&self, collection: &HttpCollection) -> anyhow::Result<()> {
        std::fs::create_dir_all(&self.collection_dir)?;
        let filename = format!("{}.json", sanitize_filename(&collection.name));
        let path = self.collection_dir.join(filename);
        let json = serde_json::to_string_pretty(collection)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    pub async fn send(
        &self,
        req: &HttpRequest,
        global_vars: &HashMap<String, String>,
    ) -> anyhow::Result<HttpResponse> {
        let url = req.render_url(global_vars);
        let body = req.render_body(global_vars);

        let start = Instant::now();

        let method = match req.method {
            HttpMethod::Get => reqwest::Method::GET,
            HttpMethod::Post => reqwest::Method::POST,
            HttpMethod::Put => reqwest::Method::PUT,
            HttpMethod::Patch => reqwest::Method::PATCH,
            HttpMethod::Delete => reqwest::Method::DELETE,
            HttpMethod::Head => reqwest::Method::HEAD,
            HttpMethod::Options => reqwest::Method::OPTIONS,
        };

        let mut request_builder = self.client.request(method, &url);

        for (name, value) in &req.headers {
            request_builder = request_builder.header(name.as_str(), value.as_str());
        }

        if let Some(body_str) = body {
            request_builder = request_builder.body(body_str);
        }

        let response = request_builder.send().await?;
        let duration_ms = start.elapsed().as_millis() as u64;

        let status = response.status().as_u16();
        let headers: Vec<(String, String)> = response
            .headers()
            .iter()
            .map(|(k, v)| {
                (
                    k.to_string(),
                    v.to_str().unwrap_or("").to_string(),
                )
            })
            .collect();

        let body_bytes = response.bytes().await?;
        let size_bytes = body_bytes.len();
        let body_str = String::from_utf8_lossy(&body_bytes).into_owned();

        let http_response = HttpResponse {
            status,
            headers,
            body: body_str,
            duration_ms,
            size_bytes,
        };

        // Store in history
        {
            let mut hist = self.history.write();
            hist.push((req.clone(), http_response.clone()));
            // Keep last 100 entries
            if hist.len() > 100 {
                let drain_count = hist.len() - 100;
                hist.drain(0..drain_count);
            }
        }

        Ok(http_response)
    }

    pub fn get_collections(&self) -> Vec<HttpCollection> {
        self.collections.read().clone()
    }
}

fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}
