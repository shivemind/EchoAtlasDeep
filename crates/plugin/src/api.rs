#![allow(dead_code, unused_imports, unused_variables)]
use serde::{Deserialize, Serialize};

/// Event that plugins can receive.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PluginEvent {
    BufEnter { path: String },
    BufLeave { path: String },
    BufWrite { path: String },
    CursorMoved { line: usize, col: usize },
    ModeChanged { mode: String },
    InsertChar { ch: char },
    KeyPress { key: String },
    Custom { name: String, data: serde_json::Value },
}

/// Commands a plugin can send back to the host.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PluginCommand {
    Log { level: LogLevel, message: String },
    EmitEvent { name: String, data: serde_json::Value },
    ReadFile { path: String },
    WriteFile { path: String, content: String },
    SetKeymap { mode: String, lhs: String, rhs: String, desc: String },
    RegisterCommand { name: String, desc: String },
    RegisterAutocmd { event: String, pattern: String, callback_id: String },
    SetOption { name: String, value: serde_json::Value },
    Notify { message: String, level: LogLevel },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LogLevel { Debug, Info, Warn, Error }

/// Metadata about a loaded plugin.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMeta {
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub kind: PluginKind,
    pub path: std::path::PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PluginKind { Wasm, Lua, Native }

/// A keymap registered by a plugin.
#[derive(Debug, Clone)]
pub struct Keymap {
    pub mode: String,
    pub lhs: String,
    pub rhs: String,
    pub desc: String,
    pub plugin: String,
}

/// An autocommand registered by a plugin.
#[derive(Debug, Clone)]
pub struct Autocmd {
    pub event: String,
    pub pattern: String,
    pub callback_id: String,
    pub plugin: String,
}
