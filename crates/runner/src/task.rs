#![allow(dead_code, unused_imports, unused_variables)]
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::SystemTime;

use parking_lot::RwLock;
use tokio::sync::broadcast;
use tracing::{error, info, warn};

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct TaskDef {
    pub name: String,
    pub command: String,
    pub cwd: String,
    pub env: HashMap<String, String>,
    pub watch: Vec<String>,
    pub depends: Vec<String>,
}

impl Default for TaskDef {
    fn default() -> Self {
        Self {
            name: String::new(),
            command: String::new(),
            cwd: String::from("."),
            env: HashMap::new(),
            watch: Vec::new(),
            depends: Vec::new(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TaskStatus {
    Idle,
    Running,
    Success,
    Failed,
    Watching,
}

impl TaskStatus {
    pub fn icon(&self) -> &'static str {
        match self {
            TaskStatus::Idle => "  ",
            TaskStatus::Running => "⏳",
            TaskStatus::Success => "✅",
            TaskStatus::Failed => "❌",
            TaskStatus::Watching => "👁 ",
        }
    }
}

#[derive(Clone, Debug)]
pub enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
    Raw,
}

#[derive(Clone, Debug)]
pub struct LogEntry {
    pub level: LogLevel,
    pub source: String,
    pub message: String,
    pub timestamp: u64,
}

#[derive(Clone, Debug)]
pub struct TaskRecord {
    pub def: TaskDef,
    pub status: TaskStatus,
    pub start_time: Option<u64>,
    pub duration_ms: Option<u64>,
    pub last_log: Option<String>,
    pub exit_code: Option<i32>,
}

pub struct TaskRunner {
    pub records: RwLock<Vec<TaskRecord>>,
    pub log_tx: broadcast::Sender<LogEntry>,
    workspace_root: PathBuf,
    cancel_txs: RwLock<HashMap<usize, tokio::sync::oneshot::Sender<()>>>,
}

impl TaskRunner {
    pub fn new(workspace_root: &std::path::Path) -> Self {
        let (log_tx, _) = broadcast::channel(4096);
        Self {
            records: RwLock::new(Vec::new()),
            log_tx,
            workspace_root: workspace_root.to_path_buf(),
            cancel_txs: RwLock::new(HashMap::new()),
        }
    }

    /// Load tasks.toml from workspace root.
    pub fn load_tasks(&self) -> anyhow::Result<()> {
        let tasks_path = self.workspace_root.join("tasks.toml");
        if !tasks_path.exists() {
            return Ok(());
        }
        let content = std::fs::read_to_string(&tasks_path)?;
        let parsed: toml::Value = toml::from_str(&content)?;

        let mut records = self.records.write();
        records.clear();

        if let Some(table) = parsed.as_table() {
            for (name, val) in table {
                if let Some(task_table) = val.as_table() {
                    let command = task_table
                        .get("command")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let cwd = task_table
                        .get("cwd")
                        .and_then(|v| v.as_str())
                        .unwrap_or(".")
                        .to_string();
                    let env: HashMap<String, String> = task_table
                        .get("env")
                        .and_then(|v| v.as_table())
                        .map(|t| {
                            t.iter()
                                .filter_map(|(k, v)| {
                                    v.as_str().map(|s| (k.clone(), s.to_string()))
                                })
                                .collect()
                        })
                        .unwrap_or_default();
                    let watch: Vec<String> = task_table
                        .get("watch")
                        .and_then(|v| v.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                                .collect()
                        })
                        .unwrap_or_default();
                    let depends: Vec<String> = task_table
                        .get("depends")
                        .and_then(|v| v.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                                .collect()
                        })
                        .unwrap_or_default();

                    let def = TaskDef {
                        name: name.clone(),
                        command,
                        cwd,
                        env,
                        watch,
                        depends,
                    };

                    records.push(TaskRecord {
                        def,
                        status: TaskStatus::Idle,
                        start_time: None,
                        duration_ms: None,
                        last_log: None,
                        exit_code: None,
                    });
                }
            }
        }

        Ok(())
    }

    pub fn task_count(&self) -> usize {
        self.records.read().len()
    }

    pub fn get_records(&self) -> Vec<TaskRecord> {
        self.records.read().clone()
    }

