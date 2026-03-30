#![allow(dead_code, unused_imports, unused_variables)]
use std::time::Instant;
use parking_lot::RwLock;
use tracing::{error, info, warn};

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum DbKind {
    Postgres,
    Mysql,
    Sqlite,
    Redis,
}

impl DbKind {
    pub fn label(&self) -> &'static str {
        match self {
            DbKind::Postgres => "PostgreSQL",
            DbKind::Mysql => "MySQL",
            DbKind::Sqlite => "SQLite",
            DbKind::Redis => "Redis",
        }
    }
}

#[derive(Clone, Debug)]
pub struct DbConnection {
    pub name: String,
    pub kind: DbKind,
    pub url: String, // connection string (masked for display)
}

impl DbConnection {
    pub fn masked_url(&self) -> String {
        // Mask password in connection strings like postgres://user:password@host/db
        let url = &self.url;
        if let Some(at_pos) = url.find('@') {
            if let Some(colon_pos) = url[..at_pos].rfind(':') {
                let before = &url[..colon_pos + 1];
                let after = &url[at_pos..];
                return format!("{}****{}", before, after);
            }
        }
        url.clone()
    }
}

#[derive(Clone, Debug)]
pub struct DbResult {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<String>>,
    pub affected_rows: Option<u64>,
    pub duration_ms: u64,
    pub error: Option<String>,
}

impl DbResult {
    pub fn error(msg: String, duration_ms: u64) -> Self {
        Self {
            columns: Vec::new(),
            rows: Vec::new(),
            affected_rows: None,
            duration_ms,
            error: Some(msg),
        }
    }
}

pub struct DbClient {
    pub connections: RwLock<Vec<DbConnection>>,
    pub active_connection: RwLock<Option<usize>>,
    pub query_history: RwLock<Vec<String>>,
}

impl DbClient {
    pub fn new() -> Self {
        Self {
            connections: RwLock::new(Vec::new()),
            active_connection: RwLock::new(None),
            query_history: RwLock::new(Vec::new()),
        }
    }

    pub fn add_connection(&self, conn: DbConnection) {
        self.connections.write().push(conn);
    }

    pub fn remove_connection(&self, idx: usize) {
        let mut conns = self.connections.write();
        if idx < conns.len() {
            conns.remove(idx);
        }
        // Adjust active connection
        let mut active = self.active_connection.write();
        if let Some(active_idx) = *active {
            if active_idx == idx {
                *active = None;
            } else if active_idx > idx {
                *active = Some(active_idx - 1);
            }
        }
    }

    /// Run a query by spawning the CLI tool as a subprocess.
    pub async fn query(&self, conn_idx: usize, sql: &str) -> DbResult {
        let conn = {
            let conns = self.connections.read();
            conns.get(conn_idx).cloned()
        };

        let Some(conn) = conn else {
            return DbResult::error("Connection not found".to_string(), 0);
        };

        // Add to history
        {
            let mut hist = self.query_history.write();
            hist.push(sql.to_string());
            if hist.len() > 100 {
                hist.remove(0);
            }
        }

        let start = Instant::now();

        match conn.kind {
            DbKind::Postgres => self.query_postgres(&conn.url, sql, start).await,
            DbKind::Mysql => self.query_mysql(&conn.url, sql, start).await,
            DbKind::Sqlite => self.query_sqlite(&conn.url, sql, start).await,
            DbKind::Redis => DbResult::error("Redis queries not supported via CLI".to_string(), 0),
        }
    }

    async fn query_postgres(&self, url: &str, sql: &str, start: Instant) -> DbResult {
        let output = tokio::process::Command::new("psql")
            .args(&[url, "-c", sql, "--csv", "--tuples-only", "--no-psqlrc"])
            .output()
            .await;

        let duration_ms = start.elapsed().as_millis() as u64;

        match output {
            Err(e) => DbResult::error(format!("Failed to run psql: {}", e), duration_ms),
            Ok(out) => {
                if !out.status.success() {
                    let err = String::from_utf8_lossy(&out.stderr).to_string();
                    return DbResult::error(err, duration_ms);
                }
                let stdout = String::from_utf8_lossy(&out.stdout).to_string();
                parse_csv_result(&stdout, duration_ms)
            }
        }
    }

    async fn query_mysql(&self, url: &str, sql: &str, start: Instant) -> DbResult {
        // mysql uses --host, --user, --password flags or a URL in some versions
        // Use --url if supported, otherwise parse the URL
        let output = tokio::process::Command::new("mysql")
            .args(&[url, "-e", sql, "--batch", "--silent"])
            .output()
            .await;

        let duration_ms = start.elapsed().as_millis() as u64;

        match output {
            Err(e) => DbResult::error(format!("Failed to run mysql: {}", e), duration_ms),
            Ok(out) => {
                if !out.status.success() {
                    let err = String::from_utf8_lossy(&out.stderr).to_string();
                    return DbResult::error(err, duration_ms);
                }
                let stdout = String::from_utf8_lossy(&out.stdout).to_string();
                parse_tsv_result(&stdout, duration_ms)
            }
        }
    }

