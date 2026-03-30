#![allow(dead_code, unused_imports, unused_variables)]
use crate::backend::{Message, Role};

/// All the context that can be injected into an AI prompt.
#[derive(Debug, Default)]
pub struct EditorContext {
    pub file_path: Option<String>,
    pub language: Option<String>,
    pub file_content: Option<String>,
    pub selection: Option<String>,
    pub cursor_line: Option<usize>,
    pub diagnostics: Vec<DiagnosticCtx>,
    pub git_diff: Option<String>,
    pub open_files: Vec<String>,
    pub max_tokens: usize,
}

#[derive(Debug, Clone)]
pub struct DiagnosticCtx {
    pub severity: String,
    pub message: String,
    pub line: u32,
}

impl EditorContext {
    pub fn new(max_tokens: usize) -> Self {
        Self {
            max_tokens,
            ..Default::default()
        }
    }

    /// Build a system prompt from available context.
    pub fn system_prompt(&self) -> String {
        let mut parts = vec![
            "You are an expert software engineering assistant embedded in a terminal IDE called rmtide.".to_string(),
            "Be concise, precise, and helpful. Prefer showing code over prose.".to_string(),
        ];

        if let Some(lang) = &self.language {
            parts.push(format!("The current file is written in {lang}."));
        }

        if !self.diagnostics.is_empty() {
            let diag_str: Vec<String> = self
                .diagnostics
                .iter()
                .map(|d| format!("  [{}] line {}: {}", d.severity, d.line + 1, d.message))
                .collect();
            parts.push(format!("Current diagnostics:\n{}", diag_str.join("\n")));
        }

        parts.join("\n")
    }

    /// Build the user message for a given task, injecting context.
    pub fn build_user_message(&self, task: &str) -> String {
        let mut parts = vec![task.to_string()];

        if let Some(sel) = &self.selection {
            parts.push(format!(
                "\n\nSelected code:\n```{}\n{sel}\n```",
                self.language.as_deref().unwrap_or("")
            ));
        } else if let Some(content) = &self.file_content {
            let truncated = truncate_to_tokens(content, self.max_tokens / 2);
            let lang = self.language.as_deref().unwrap_or("");
            if let Some(path) = &self.file_path {
                parts.push(format!("\n\nFile `{path}`:\n```{lang}\n{truncated}\n```"));
            } else {
                parts.push(format!("\n\nCurrent file:\n```{lang}\n{truncated}\n```"));
            }
        }

        if let Some(diff) = &self.git_diff {
            let truncated = truncate_to_tokens(diff, self.max_tokens / 4);
            parts.push(format!("\n\nGit diff:\n```diff\n{truncated}\n```"));
        }

        parts.join("")
    }

    /// Build a complete message list for the AI.
    pub fn to_messages(&self, task: &str) -> Vec<Message> {
        vec![
            Message {
                role: Role::System,
                content: self.system_prompt(),
            },
            Message {
                role: Role::User,
                content: self.build_user_message(task),
            },
        ]
    }

    /// Explain-code prompt.
    pub fn explain_prompt(&self) -> Vec<Message> {
        self.to_messages("Explain what this code does, step by step.")
    }

    /// Fix-diagnostics prompt.
    pub fn fix_prompt(&self) -> Vec<Message> {
        self.to_messages("Fix the diagnostics/errors shown above. Show the corrected code.")
    }

    /// Generate tests prompt.
    pub fn tests_prompt(&self) -> Vec<Message> {
        self.to_messages("Write comprehensive unit tests for this code.")
    }

    /// Generate docstring prompt.
    pub fn docstring_prompt(&self) -> Vec<Message> {
        self.to_messages(
            "Write a clear documentation comment (docstring) for this function or module.",
        )
    }

    /// Refactor prompt with instruction.
    pub fn refactor_prompt(&self, instruction: &str) -> Vec<Message> {
        self.to_messages(&format!("Refactor this code: {instruction}"))
    }
}

/// Rough token estimate: ~4 chars per token.
fn truncate_to_tokens(s: &str, max_tokens: usize) -> &str {
    let max_chars = max_tokens * 4;
    if s.len() <= max_chars {
        s
    } else {
        let mut idx = max_chars;
        while !s.is_char_boundary(idx) {
            idx -= 1;
        }
        &s[..idx]
    }
}
