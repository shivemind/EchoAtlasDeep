#![allow(dead_code, unused_imports, unused_variables)]
use std::path::Path;
use git2::{Repository, BlameOptions};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlameLine {
    pub line: usize,         // 0-based
    pub sha: String,         // 7-char short SHA
    pub author: String,
    pub time_ago: String,    // e.g. "2 days ago"
    pub summary: String,     // commit message first line
}

pub fn blame_file(repo_path: &Path, file_path: &Path) -> Vec<BlameLine> {
    blame_inner(repo_path, file_path).unwrap_or_default()
}

fn blame_inner(repo_path: &Path, file_path: &Path) -> anyhow::Result<Vec<BlameLine>> {
    let repo = Repository::open(repo_path)?;
    let rel_path = file_path.strip_prefix(repo_path).unwrap_or(file_path);
    let mut opts = BlameOptions::new();
    let blame = repo.blame_file(rel_path, Some(&mut opts))?;

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;

    let mut result = Vec::new();
    let mut line_idx = 0usize;

    for hunk in blame.iter() {
        let commit_id = hunk.final_commit_id();
        let lines_in_hunk = hunk.lines_in_hunk();

        let (sha, author, time_ago, summary) = match repo.find_commit(commit_id) {
            Ok(commit) => {
                let sha = commit_id.to_string()[..7].to_string();
                let sig = commit.author();
                let author = sig.name().unwrap_or("?").to_string();
                let commit_time = commit.time().seconds();
                let diff_secs = now - commit_time;
                let time_ago = format_time_ago(diff_secs);
                let summary = commit.summary().unwrap_or("").to_string();
                (sha, author, time_ago, summary)
            }
            Err(_) => {
                let sha = commit_id.to_string()[..7].to_string();
                (sha, "?".to_string(), "?".to_string(), String::new())
            }
        };

        for _ in 0..lines_in_hunk {
            result.push(BlameLine {
                line: line_idx,
                sha: sha.clone(),
                author: author.clone(),
                time_ago: time_ago.clone(),
                summary: summary.clone(),
            });
            line_idx += 1;
        }
    }

    Ok(result)
}

fn format_time_ago(secs: i64) -> String {
    if secs < 0 {
        return "in the future".to_string();
    }
    if secs < 60 {
        return format!("{secs}s ago");
    }
    let mins = secs / 60;
    if mins < 60 {
        return format!("{mins}m ago");
    }
    let hours = mins / 60;
    if hours < 24 {
        return format!("{hours}h ago");
    }
    let days = hours / 24;
    if days < 30 {
        return format!("{days} days ago");
    }
    let months = days / 30;
    if months < 12 {
        return format!("{months} months ago");
    }
    let years = months / 12;
    format!("{years} years ago")
}