    async fn query_sqlite(&self, path: &str, sql: &str, start: Instant) -> DbResult {
        // Extract file path from sqlite:// URL or use directly
        let file_path = path
            .trim_start_matches("sqlite://")
            .trim_start_matches("sqlite:///");

        let output = tokio::process::Command::new("sqlite3")
            .args(&[file_path, "-csv", "-header", sql])
            .output()
            .await;

        let duration_ms = start.elapsed().as_millis() as u64;

        match output {
            Err(e) => DbResult::error(format!("Failed to run sqlite3: {}", e), duration_ms),
            Ok(out) => {
                if !out.status.success() {
                    let err = String::from_utf8_lossy(&out.stderr).to_string();
                    return DbResult::error(err, duration_ms);
                }
                let stdout = String::from_utf8_lossy(&out.stdout).to_string();
                parse_csv_result_with_header(&stdout, duration_ms)
            }
        }
    }

    /// List tables for the active connection.
    pub async fn list_tables(&self, conn_idx: usize) -> Vec<String> {
        let conn = {
            let conns = self.connections.read();
            conns.get(conn_idx).cloned()
        };

        let Some(conn) = conn else {
            return Vec::new();
        };

        let start = Instant::now();
        let result = match conn.kind {
            DbKind::Postgres => {
                self.query_postgres(
                    &conn.url,
                    "SELECT tablename FROM pg_tables WHERE schemaname='public' ORDER BY tablename",
                    start,
                )
                .await
            }
            DbKind::Mysql => {
                self.query_mysql(&conn.url, "SHOW TABLES", start).await
            }
            DbKind::Sqlite => {
                self.query_sqlite(
                    &conn.url,
                    ".tables",
                    start,
                )
                .await
            }
            DbKind::Redis => return Vec::new(),
        };

        if result.error.is_some() {
            return Vec::new();
        }

        result
            .rows
            .into_iter()
            .filter_map(|row| row.into_iter().next())
            .collect()
    }
}

fn parse_csv_result(csv: &str, duration_ms: u64) -> DbResult {
    let mut lines = csv.lines().peekable();
    let mut columns = Vec::new();
    let mut rows = Vec::new();

    // First line is header (from psql --csv without --tuples-only for column names)
    // With --tuples-only, no header is emitted. We need to parse without header.
    // Parse as plain CSV rows.
    for line in lines {
        if line.trim().is_empty() {
            continue;
        }
        let fields = parse_csv_line(line);
        rows.push(fields);
    }

    DbResult {
        columns,
        rows,
        affected_rows: None,
        duration_ms,
        error: None,
    }
}

fn parse_csv_result_with_header(csv: &str, duration_ms: u64) -> DbResult {
    let mut lines = csv.lines().peekable();
    let mut columns = Vec::new();
    let mut rows = Vec::new();
    let mut first_line = true;

    for line in lines {
        if line.trim().is_empty() {
            continue;
        }
        let fields = parse_csv_line(line);
        if first_line {
            columns = fields;
            first_line = false;
        } else {
            rows.push(fields);
        }
    }

    DbResult {
        columns,
        rows,
        affected_rows: None,
        duration_ms,
        error: None,
    }
}

fn parse_tsv_result(tsv: &str, duration_ms: u64) -> DbResult {
    let mut lines = tsv.lines().peekable();
    let mut columns = Vec::new();
    let mut rows = Vec::new();
    let mut first_line = true;

    for line in lines {
        if line.trim().is_empty() {
            continue;
        }
        let fields: Vec<String> = line.split('\t').map(|s| s.to_string()).collect();
        if first_line {
            columns = fields;
            first_line = false;
        } else {
            rows.push(fields);
        }
    }

    DbResult {
        columns,
        rows,
        affected_rows: None,
        duration_ms,
        error: None,
    }
}

/// Parse a single CSV line handling quoted fields.
fn parse_csv_line(line: &str) -> Vec<String> {
    let mut fields = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut chars = line.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '"' if in_quotes => {
                if chars.peek() == Some(&'"') {
                    chars.next();
                    current.push('"');
                } else {
                    in_quotes = false;
                }
            }
            '"' => {
                in_quotes = true;
            }
            ',' if !in_quotes => {
                fields.push(current.trim().to_string());
                current = String::new();
            }
            _ => {
                current.push(c);
            }
        }
    }
    fields.push(current.trim().to_string());
    fields
}
