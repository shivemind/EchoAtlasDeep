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
    /// Optional hint for task-based model routing (e.g. "fast", "powerful").
    pub model_hint: Option<String>,
    /// Import statements extracted from the file for context injection.
    pub imports: Vec<String>,
    /// Nearby function signatures for additional context.
    pub nearby_functions: Vec<String>,
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

    /// Build a language-aware system prompt from available context.
    pub fn system_prompt(&self) -> String {
        let mut parts = vec![
            "You are an expert software engineering assistant embedded in a terminal IDE called rmtide.".to_string(),
            "Be concise, precise, and helpful. Prefer showing code over prose.".to_string(),
        ];

        if let Some(lang) = &self.language {
            parts.push(format!("The current file is written in {lang}."));

            // Language-specific tips
            match lang.as_str() {
                "rust" | "rs" => {
                    parts.push(
                        "Rust tips: prefer idiomatic ownership/borrowing, use Result/Option \
                         over panics, leverage the type system, and follow clippy lints."
                            .to_string(),
                    );
                }
                "python" | "py" => {
                    parts.push(
                        "Python tips: follow PEP 8, use type hints where appropriate, prefer \
                         list/dict comprehensions over loops when readable, and use dataclasses \
                         or Pydantic for structured data."
                            .to_string(),
                    );
                }
                "javascript" | "js" | "typescript" | "ts" | "tsx" | "jsx" => {
                    parts.push(
                        "JS/TS tips: prefer const over let, use async/await over raw promises, \
                         leverage TypeScript types for safety, and avoid implicit any."
                            .to_string(),
                    );
                }
                "go" => {
                    parts.push(
                        "Go tips: handle errors explicitly, keep interfaces small, prefer \
                         composition over inheritance, and use goroutines/channels idiomatically."
                            .to_string(),
                    );
                }
                _ => {}
            }
        }

        if !self.imports.is_empty() {
            let import_str = self.imports.join("\n");
            parts.push(format!("File imports:\n{import_str}"));
        }

        if !self.nearby_functions.is_empty() {
            let fn_str = self.nearby_functions.join("\n");
            parts.push(format!("Nearby function signatures:\n{fn_str}"));
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

    /// Code review prompt — asks the AI to review the current file or selection.
    pub fn review_prompt(&self) -> Vec<Message> {
        self.to_messages(
            "Review this code for correctness, performance, readability, and potential bugs. \
             Provide specific, actionable feedback.",
        )
    }

    /// Generate a commit message from a git diff.
    pub fn commit_prompt(diff: &str) -> Vec<Message> {
        vec![
            Message {
                role: Role::System,
                content: "You are an expert at writing clear, concise git commit messages. \
                           Follow the Conventional Commits specification when possible. \
                           Be specific about what changed and why."
                    .to_string(),
            },
            Message {
                role: Role::User,
                content: format!(
                    "Write a git commit message for the following diff:\n\n```diff\n{diff}\n```\n\n\
                     Respond with only the commit message (subject line, blank line if needed, \
                     optional body). No extra commentary."
                ),
            },
        ]
    }

    /// Semantic search prompt — asks the AI to find relevant code for a natural-language query.
    pub fn semantic_search_prompt(query: &str, code: &str) -> Vec<Message> {
        vec![
            Message {
                role: Role::System,
                content: "You are a code search assistant. Given a natural-language query and \
                           a code snippet, identify and extract the most relevant sections. \
                           Return only the relevant code with brief explanations."
                    .to_string(),
            },
            Message {
                role: Role::User,
                content: format!(
                    "Query: {query}\n\nCode:\n```\n{code}\n```\n\n\
                     Which parts of this code are most relevant to the query? \
                     Quote the relevant sections and explain why each is relevant."
                ),
            },
        ]
    }

    /// Returns a model routing hint based on the task type.
    /// "fast" for lightweight tasks; "powerful" for complex reasoning tasks.
    pub fn model_hint_for_task(task: &str) -> &'static str {
        let lower = task.to_ascii_lowercase();
        if lower.contains("refactor")
            || lower.contains("test")
            || lower.contains("review")
            || lower.contains("complex")
            || lower.contains("architect")
        {
            "powerful"
        } else {
            // completion, explain, docstring, search, etc.
            "fast"
        }
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
