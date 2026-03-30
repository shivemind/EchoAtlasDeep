#![allow(dead_code)]
use serde::{Deserialize, Serialize};

/// LSP Position (0-indexed line + character)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Position {
    pub line: u32,
    pub character: u32,
}

/// LSP Range
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Range {
    pub start: Position,
    pub end: Position,
}

/// LSP Location
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Location {
    pub uri: String,
    pub range: Range,
}

/// LSP DiagnosticSeverity
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum DiagnosticSeverity {
    Error = 1,
    Warning = 2,
    Information = 3,
    Hint = 4,
}

impl DiagnosticSeverity {
    pub fn from_u8(v: u8) -> Self {
        match v {
            1 => Self::Error,
            2 => Self::Warning,
            3 => Self::Information,
            _ => Self::Hint,
        }
    }
}

/// LSP Diagnostic
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diagnostic {
    pub range: Range,
    pub severity: Option<DiagnosticSeverity>,
    pub code: Option<serde_json::Value>,
    pub source: Option<String>,
    pub message: String,
    #[serde(default)]
    pub related_information: Vec<DiagnosticRelatedInformation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticRelatedInformation {
    pub location: Location,
    pub message: String,
}

/// LSP CompletionItemKind
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct CompletionItemKind(pub u8);

/// LSP CompletionItem
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionItem {
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<CompletionItemKind>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub documentation: Option<StringOrMarkup>,
    #[serde(rename = "insertText", skip_serializing_if = "Option::is_none")]
    pub insert_text: Option<String>,
    #[serde(rename = "insertTextFormat", skip_serializing_if = "Option::is_none")]
    pub insert_text_format: Option<u8>, // 1=plain, 2=snippet
    #[serde(rename = "textEdit", skip_serializing_if = "Option::is_none")]
    pub text_edit: Option<TextEdit>,
    #[serde(default)]
    pub additional_text_edits: Vec<TextEdit>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter_text: Option<String>,
    #[serde(rename = "sortText", skip_serializing_if = "Option::is_none")]
    pub sort_text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preselect: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextEdit {
    pub range: Range,
    #[serde(rename = "newText")]
    pub new_text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum StringOrMarkup {
    String(String),
    Markup(MarkupContent),
}

impl StringOrMarkup {
    pub fn as_str(&self) -> &str {
        match self {
            Self::String(s) => s,
            Self::Markup(m) => &m.value,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarkupContent {
    pub kind: String, // "plaintext" or "markdown"
    pub value: String,
}

/// LSP CodeAction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeAction {
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diagnostics: Option<Vec<Diagnostic>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub edit: Option<WorkspaceEdit>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<Command>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Command {
    pub title: String,
    pub command: String,
    #[serde(default)]
    pub arguments: Vec<serde_json::Value>,
}

/// LSP WorkspaceEdit
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceEdit {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub changes: Option<std::collections::HashMap<String, Vec<TextEdit>>>,
}

/// Convert a file path to an LSP URI.
pub fn path_to_uri(path: &std::path::Path) -> String {
    let path_str = path.to_string_lossy();
    if cfg!(windows) {
        // Windows: C:\foo\bar -> file:///C:/foo/bar
        let normalized = path_str.replace('\\', "/");
        format!("file:///{}", normalized)
    } else {
        format!("file://{}", path_str)
    }
}

/// Convert an LSP URI back to a PathBuf.
pub fn uri_to_path(uri: &str) -> std::path::PathBuf {
    let path = uri
        .trim_start_matches("file:///")
        .trim_start_matches("file://");
    std::path::PathBuf::from(if cfg!(windows) {
        path.replace('/', "\\")
    } else {
        path.to_string()
    })
}
