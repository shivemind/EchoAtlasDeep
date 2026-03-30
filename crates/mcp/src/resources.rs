#![allow(dead_code, unused_imports, unused_variables)]
use std::sync::Arc;
use parking_lot::RwLock;
use serde_json::json;

use super::protocol::{ResourceInfo, ResourceContents};
use super::bridge::McpBridge;

pub fn list_resources() -> Vec<ResourceInfo> {
    vec![
        ResourceInfo {
            uri: "workspace://files".into(),
            name: "Workspace Files".into(),
            description: Some("Directory listing of workspace root".into()),
            mime_type: Some("text/plain".into()),
        },
        ResourceInfo {
            uri: "workspace://git-status".into(),
            name: "Git Status".into(),
            description: Some("Current git repository status".into()),
            mime_type: Some("text/plain".into()),
        },
        ResourceInfo {
            uri: "workspace://diagnostics".into(),
            name: "LSP Diagnostics".into(),
            description: Some("All current LSP errors and warnings".into()),
            mime_type: Some("text/plain".into()),
        },
        ResourceInfo {
            uri: "workspace://open-buffers".into(),
            name: "Open Buffers".into(),
            description: Some("List of open editor buffers with content".into()),
            mime_type: Some("text/plain".into()),
        },
    ]
}

pub fn read_resource(uri: &str, bridge: &McpBridge) -> Option<ResourceContents> {
    match uri {
        "workspace://files" => {
            let text = bridge.file_tree_text();
            Some(ResourceContents {
                uri: uri.to_string(),
                mime_type: Some("text/plain".into()),
                text: Some(text),
                blob: None,
            })
        }
        "workspace://git-status" => {
            let text = if bridge.git_status_summary.is_empty() {
                format!("Branch: {}\nNo changes", bridge.git_branch)
            } else {
                format!("Branch: {}\n{}", bridge.git_branch, bridge.git_status_summary)
            };
            Some(ResourceContents {
                uri: uri.to_string(),
                mime_type: Some("text/plain".into()),
                text: Some(text),
                blob: None,
            })
        }
        "workspace://diagnostics" => {
            let text = if bridge.diagnostics.is_empty() {
                "No diagnostics".to_string()
            } else {
                bridge.diagnostics.iter().map(|d| {
                    format!("{}:{}:{}: [{}] {}", d.file, d.line, d.col, d.severity, d.message)
                }).collect::<Vec<_>>().join("\n")
            };
            Some(ResourceContents {
                uri: uri.to_string(),
                mime_type: Some("text/plain".into()),
                text: Some(text),
                blob: None,
            })
        }
        "workspace://open-buffers" => {
            let text = if bridge.open_buffers.is_empty() {
                "No open buffers".to_string()
            } else {
                bridge.open_buffers.iter().map(|b| {
                    let dirty = if b.is_dirty { " [modified]" } else { "" };
                    format!("=== {}{} ===\n{}", b.path, dirty, b.content)
                }).collect::<Vec<_>>().join("\n\n")
            };
            Some(ResourceContents {
                uri: uri.to_string(),
                mime_type: Some("text/plain".into()),
                text: Some(text),
                blob: None,
            })
        }
        _ => None,
    }
}