    /// Spawn a task by index.
    pub async fn run_task(&self, idx: usize) -> anyhow::Result<()> {
        let (def, cwd) = {
            let records = self.records.read();
            let rec = records.get(idx).ok_or_else(|| anyhow::anyhow!("Task index out of range"))?;
            (rec.def.clone(), rec.def.cwd.clone())
        };

        // Set status to Running
        {
            let mut records = self.records.write();
            if let Some(rec) = records.get_mut(idx) {
                rec.status = TaskStatus::Running;
                rec.start_time = Some(now_secs());
                rec.duration_ms = None;
                rec.exit_code = None;
            }
        }

        let start_ms = std::time::Instant::now();
        self.emit_log(LogLevel::Info, &def.name, &format!("Starting task: {}", def.command));

        // Resolve CWD
        let task_cwd = if cwd == "." || cwd.is_empty() {
            self.workspace_root.clone()
        } else {
            let p = std::path::PathBuf::from(&cwd);
            if p.is_absolute() {
                p
            } else {
                self.workspace_root.join(&cwd)
            }
        };

        // Parse command into program + args
        let parts: Vec<&str> = def.command.split_whitespace().collect();
        if parts.is_empty() {
            anyhow::bail!("Empty command");
        }
        let program = parts[0];
        let args = &parts[1..];

        let mut cmd = tokio::process::Command::new(program);
        cmd.args(args)
            .current_dir(&task_cwd)
            .envs(&def.env)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true);

        let mut child = match cmd.spawn() {
            Ok(c) => c,
            Err(e) => {
                let msg = format!("Failed to spawn '{}': {}", def.command, e);
                self.emit_log(LogLevel::Error, &def.name, &msg);
                let elapsed = start_ms.elapsed().as_millis() as u64;
                let mut records = self.records.write();
                if let Some(rec) = records.get_mut(idx) {
                    rec.status = TaskStatus::Failed;
                    rec.duration_ms = Some(elapsed);
                    rec.last_log = Some(msg.clone());
                }
                return Err(anyhow::anyhow!(msg));
            }
        };

        let (cancel_tx, mut cancel_rx) = tokio::sync::oneshot::channel::<()>();
        {
            let mut txs = self.cancel_txs.write();
            txs.insert(idx, cancel_tx);
        }

        // Read stdout
        let stdout = child.stdout.take();
        let stderr = child.stderr.take();

        let log_tx = self.log_tx.clone();
        let task_name = def.name.clone();

        // Spawn stdout reader
        if let Some(stdout) = stdout {
            use tokio::io::AsyncBufReadExt;
            let mut reader = tokio::io::BufReader::new(stdout).lines();
            let tx = log_tx.clone();
            let name = task_name.clone();
            tokio::spawn(async move {
                while let Ok(Some(line)) = reader.next_line().await {
                    let entry = LogEntry {
                        level: LogLevel::Raw,
                        source: name.clone(),
                        message: line,
                        timestamp: now_secs(),
                    };
                    let _ = tx.send(entry);
                }
            });
        }

        // Spawn stderr reader
        if let Some(stderr) = stderr {
            use tokio::io::AsyncBufReadExt;
            let mut reader = tokio::io::BufReader::new(stderr).lines();
            let tx = log_tx.clone();
            let name = task_name.clone();
            tokio::spawn(async move {
                while let Ok(Some(line)) = reader.next_line().await {
                    let entry = LogEntry {
                        level: LogLevel::Warn,
                        source: name.clone(),
                        message: line,
                        timestamp: now_secs(),
                    };
                    let _ = tx.send(entry);
                }
            });
        }

        // Wait for child or cancel
        let status = tokio::select! {
            result = child.wait() => result.ok(),
            _ = &mut cancel_rx => {
                let _ = child.kill().await;
                None
            }
        };

        let elapsed = start_ms.elapsed().as_millis() as u64;
        let exit_code = status.and_then(|s| s.code());
        let success = exit_code.map(|c| c == 0).unwrap_or(false);

        {
            let mut records = self.records.write();
            if let Some(rec) = records.get_mut(idx) {
                rec.status = if success { TaskStatus::Success } else { TaskStatus::Failed };
                rec.duration_ms = Some(elapsed);
                rec.exit_code = exit_code;
                rec.last_log = Some(format!(
                    "Exit code: {}",
                    exit_code.map(|c| c.to_string()).unwrap_or_else(|| "cancelled".to_string())
                ));
            }
        }

        let msg = format!(
            "Task '{}' finished in {}ms, exit: {}",
            def.name,
            elapsed,
            exit_code.map(|c| c.to_string()).unwrap_or_else(|| "cancelled".to_string())
        );
        let level = if success { LogLevel::Info } else { LogLevel::Error };
        self.emit_log(level, &def.name, &msg);

        Ok(())
    }

    /// Cancel a running task.
    pub fn cancel_task(&self, idx: usize) {
        let mut txs = self.cancel_txs.write();
        if let Some(tx) = txs.remove(&idx) {
            let _ = tx.send(());
        }
    }

    pub fn subscribe_logs(&self) -> broadcast::Receiver<LogEntry> {
        self.log_tx.subscribe()
    }

    pub fn emit_log(&self, level: LogLevel, source: &str, message: &str) {
        let entry = LogEntry {
            level,
            source: source.to_string(),
            message: message.to_string(),
            timestamp: now_secs(),
        };
        let _ = self.log_tx.send(entry);
    }
}
