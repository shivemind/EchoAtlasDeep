#![allow(dead_code, unused_imports, unused_variables)]
use std::path::Path;
use git2::{Repository, DiffOptions};
use serde::{Deserialize, Serialize};

/// What happened to a line relative to HEAD.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum GutterMark {
    Added,       // +  green
    Modified,    // ~  yellow
    DeletedBelow, // _  red (shown on the line above the deletion)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GutterLine {
    pub line: usize,   // 0-based buffer line
    pub mark: GutterMark,
}

/// Compute gutter marks by comparing current buffer content to the HEAD blob.
pub fn compute_gutter(repo_path: &Path, file_path: &Path, buffer_content: &str) -> Vec<GutterLine> {
    let result = compute_gutter_inner(repo_path, file_path, buffer_content);
    result.unwrap_or_default()
}

fn compute_gutter_inner(repo_path: &Path, file_path: &Path, buffer_content: &str) -> anyhow::Result<Vec<GutterLine>> {
    let repo = Repository::open(repo_path)?;

    // Get the HEAD blob for this file
    let head = repo.head()?;
    let tree = head.peel_to_tree()?;

    let rel_path = file_path.strip_prefix(repo_path).unwrap_or(file_path);
    let entry = match tree.get_path(rel_path) {
        Ok(e) => e,
        Err(_) => {
            // File not in HEAD — everything is added
            let lines = buffer_content.lines().count();
            return Ok((0..lines).map(|l| GutterLine { line: l, mark: GutterMark::Added }).collect());
        }
    };

    let blob = repo.find_blob(entry.id())?;
    let head_content = std::str::from_utf8(blob.content()).unwrap_or("");

    Ok(myers_diff(head_content, buffer_content))
}

/// Simple Myers-style line diff returning gutter annotations.
fn myers_diff(old: &str, new: &str) -> Vec<GutterLine> {
    let old_lines: Vec<&str> = old.lines().collect();
    let new_lines: Vec<&str> = new.lines().collect();

    let mut marks: Vec<GutterLine> = Vec::new();

    // Build a simple LCS-based diff using the patience algorithm approximation.
    // For each new line, determine if it's added, modified, or unchanged.
    let n = new_lines.len();
    let o = old_lines.len();

    // Build old line set for quick membership (for simplicity, use position matching)
    // This is a simplified diff — not full Myers — sufficient for gutter markers.
    let mut old_idx = 0usize;
    let mut new_idx = 0usize;

    while new_idx < n {
        if old_idx < o {
            if new_lines[new_idx] == old_lines[old_idx] {
                // Unchanged
                old_idx += 1;
                new_idx += 1;
            } else {
                // Check if it's a modification (old line was replaced) or insertion
                // Look ahead a small window to detect insertions vs replacements
                let found_ahead = old_lines[old_idx..o.min(old_idx + 8)]
                    .iter()
                    .position(|&l| l == new_lines[new_idx]);

                if let Some(skip) = found_ahead {
                    if skip == 0 {
                        // Modified line
                        marks.push(GutterLine { line: new_idx, mark: GutterMark::Modified });
                        old_idx += 1;
                        new_idx += 1;
                    } else {
                        // Inserted line(s)
                        marks.push(GutterLine { line: new_idx, mark: GutterMark::Added });
                        new_idx += 1;
                        // Don't advance old_idx
                    }
                } else {
                    // No match in lookahead — treat as modified
                    marks.push(GutterLine { line: new_idx, mark: GutterMark::Modified });
                    old_idx += 1;
                    new_idx += 1;
                }
            }
        } else {
            // New file is longer — rest are additions
            marks.push(GutterLine { line: new_idx, mark: GutterMark::Added });
            new_idx += 1;
        }
    }

    // Deleted lines at the end
    while old_idx < o {
        if new_idx > 0 {
            marks.push(GutterLine { line: new_idx.saturating_sub(1), mark: GutterMark::DeletedBelow });
        }
        old_idx += 1;
    }

    marks
}

/// Hunk: a contiguous range of changed lines in the new file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffHunk {
    pub new_start: usize,
    pub new_count: usize,
    pub old_start: usize,
    pub old_count: usize,
    pub lines: Vec<HunkLine>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HunkLine {
    pub kind: HunkLineKind,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HunkLineKind {
    Context,
    Added,
    Removed,
}

/// Full diff between HEAD and current content, as hunks (for preview pane).
pub fn full_diff(repo_path: &Path, file_path: &Path, buffer_content: &str) -> Vec<DiffHunk> {
    full_diff_inner(repo_path, file_path, buffer_content).unwrap_or_default()
}

fn full_diff_inner(repo_path: &Path, file_path: &Path, buffer_content: &str) -> anyhow::Result<Vec<DiffHunk>> {
    let repo = Repository::open(repo_path)?;
    let head = repo.head()?;
    let tree = head.peel_to_tree()?;

    let rel_path = file_path.strip_prefix(repo_path).unwrap_or(file_path);
    let head_content = match tree.get_path(rel_path) {
        Ok(entry) => {
            let blob = repo.find_blob(entry.id())?;
            std::str::from_utf8(blob.content()).unwrap_or("").to_string()
        }
        Err(_) => String::new(),
    };

    let old_lines: Vec<&str> = head_content.lines().collect();
    let new_lines: Vec<&str> = buffer_content.lines().collect();

    // Build simple unified diff hunks
    let mut hunks: Vec<DiffHunk> = Vec::new();
    let gutter = myers_diff(&head_content, buffer_content);

    if gutter.is_empty() {
        return Ok(hunks);
    }

    // Group changed lines into hunks with 3 lines of context
    let context = 3;
    let mut hunk_lines: Vec<HunkLine> = Vec::new();
    let mut hunk_start = 0usize;

    for gl in &gutter {
        let new_idx = gl.line;

        // Add context before
        let ctx_start = new_idx.saturating_sub(context);
        for ci in ctx_start..new_idx {
            if ci < new_lines.len() {
                hunk_lines.push(HunkLine { kind: HunkLineKind::Context, content: new_lines[ci].to_string() });
            }
        }

        // The changed line
        let content = if new_idx < new_lines.len() { new_lines[new_idx].to_string() } else { String::new() };
        let kind = match gl.mark {
            GutterMark::Added => HunkLineKind::Added,
            GutterMark::Modified => HunkLineKind::Added,
            GutterMark::DeletedBelow => HunkLineKind::Removed,
        };
        hunk_lines.push(HunkLine { kind, content });
    }

    if !hunk_lines.is_empty() {
        hunks.push(DiffHunk {
            new_start: 0,
            new_count: new_lines.len(),
            old_start: 0,
            old_count: old_lines.len(),
            lines: hunk_lines,
        });
    }

    Ok(hunks)
}
