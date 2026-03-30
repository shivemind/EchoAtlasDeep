#![allow(dead_code, unused_imports, unused_variables)]
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use git2::{Repository, StatusOptions, StatusShow, Status};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FileStatus {
    Unmodified,
    Modified,
    Added,
    Deleted,
    Renamed,
    Copied,
    Untracked,
    Conflicted,
    Staged,
    StagedModified,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntry {
    pub path: PathBuf,
    pub status: FileStatus,
    pub staged_status: Option<FileStatus>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoStatus {
    pub repo_root: PathBuf,
    pub head_branch: String,
    pub head_sha: String,
    pub staged: Vec<FileEntry>,
    pub unstaged: Vec<FileEntry>,
    pub untracked: Vec<FileEntry>,
    pub conflicted: Vec<FileEntry>,
}

impl RepoStatus {
    pub fn all_files(&self) -> Vec<&FileEntry> {
        let mut all: Vec<&FileEntry> = Vec::new();
        all.extend(&self.staged);
        all.extend(&self.unstaged);
        all.extend(&self.untracked);
        all.extend(&self.conflicted);
        all
    }
}

pub struct RepoDetector;

impl RepoDetector {
    /// Walk up from `start` to find the git repo root.
    pub fn find_repo(start: &Path) -> Option<PathBuf> {
        let mut cur = start.to_path_buf();
        loop {
            if cur.join(".git").exists() {
                return Some(cur);
            }
            if !cur.pop() {
                return None;
            }
        }
    }

    /// Open the repo and compute full status.
    pub fn status(repo_path: &Path) -> anyhow::Result<RepoStatus> {
        let repo = Repository::open(repo_path)?;

        // Head info
        let (head_branch, head_sha) = match repo.head() {
            Ok(head) => {
                let branch = head.shorthand().unwrap_or("HEAD").to_string();
                let sha = head.peel_to_commit()
                    .map(|c| c.id().to_string()[..7].to_string())
                    .unwrap_or_default();
                (branch, sha)
            }
            Err(_) => ("(no branch)".to_string(), String::new()),
        };

        let mut opts = StatusOptions::new();
        opts.show(StatusShow::IndexAndWorkdir);
        opts.include_untracked(true);
        opts.recurse_untracked_dirs(true);
        opts.exclude_submodules(true);

        let statuses = repo.statuses(Some(&mut opts))?;

        let mut staged = Vec::new();
        let mut unstaged = Vec::new();
        let mut untracked = Vec::new();
        let mut conflicted = Vec::new();

        for entry in statuses.iter() {
            let path = PathBuf::from(entry.path().unwrap_or(""));
            let full_path = repo_path.join(&path);
            let s = entry.status();

            if s.contains(Status::CONFLICTED) {
                conflicted.push(FileEntry { path: full_path.clone(), status: FileStatus::Conflicted, staged_status: None });
                continue;
            }

            // Index (staged) changes
            let staged_status = if s.contains(Status::INDEX_NEW) {
                Some(FileStatus::Added)
            } else if s.contains(Status::INDEX_MODIFIED) {
                Some(FileStatus::StagedModified)
            } else if s.contains(Status::INDEX_DELETED) {
                Some(FileStatus::Deleted)
            } else if s.contains(Status::INDEX_RENAMED) {
                Some(FileStatus::Renamed)
            } else {
                None
            };

            // Working tree (unstaged) changes
            let wt_status = if s.contains(Status::WT_NEW) {
                Some(FileStatus::Untracked)
            } else if s.contains(Status::WT_MODIFIED) {
                Some(FileStatus::Modified)
            } else if s.contains(Status::WT_DELETED) {
                Some(FileStatus::Deleted)
            } else if s.contains(Status::WT_RENAMED) {
                Some(FileStatus::Renamed)
            } else {
                None
            };

            if let Some(ss) = &staged_status {
                staged.push(FileEntry {
                    path: full_path.clone(),
                    status: ss.clone(),
                    staged_status: staged_status.clone(),
                });
            }

            if let Some(ws) = wt_status {
                if ws == FileStatus::Untracked {
                    untracked.push(FileEntry { path: full_path, status: ws, staged_status: None });
                } else {
                    unstaged.push(FileEntry { path: full_path, status: ws, staged_status: None });
                }
            }
        }

        Ok(RepoStatus {
            repo_root: repo_path.to_path_buf(),
            head_branch,
            head_sha,
            staged,
            unstaged,
            untracked,
            conflicted,
        })
    }
}
