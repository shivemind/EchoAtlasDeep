#![allow(dead_code, unused_imports, unused_variables)]
//! DAP (Debug Adapter Protocol) JSON-RPC protocol types.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A DAP request message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DapRequest {
    pub seq: u64,
    #[serde(rename = "type")]
    pub msg_type: String, // always "request"
    pub command: String,
    pub arguments: Option<serde_json::Value>,
}

impl DapRequest {
    pub fn new(seq: u64, command: impl Into<String>, arguments: Option<serde_json::Value>) -> Self {
        Self {
            seq,
            msg_type: "request".to_string(),
            command: command.into(),
            arguments,
        }
    }
}

/// A DAP response message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DapResponse {
    pub seq: u64,
    #[serde(rename = "type")]
    pub msg_type: String, // always "response"
    pub request_seq: u64,
    pub success: bool,
    pub command: String,
    pub message: Option<String>,
    pub body: Option<serde_json::Value>,
}

/// A DAP event message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DapEvent {
    pub seq: u64,
    #[serde(rename = "type")]
    pub msg_type: String, // always "event"
    pub event: String,
    pub body: Option<serde_json::Value>,
}

/// Arguments for the "launch" request.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LaunchArgs {
    pub program: String,
    #[serde(default)]
    pub no_debug: bool,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
    pub cwd: String,
    #[serde(default)]
    pub stop_on_entry: bool,
}

/// Arguments for the "initialize" request.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InitializeArgs {
    pub client_id: String,
    pub adapter_id: String,
    #[serde(default = "default_true")]
    pub lines_start_at1: bool,
    #[serde(default = "default_true")]
    pub columns_start_at1: bool,
    pub path_format: String,
}

fn default_true() -> bool {
    true
}

/// A breakpoint location in source.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BreakpointLocation {
    pub source: Source,
    pub line: u64,
    pub column: Option<u64>,
    pub condition: Option<String>,
}

/// A DAP Source object.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Source {
    pub name: Option<String>,
    pub path: Option<String>,
}

/// A stack frame.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StackFrame {
    pub id: u64,
    pub name: String,
    pub source: Option<Source>,
    pub line: u64,
    pub column: u64,
}

/// A debug variable.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Variable {
    pub name: String,
    pub value: String,
    #[serde(rename = "type")]
    pub var_type: Option<String>,
    pub variables_reference: u64,
}

/// DAP stopped event body.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoppedEventBody {
    pub reason: String,
    pub thread_id: Option<u64>,
    pub all_threads_stopped: Option<bool>,
    pub description: Option<String>,
}

/// DAP output event body.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputEventBody {
    pub category: Option<String>,
    pub output: String,
}
