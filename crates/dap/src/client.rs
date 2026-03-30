#![allow(dead_code, unused_imports, unused_variables)]
//! DAP client — manages the debug adapter process and protocol state.

use parking_lot::RwLock;
use tracing::{info, warn};

use crate::protocol::{StackFrame, Variable};

/// Current status of the debug adapter.
#[derive(Debug, Clone)]
pub enum DapStatus {
    Idle,
    Starting,
    Paused { thread_id: u64, reason: String },
    Running,
    Terminated,
}

impl DapStatus {
    pub fn label(&self) -> &str {
        match self {
            DapStatus::Idle       => "Idle",
            DapStatus::Starting   => "Starting",
            DapStatus::Paused { .. } => "Paused",
            DapStatus::Running    => "Running",
            DapStatus::Terminated => "Terminated",
        }
    }
}

/// Manages communication with a Debug Adapter Protocol server.
pub struct DapClient {
    pub status: RwLock<DapStatus>,
    pub stack_frames: RwLock<Vec<StackFrame>>,
    pub variables: RwLock<Vec<Variable>>,
    pub output_log: RwLock<Vec<String>>,
    seq_counter: RwLock<u64>,
}

impl DapClient {
    pub fn new() -> Self {
        Self {
            status: RwLock::new(DapStatus::Idle),
            stack_frames: RwLock::new(Vec::new()),
            variables: RwLock::new(Vec::new()),
            output_log: RwLock::new(Vec::new()),
            seq_counter: RwLock::new(1),
        }
    }

    fn next_seq(&self) -> u64 {
        let mut seq = self.seq_counter.write();
        let n = *seq;
        *seq += 1;
        n
    }

    /// Launch a debug session for the given program.
    pub async fn launch(
        &self,
        program: &str,
        args: Vec<String>,
        cwd: &str,
    ) -> anyhow::Result<()> {
        info!("DAP launch: program={} cwd={}", program, cwd);
        *self.status.write() = DapStatus::Starting;
        self.output_log
            .write()
            .push(format!("[DAP] Launching: {} {:?}", program, args));

        // In a full implementation this would:
        // 1. Spawn the debug adapter process (e.g. `rust-lldb`, `codelldb`, `debugpy`)
        // 2. Establish stdio/TCP transport
        // 3. Send initialize + launch requests
        // 4. Read events from the adapter and update status/frames/variables
        //
        // For now we simulate a running state.
        *self.status.write() = DapStatus::Running;
        self.output_log
            .write()
            .push(format!("[DAP] Program launched (stub)"));

        Ok(())
    }

    /// Pause the target.
    pub fn pause(&self) {
        let mut status = self.status.write();
        if matches!(*status, DapStatus::Running) {
            *status = DapStatus::Paused {
                thread_id: 1,
                reason: "pause".to_string(),
            };
            self.output_log.write().push("[DAP] Paused".to_string());
        }
    }

    /// Resume / continue execution.
    pub fn resume(&self) {
        let mut status = self.status.write();
        if matches!(*status, DapStatus::Paused { .. }) {
            *status = DapStatus::Running;
            self.output_log.write().push("[DAP] Resumed".to_string());
        }
    }

    /// Terminate the debug session.
    pub fn terminate(&self) {
        *self.status.write() = DapStatus::Terminated;
        self.stack_frames.write().clear();
        self.variables.write().clear();
        self.output_log
            .write()
            .push("[DAP] Session terminated".to_string());
    }

    /// Returns true if the adapter is currently paused at a breakpoint.
    pub fn is_paused(&self) -> bool {
        matches!(*self.status.read(), DapStatus::Paused { .. })
    }

    /// Append a line to the output log.
    pub fn log(&self, line: impl Into<String>) {
        self.output_log.write().push(line.into());
    }
}

impl Default for DapClient {
    fn default() -> Self {
        Self::new()
    }
}
