#![allow(dead_code, unused_imports, unused_variables)]
pub mod repo;
pub mod diff;
pub mod blame;
pub mod branch;

use std::path::{Path, PathBuf};
use std::sync::Arc;
use parking_lot::RwLock;

pub use repo::{RepoStatus, FileStatus, FileEntry, RepoDetector};
pub use diff::{GutterLine, GutterMark, DiffHunk, HunkLine, HunkLineKind};
pub use blame::BlameLine;
pub use branch::{BranchInfo, list_branches, checkout_branch, create_branch, delete_branch, stage_file, unstage_file, commit};

/// Central git manager — holds the detected repo root and cached status.
pub struct GitManager {
    pub repo_root: Option<PathBuf>,
    pub status: Arc<RwLock<Option<RepoStatus>>>,
    pub gutter_cache: Arc<RwLock<Vec<GutterLine>>>,
    pub blame_cache: Arc<RwLock<Vec<BlameLine>>>,
    pub branches: Arc<RwLock<Vec<BranchInfo>>>,
}

impl GitManager {
    pub fn new(workspace_root: &Path) -> Self {
        let repo_root = RepoDetector::find_repo(workspace_root);
        let status = if let Some(ref root) = repo_root {
            RepoDetector::status(root).ok()
        } else {
            None
        };
        Self {
            repo_root,
            status: Arc::new(RwLock::new(status)),
            gutter_cache: Arc::new(RwLock::new(Vec::new())),
            blame_cache: Arc::new(RwLock::new(Vec::new())),
            branches: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub fn has_repo(&self) -> bool {
        self.repo_root.is_some()
    }

    pub fn refresh_status(&self) {
        if let Some(ref root) = self.repo_root {
            if let Ok(s) = RepoDetector::status(root) {
                *self.status.write() = Some(s);
            }
        }
    }

    pub fn refresh_branches(&self) {
        if let Some(ref root) = self.repo_root {
            let branches = list_branches(root);
            *self.branches.write() = branches;
        }
    }

    pub fn refresh_gutter(&self, file_path: &Path, content: &str) {
        if let Some(ref root) = self.repo_root {
            let marks = diff::compute_gutter(root, file_path, content);
            *self.gutter_cache.write() = marks;
        }
    }

    pub fn refresh_blame(&self, file_path: &Path) {
        if let Some(ref root) = self.repo_root {
            let lines = blame::blame_file(root, file_path);
            *self.blame_cache.write() = lines;
        }
    }

    pub fn current_branch(&self) -> String {
        self.status.read()
            .as_ref()
            .map(|s| s.head_branch.clone())
            .unwrap_or_default()
    }

    pub fn staged_count(&self) -> usize {
        self.status.read().as_ref().map(|s| s.staged.len()).unwrap_or(0)
    }

    pub fn unstaged_count(&self) -> usize {
        self.status.read().as_ref().map(|s| s.unstaged.len()).unwrap_or(0)
    }

    pub fn stage_file(&self, file_path: &Path) -> anyhow::Result<()> {
        let root = self.repo_root.as_ref()
            .ok_or_else(|| anyhow::anyhow!("No git repo"))?;
        branch::stage_file(root, file_path)?;
        self.refresh_status();
        Ok(())
    }

    pub fn unstage_file(&self, file_path: &Path) -> anyhow::Result<()> {
        let root = self.repo_root.as_ref()
            .ok_or_else(|| anyhow::anyhow!("No git repo"))?;
        branch::unstage_file(root, file_path)?;
        self.refresh_status();
        Ok(())
    }

    pub fn commit(&self, message: &str) -> anyhow::Result<String> {
        let root = self.repo_root.as_ref()
            .ok_or_else(|| anyhow::anyhow!("No git repo"))?;
        let sha = branch::commit(root, message)?;
        self.refresh_status();
        Ok(sha)
    }
}
