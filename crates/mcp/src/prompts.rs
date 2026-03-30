#![allow(dead_code, unused_imports, unused_variables)]
use std::collections::HashMap;
use super::protocol::{PromptInfo, PromptArgument, GetPromptParams, GetPromptResult, PromptMessage, ContentBlock};

pub fn list_prompts() -> Vec<PromptInfo> {
    vec![
        PromptInfo {
            name: "explain-code".into(),
            description: "Explain a piece of code in detail".into(),
            arguments: vec![
                PromptArgument { name: "code".into(), description: "The code to explain".into(), required: true },
                PromptArgument { name: "language".into(), description: "Programming language".into(), required: false },
            ],
        },
        PromptInfo {
            name: "write-tests".into(),
            description: "Generate unit tests for a function or module".into(),
            arguments: vec![
                PromptArgument { name: "code".into(), description: "The code to write tests for".into(), required: true },
                PromptArgument { name: "framework".into(), description: "Testing framework (e.g. pytest, jest, cargo test)".into(), required: false },
            ],
        },
        PromptInfo {
            name: "fix-diagnostics".into(),
            description: "Fix LSP errors and warnings in code".into(),
            arguments: vec![
                PromptArgument { name: "code".into(), description: "The code with errors".into(), required: true },
                PromptArgument { name: "diagnostics".into(), description: "The diagnostic messages to fix".into(), required: true },
            ],
        },
        PromptInfo {
            name: "summarize-diff".into(),
            description: "Summarize a git diff in plain English".into(),
            arguments: vec![
                PromptArgument { name: "diff".into(), description: "The git diff to summarize".into(), required: true },
            ],
        },
        PromptInfo {
            name: "refactor-code".into(),
            description: "Refactor code according to an instruction".into(),
            arguments: vec![
                PromptArgument { name: "code".into(), description: "Code to refactor".into(), required: true },
                PromptArgument { name: "instruction".into(), description: "How to refactor it".into(), required: true },
            ],
        },
    ]
}

pub fn get_prompt(params: &GetPromptParams) -> Option<GetPromptResult> {
    let get = |key: &str| params.arguments.get(key).cloned().unwrap_or_default();

    match params.name.as_str() {
        "explain-code" => {
            let code = get("code");
            let lang = get("language");
            let lang_hint = if lang.is_empty() { String::new() } else { format!(" ({lang})") };
            Some(GetPromptResult {
                description: "Explain the provided code".into(),
                messages: vec![
                    PromptMessage {
                        role: "user".into(),
                        content: ContentBlock::text(format!(
                            "Please explain the following code{lang_hint} in detail, including what it does, how it works, and any notable patterns:\n\n```\n{code}\n```"
                        )),
                    }
                ],
            })
        }
        "write-tests" => {
            let code = get("code");
            let fw = get("framework");
            let fw_hint = if fw.is_empty() { String::new() } else { format!(" using {fw}") };
            Some(GetPromptResult {
                description: "Generate unit tests".into(),
                messages: vec![
                    PromptMessage {
                        role: "user".into(),
                        content: ContentBlock::text(format!(
                            "Write comprehensive unit tests{fw_hint} for the following code:\n\n```\n{code}\n```\n\nCover edge cases, happy paths, and error conditions."
                        )),
                    }
                ],
            })
        }
        "fix-diagnostics" => {
            let code = get("code");
            let diags = get("diagnostics");
            Some(GetPromptResult {
                description: "Fix compiler/linter errors".into(),
                messages: vec![
                    PromptMessage {
                        role: "user".into(),
                        content: ContentBlock::text(format!(
                            "Fix the following errors/warnings in this code:\n\nCode:\n```\n{code}\n```\n\nDiagnostics:\n{diags}\n\nProvide the corrected code."
                        )),
                    }
                ],
            })
        }
        "summarize-diff" => {
            let diff = get("diff");
            Some(GetPromptResult {
                description: "Summarize a git diff".into(),
                messages: vec![
                    PromptMessage {
                        role: "user".into(),
                        content: ContentBlock::text(format!(
                            "Please summarize the following git diff in plain English, explaining what changed and why it matters:\n\n```diff\n{diff}\n```"
                        )),
                    }
                ],
            })
        }
        "refactor-code" => {
            let code = get("code");
            let instruction = get("instruction");
            Some(GetPromptResult {
                description: "Refactor code".into(),
                messages: vec![
                    PromptMessage {
                        role: "user".into(),
                        content: ContentBlock::text(format!(
                            "Refactor the following code according to this instruction: {instruction}\n\nCode:\n```\n{code}\n```\n\nProvide the refactored code with a brief explanation of the changes."
                        )),
                    }
                ],
            })
        }
        _ => None,
    }
}
