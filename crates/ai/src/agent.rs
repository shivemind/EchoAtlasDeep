#![allow(dead_code, unused_imports, unused_variables)]
//! Agent session with plan→act→verify loop.
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tracing::{debug, warn};

use crate::backend::{CompletionOptions, Message, Role};
use crate::registry::BackendRegistry;
use crate::approval::{ApprovalQueue, ActionKind, ApprovalDecision};
use crate::spend::SpendTracker;

// ── Types ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentStep {
    Plan(Vec<String>),
    Act { step_idx: usize, description: String },
    Verify,
    Complete { summary: String },
    Failed { reason: String },
    Interrupted,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentStatus {
    Idle,
    Planning,
    Acting,
    Verifying,
    WaitingApproval,
    Complete,
    Failed,
    Interrupted,
}

impl AgentStatus {
    pub fn label(&self) -> &'static str {
        match self {
            AgentStatus::Idle            => "Idle",
            AgentStatus::Planning        => "Planning",
            AgentStatus::Acting          => "Acting",
            AgentStatus::Verifying       => "Verifying",
            AgentStatus::WaitingApproval => "Waiting Approval",
            AgentStatus::Complete        => "Complete",
            AgentStatus::Failed          => "Failed",
            AgentStatus::Interrupted     => "Interrupted",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub tool: String,
    pub args: String,
    pub result: Option<String>,
    pub duration_ms: u64,
    pub cost_usd: f64,
    pub error: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubAgentInfo {
    pub role: String,
    pub status: AgentStatus,
    pub progress: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSession {
    pub task: String,
    pub plan: Vec<String>,
    pub plan_editable: bool,
    pub current_step: usize,
    pub status: AgentStatus,
    pub tool_trace: Vec<ToolCall>,
    pub conversation: Vec<Message>,
    pub sub_agents: Vec<SubAgentInfo>,
    pub session_id: String,
    pub created_at: u64,
    #[serde(skip)]
    pub interrupt_flag: Arc<AtomicBool>,
}

// ── AgentSession impl ────────────────────────────────────────────────────────

impl AgentSession {
    pub fn new(task: &str) -> Self {
        let session_id = uuid::Uuid::new_v4().to_string();
        let created_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        Self {
            task: task.to_string(),
            plan: Vec::new(),
            plan_editable: true,
            current_step: 0,
            status: AgentStatus::Idle,
            tool_trace: Vec::new(),
            conversation: Vec::new(),
            sub_agents: Vec::new(),
            session_id,
            created_at,
            interrupt_flag: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Signal the agent loop to stop after the current step.
    pub fn interrupt(&self) {
        self.interrupt_flag.store(true, Ordering::SeqCst);
    }

    /// Clear the interrupt flag and set status back to Acting (or Planning).
    pub fn resume(&mut self) {
        self.interrupt_flag.store(false, Ordering::SeqCst);
        if self.status == AgentStatus::Interrupted {
            if self.plan.is_empty() {
                self.status = AgentStatus::Planning;
            } else {
                self.status = AgentStatus::Acting;
            }
        }
    }

    /// Lock the plan and start execution.
    pub fn confirm_plan(&mut self) {
        self.plan_editable = false;
        self.current_step = 0;
        self.status = AgentStatus::Acting;
    }

    /// Edit a plan step text.
    pub fn edit_plan_step(&mut self, idx: usize, new_text: String) {
        if self.plan_editable {
            if let Some(step) = self.plan.get_mut(idx) {
                *step = new_text;
            }
        }
    }

    /// Add a new plan step at the end.
    pub fn add_plan_step(&mut self, text: String) {
        if self.plan_editable {
            self.plan.push(text);
        }
    }

    /// Remove a plan step by index.
    pub fn remove_plan_step(&mut self, idx: usize) {
        if self.plan_editable && idx < self.plan.len() {
            self.plan.remove(idx);
        }
    }

    /// Record a tool call in the trace.
    pub fn record_tool_call(
        &mut self,
        tool: &str,
        args: &str,
        result: Option<&str>,
        duration_ms: u64,
        cost_usd: f64,
        error: bool,
    ) {
        self.tool_trace.push(ToolCall {
            tool: tool.to_string(),
            args: args.to_string(),
            result: result.map(|s| s.to_string()),
            duration_ms,
            cost_usd,
            error,
        });
    }

    /// Persist session to `.rmtide/agent-session.json`.
    pub fn save(&self, workspace_root: &Path) -> anyhow::Result<()> {
        let dir = workspace_root.join(".rmtide");
        std::fs::create_dir_all(&dir)?;
        let path = dir.join("agent-session.json");
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(&path, json)?;
        Ok(())
    }

    /// Load from `.rmtide/agent-session.json`.
    pub fn load(workspace_root: &Path) -> anyhow::Result<Self> {
        let path = workspace_root.join(".rmtide").join("agent-session.json");
        let json = std::fs::read_to_string(&path)?;
        let mut session: Self = serde_json::from_str(&json)?;
        // Re-create the atomic flag (not serialized)
        session.interrupt_flag = Arc::new(AtomicBool::new(false));
        Ok(session)
    }

    /// Drive a single agentic step: call AI, parse response, update state.
    pub async fn drive_step(
        &mut self,
        registry: &BackendRegistry,
        approval_queue: &ApprovalQueue,
    ) -> anyhow::Result<()> {
        if self.interrupt_flag.load(Ordering::SeqCst) {
            self.status = AgentStatus::Interrupted;
            return Ok(());
        }

        match self.status.clone() {
            AgentStatus::Idle | AgentStatus::Planning => {
                self.status = AgentStatus::Planning;
                let plan = self.generate_plan(registry).await?;
                self.plan = plan;
                self.plan_editable = true;
                self.status = AgentStatus::WaitingApproval;
            }
            AgentStatus::Acting => {
                let step_idx = self.current_step;
                let n_steps = self.plan.len();
                if step_idx >= n_steps {
                    self.status = AgentStatus::Verifying;
                    return Ok(());
                }
                let description = self.plan[step_idx].clone();
                // Ask for approval before acting
                let action = ActionKind::Custom {
                    name: format!("Step {}/{}", step_idx + 1, n_steps),
                    description: description.clone(),
                };
                let decision = approval_queue.request(action, "agent").await;
                match decision {
                    ApprovalDecision::Deny | ApprovalDecision::DenyAll => {
                        self.status = AgentStatus::Interrupted;
                        return Ok(());
                    }
                    _ => {}
                }
                // Execute the step via AI
                self.execute_step(registry, step_idx, &description).await?;
                self.current_step += 1;
                if self.current_step >= self.plan.len() {
                    self.status = AgentStatus::Verifying;
                }
            }
            AgentStatus::Verifying => {
                let summary = self.verify_completion(registry).await?;
                self.status = AgentStatus::Complete;
                // Store summary as a final tool call marker
                self.record_tool_call("verify", "", Some(&summary), 0, 0.0, false);
            }
            AgentStatus::WaitingApproval => {
                // Nothing to do — waiting for user to confirm plan
            }
            AgentStatus::Complete | AgentStatus::Failed | AgentStatus::Interrupted => {
                // Terminal states
            }
        }
        Ok(())
    }

    // ── Private helpers ──────────────────────────────────────────────────────

    async fn generate_plan(&mut self, registry: &BackendRegistry) -> anyhow::Result<Vec<String>> {
        let backend = match registry.get_active() {
            Some(b) => b,
            None => return Ok(vec!["[No AI backend available]".to_string()]),
        };
        let system_msg = "You are an expert software engineering agent. \
            Given a task, output a numbered plan of concrete steps to accomplish it. \
            Each step should be specific and actionable. Output ONLY the steps, one per line, \
            prefixed with the step number and a period (e.g. '1. Do X').";
        let user_content = format!("Task: {}\n\nProvide a detailed plan:", self.task);
        let messages = vec![
            Message { role: Role::System, content: system_msg.to_string() },
            Message { role: Role::User, content: user_content },
        ];
        let opts = CompletionOptions {
            model: None,
            max_tokens: Some(1024),
            temperature: None,
            system: None,
        };
        let text = backend.complete(messages, opts).await.unwrap_or_else(|e| {
            format!("1. [Plan generation failed: {e}]")
        });
        let plan: Vec<String> = text
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(|l| {
                // Strip leading "1. " or "12. " style prefixes
                let stripped = l.trim();
                // Use char iteration for safety
                let chars: Vec<char> = stripped.chars().collect();
                if chars.len() > 3
                    && chars[0].is_ascii_digit()
                    && chars[1] == '.'
                    && chars[2] == ' '
                {
                    return chars[3..].iter().collect();
                }
                if chars.len() > 4
                    && chars[0].is_ascii_digit()
                    && chars[1].is_ascii_digit()
                    && chars[2] == '.'
                    && chars[3] == ' '
                {
                    return chars[4..].iter().collect();
                }
                stripped.to_string()
            })
            .collect();
        // Push conversation record
        self.conversation.push(Message { role: Role::Assistant, content: text });
        Ok(plan)
    }

    async fn execute_step(
        &mut self,
        registry: &BackendRegistry,
        step_idx: usize,
        description: &str,
    ) -> anyhow::Result<()> {
        let backend = match registry.get_active() {
            Some(b) => b,
            None => return Ok(()),
        };
        let start = std::time::Instant::now();
        let user_content = format!(
            "Execute step {}: {}\n\nReport what was done concisely.",
            step_idx + 1,
            description
        );
        let messages = vec![Message { role: Role::User, content: user_content }];
        let opts = CompletionOptions::default();
        let result = backend.complete(messages, opts).await;
        let elapsed = start.elapsed().as_millis() as u64;
        match result {
            Ok(text) => {
                self.conversation.push(Message { role: Role::Assistant, content: text.clone() });
                self.record_tool_call("ai_step", description, Some(&text), elapsed, 0.0, false);
            }
            Err(e) => {
                let err_msg = e.to_string();
                self.record_tool_call("ai_step", description, Some(&err_msg), elapsed, 0.0, true);
            }
        }
        Ok(())
    }

    async fn verify_completion(&mut self, registry: &BackendRegistry) -> anyhow::Result<String> {
        let backend = match registry.get_active() {
            Some(b) => b,
            None => return Ok("Verification skipped (no backend).".to_string()),
        };
        let plan_text = self.plan.iter().enumerate()
            .map(|(i, s)| format!("{}. {}", i + 1, s))
            .collect::<Vec<_>>()
            .join("\n");
        let user_content = format!(
            "I have executed the following plan for the task \"{}\":\n{}\n\n\
             Please verify the work is complete and provide a brief summary.",
            self.task, plan_text
        );
        let messages = vec![Message { role: Role::User, content: user_content }];
        let opts = CompletionOptions::default();
        let summary = backend.complete(messages, opts).await
            .unwrap_or_else(|e| format!("Verification failed: {e}"));
        Ok(summary)
    }
}

// ── AgentUpdate ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum AgentUpdate {
    StepComplete(usize),
    ToolCalled(ToolCall),
    PlanReady(Vec<String>),
    StatusChanged(AgentStatus),
    Complete(String),
    Failed(String),
}

// ── spawn_agent_loop ─────────────────────────────────────────────────────────

/// Run the full agentic loop in a background tokio task, sending updates via channel.
pub fn spawn_agent_loop(
    session: Arc<tokio::sync::Mutex<AgentSession>>,
    registry: Arc<BackendRegistry>,
    approval_queue: Arc<ApprovalQueue>,
    spend: Arc<SpendTracker>,
    workspace_root: PathBuf,
    update_tx: mpsc::UnboundedSender<AgentUpdate>,
) {
    tokio::spawn(async move {
        loop {
            let (status, interrupted) = {
                let s = session.lock().await;
                (s.status.clone(), s.interrupt_flag.load(Ordering::SeqCst))
            };

            if interrupted {
                let mut s = session.lock().await;
                s.status = AgentStatus::Interrupted;
                let _ = update_tx.send(AgentUpdate::StatusChanged(AgentStatus::Interrupted));
                break;
            }

            match status {
                AgentStatus::Complete => {
                    let summary = {
                        let s = session.lock().await;
                        s.tool_trace.last()
                            .and_then(|t| t.result.clone())
                            .unwrap_or_else(|| "Done".to_string())
                    };
                    let _ = update_tx.send(AgentUpdate::Complete(summary));
                    break;
                }
                AgentStatus::Failed => {
                    let _ = update_tx.send(AgentUpdate::Failed("Agent failed".to_string()));
                    break;
                }
                AgentStatus::Interrupted => {
                    let _ = update_tx.send(AgentUpdate::StatusChanged(AgentStatus::Interrupted));
                    break;
                }
                AgentStatus::WaitingApproval => {
                    // Emit PlanReady if we have a plan
                    let plan = {
                        let s = session.lock().await;
                        s.plan.clone()
                    };
                    if !plan.is_empty() {
                        let _ = update_tx.send(AgentUpdate::PlanReady(plan));
                    }
                    // Wait briefly then re-check
                    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
                    continue;
                }
                _ => {
                    let prev_trace_len = {
                        let s = session.lock().await;
                        s.tool_trace.len()
                    };

                    // Drive one step
                    let result = {
                        let mut s = session.lock().await;
                        s.drive_step(&registry, &approval_queue).await
                    };

                    if let Err(e) = result {
                        warn!("Agent step error: {e}");
                        let mut s = session.lock().await;
                        s.status = AgentStatus::Failed;
                        let _ = update_tx.send(AgentUpdate::Failed(e.to_string()));
                        break;
                    }

                    // Report new tool calls
                    {
                        let s = session.lock().await;
                        for call in s.tool_trace.iter().skip(prev_trace_len) {
                            let _ = update_tx.send(AgentUpdate::ToolCalled(call.clone()));
                        }
                        let new_status = s.status.clone();
                        let step = s.current_step;
                        drop(s);
                        let _ = update_tx.send(AgentUpdate::StatusChanged(new_status));
                        if step > 0 {
                            let _ = update_tx.send(AgentUpdate::StepComplete(step.saturating_sub(1)));
                        }
                    }

                    // Save session to disk
                    {
                        let s = session.lock().await;
                        let _ = s.save(&workspace_root);
                    }

                    // Small yield to avoid spinning
                    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
                }
            }
        }

        debug!("Agent loop exiting");
    });
}
