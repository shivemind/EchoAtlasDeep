#![allow(dead_code, unused_imports, unused_variables)]
use std::time::SystemTime;
use parking_lot::RwLock;
use tracing::{info, warn};

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[derive(Clone, Debug)]
pub enum ProcessStatus {
    Starting,
    Running,
    Stopped,
    Crashed,
    Restarting,
}

impl ProcessStatus {
    pub fn label(&self) -> &'static str {
        match self {
            ProcessStatus::Starting => "Starting",
            ProcessStatus::Running => "Running",
            ProcessStatus::Stopped => "Stopped",
            ProcessStatus::Crashed => "Crashed",
            ProcessStatus::Restarting => "Restarting",
        }
    }
}

#[derive(Clone, Debug)]
pub struct ManagedProcess {
    pub id: usize,
    pub name: String,
    pub command: String,
    pub status: ProcessStatus,
    pub pid: Option<u32>,
    pub port: Option<u16>,
    pub started_at: u64,
    pub restart: bool,
}

impl ManagedProcess {
    pub fn uptime_secs(&self) -> u64 {
        now_secs().saturating_sub(self.started_at)
    }

    pub fn uptime_str(&self) -> String {
        let secs = self.uptime_secs();
        if secs < 60 {
            format!("{}s", secs)
        } else if secs < 3600 {
            format!("{}m{}s", secs / 60, secs % 60)
        } else {
            format!("{}h{}m", secs / 3600, (secs % 3600) / 60)
        }
    }
}

pub struct ProcessManager {
    pub processes: RwLock<Vec<ManagedProcess>>,
    id_counter: RwLock<usize>,
}

impl ProcessManager {
    pub fn new() -> Self {
        Self {
            processes: RwLock::new(Vec::new()),
            id_counter: RwLock::new(0),
        }
    }

    pub fn register(
        &self,
        name: &str,
        command: &str,
        pid: Option<u32>,
        port: Option<u16>,
        restart: bool,
    ) -> usize {
        let id = {
            let mut counter = self.id_counter.write();
            *counter += 1;
            *counter
        };

        let proc = ManagedProcess {
            id,
            name: name.to_string(),
            command: command.to_string(),
            status: ProcessStatus::Starting,
            pid,
            port,
            started_at: now_secs(),
            restart,
        };

        self.processes.write().push(proc);
        id
    }

    pub fn update_status(&self, id: usize, status: ProcessStatus) {
        let mut procs = self.processes.write();
        if let Some(p) = procs.iter_mut().find(|p| p.id == id) {
            p.status = status;
        }
    }

    /// Send SIGTERM (Unix) or TerminateProcess (Windows).
    pub fn kill(&self, id: usize) {
        let pid = {
            let procs = self.processes.read();
            procs.iter().find(|p| p.id == id).and_then(|p| p.pid)
        };

        if let Some(pid) = pid {
            #[cfg(windows)]
            {
                // On Windows use taskkill
                let _ = std::process::Command::new("taskkill")
                    .args(&["/PID", &pid.to_string(), "/F"])
                    .output();
            }
            #[cfg(unix)]
            {
                // Send SIGTERM on Unix via `kill` CLI to avoid needing libc
                let _ = std::process::Command::new("kill")
                    .args(&["-TERM", &pid.to_string()])
                    .output();
            }
            #[cfg(not(any(unix, windows)))]
            {
                warn!("kill() not implemented on this platform for pid {}", pid);
            }
        }

        self.update_status(id, ProcessStatus::Stopped);
    }

    pub fn list(&self) -> Vec<ManagedProcess> {
        self.processes.read().clone()
    }

    pub fn remove(&self, id: usize) {
        let mut procs = self.processes.write();
        procs.retain(|p| p.id != id);
    }
}
