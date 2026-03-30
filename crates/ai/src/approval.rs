#![allow(dead_code, unused_imports, unused_variables)]
//! Human-in-the-loop approval workflow for AI-initiated mutations.
use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, oneshot};

/// The kind of action requiring approval.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActionKind {
    ReadFile { path: PathBuf },
    WriteFile { path: PathBuf, preview: String },
    DeleteFile { path: PathBuf },
    MoveFile { from: PathBuf, to: PathBuf },
    RunCommand { command: String, cwd: Option<PathBuf> },
    GitCommit { message: String, staged_count: usize },
    GitPush { remote: String, branch: String },
    NetworkRequest { url: String, method: String },
    ApplyEdit { path: PathBuf, line_range: (usize, usize), preview: String },
    Custom { name: String, description: String },
}

impl ActionKind {
    /// True if this action type always requires explicit approval (never auto-approved).
    pub fn always_requires_approval(&self) -> bool {
        matches!(self,
            ActionKind::DeleteFile { .. }
            | ActionKind::GitPush { .. }
            | ActionKind::RunCommand { .. }
        )
    }

    /// Display name for the action.
    pub fn display_name(&self) -> &'static str {
        match self {
            ActionKind::ReadFile { .. }      => "Read File",
            ActionKind::WriteFile { .. }     => "Write File",
            ActionKind::DeleteFile { .. }    => "Delete File",
            ActionKind::MoveFile { .. }      => "Move File",
            ActionKind::RunCommand { .. }    => "Run Command",
            ActionKind::GitCommit { .. }     => "Git Commit",
            ActionKind::GitPush { .. }       => "Git Push",
            ActionKind::NetworkRequest { .. }=> "Network Request",
            ActionKind::ApplyEdit { .. }     => "Apply Edit",
            ActionKind::Custom { .. }        => "Custom Action",
        }
    }

    /// Risk level badge.
    pub fn risk_badge(&self) -> &'static str {
        match self {
            ActionKind::ReadFile { .. }      => "low",
            ActionKind::WriteFile { .. }     => "medium",
            ActionKind::ApplyEdit { .. }     => "medium",
            ActionKind::DeleteFile { .. }    => "high",
            ActionKind::MoveFile { .. }      => "medium",
            ActionKind::RunCommand { .. }    => "high",
            ActionKind::GitCommit { .. }     => "medium",
            ActionKind::GitPush { .. }       => "high",
            ActionKind::NetworkRequest { .. }=> "medium",
            ActionKind::Custom { .. }        => "medium",
        }
    }
}

/// The user's decision on an approval request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApprovalDecision {
    Approve,
    Deny,
    ApproveAll,  // approve this and all subsequent in this batch
    DenyAll,     // deny this and all subsequent in this batch
}

/// One pending approval request.
pub struct ApprovalRequest {
    pub id: uuid::Uuid,
    pub action: ActionKind,
    pub agent_name: String,
    pub responder: oneshot::Sender<ApprovalDecision>,
}

/// The approval queue — the main loop reads from this.
pub struct ApprovalQueue {
    pub tx: mpsc::UnboundedSender<ApprovalRequest>,
    pub rx: parking_lot::Mutex<mpsc::UnboundedReceiver<ApprovalRequest>>,
}

impl ApprovalQueue {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        Self { tx, rx: parking_lot::Mutex::new(rx) }
    }

    /// Request approval; blocks until the user responds.
    pub async fn request(
        &self,
        action: ActionKind,
        agent_name: &str,
    ) -> ApprovalDecision {
        let (resp_tx, resp_rx) = oneshot::channel();
        let req = ApprovalRequest {
            id: uuid::Uuid::new_v4(),
            action,
            agent_name: agent_name.to_string(),
            responder: resp_tx,
        };
        let _ = self.tx.send(req);
        resp_rx.await.unwrap_or(ApprovalDecision::Deny)
    }
}

/// State of the currently-visible approval modal.
#[derive(Debug, Clone)]
pub struct ApprovalModalState {
    pub pending: Option<PendingApproval>,
    pub log: Vec<ApprovalLogEntry>,
    pub approve_all: bool,  // if true, auto-approve all remaining in batch
}

#[derive(Debug, Clone)]
pub struct PendingApproval {
    pub id: String,
    pub action_name: String,
    pub risk: String,
    pub description: String,
    pub preview: Option<String>,
    pub agent: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalLogEntry {
    pub timestamp: u64,
    pub action: String,
    pub decision: String,
    pub agent: String,
}

impl ApprovalModalState {
    pub fn new() -> Self {
        Self { pending: None, log: Vec::new(), approve_all: false }
    }

    pub fn set_pending(&mut self, req: &ApprovalRequest) {
        let description = match &req.action {
            ActionKind::WriteFile { path, preview } =>
                format!("Write to: {}\nPreview:\n{}", path.display(), &preview[..preview.len().min(200)]),
            ActionKind::ReadFile { path } =>
                format!("Read: {}", path.display()),
            ActionKind::DeleteFile { path } =>
                format!("DELETE: {}", path.display()),
            ActionKind::RunCommand { command, cwd } =>
                format!("$ {command}\n  in: {}", cwd.as_ref().map(|p| p.display().to_string()).unwrap_or_default()),
            ActionKind::GitCommit { message, staged_count } =>
                format!("Commit {staged_count} files:\n\"{message}\""),
            ActionKind::ApplyEdit { path, line_range, preview } =>
                format!("Edit {}:{}-{}\n{}", path.display(), line_range.0, line_range.1, &preview[..preview.len().min(300)]),
            ActionKind::Custom { name, description } =>
                format!("{name}: {description}"),
            _ => format!("{}", req.action.display_name()),
        };
        self.pending = Some(PendingApproval {
            id: req.id.to_string(),
            action_name: req.action.display_name().to_string(),
            risk: req.action.risk_badge().to_string(),
            description,
            preview: None,
            agent: req.agent_name.clone(),
        });
    }

    pub fn record_decision(&mut self, decision: &str, action: &str, agent: &str) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        self.log.push(ApprovalLogEntry {
            timestamp: now,
            action: action.to_string(),
            decision: decision.to_string(),
            agent: agent.to_string(),
        });
    }
}
