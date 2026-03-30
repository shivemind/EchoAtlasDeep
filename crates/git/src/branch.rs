#![allow(dead_code, unused_imports, unused_variables)]
use std::path::Path;
use git2::{Repository, BranchType, Branch, Oid};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchInfo {
    pub name: String,
    pub is_head: bool,
    pub is_remote: bool,
    pub upstream: Option<String>,
    pub ahead: usize,
    pub behind: usize,
    pub last_sha: String,
    pub last_message: String,
}

pub fn list_branches(repo_path: &Path) -> Vec<BranchInfo> {
    list_branches_inner(repo_path).unwrap_or_default()
}

fn list_branches_inner(repo_path: &Path) -> anyhow::Result<Vec<BranchInfo>> {
    let repo = Repository::open(repo_path)?;
    let mut result = Vec::new();

    // Local branches
    for branch in repo.branches(Some(BranchType::Local))? {
        let (branch, _) = branch?;
        if let Some(info) = branch_info(&repo, &branch, false) {
            result.push(info);
        }
    }

    // Remote tracking branches
    for branch in repo.branches(Some(BranchType::Remote))? {
        let (branch, _) = branch?;
        if let Some(info) = branch_info(&repo, &branch, true) {
            result.push(info);
        }
    }

    Ok(result)
}

fn branch_info(repo: &Repository, branch: &Branch, is_remote: bool) -> Option<BranchInfo> {
    let name = branch.name().ok()??.to_string();
    let is_head = branch.is_head();

    let (last_sha, last_message) = match branch.get().peel_to_commit() {
        Ok(commit) => {
            let sha = commit.id().to_string()[..7].to_string();
            let msg = commit.summary().unwrap_or("").to_string();
            (sha, msg)
        }
        Err(_) => (String::new(), String::new()),
    };

    let (upstream, ahead, behind) = if !is_remote {
        match branch.upstream() {
            Ok(up) => {
                let up_name = up.name().ok().flatten().unwrap_or("").to_string();
                let local_oid = branch.get().peel_to_commit().ok().map(|c| c.id()).unwrap_or(Oid::zero());
                let remote_oid = up.get().peel_to_commit().ok().map(|c| c.id()).unwrap_or(Oid::zero());
                let (ahead, behind) = if local_oid != Oid::zero() && remote_oid != Oid::zero() {
                    repo.graph_ahead_behind(local_oid, remote_oid).unwrap_or((0, 0))
                } else {
                    (0, 0)
                };
                (Some(up_name), ahead, behind)
            }
            Err(_) => (None, 0, 0),
        }
    } else {
        (None, 0, 0)
    };

    Some(BranchInfo {
        name,
        is_head,
        is_remote,
        upstream,
        ahead,
        behind,
        last_sha,
        last_message,
    })
}

/// Checkout a local branch by name.
pub fn checkout_branch(repo_path: &Path, branch_name: &str) -> anyhow::Result<()> {
    let repo = Repository::open(repo_path)?;
    let branch = repo.find_branch(branch_name, BranchType::Local)
        .map_err(|e| anyhow::anyhow!("Branch not found: {}", e))?;
    let obj = branch.get().peel_to_commit()?.into_object();
    repo.checkout_tree(&obj, None)?;
    repo.set_head(&format!("refs/heads/{branch_name}"))?;
    Ok(())
}

/// Create a new branch from HEAD.
pub fn create_branch(repo_path: &Path, branch_name: &str) -> anyhow::Result<()> {
    let repo = Repository::open(repo_path)?;
    let head = repo.head()?.peel_to_commit()?;
    repo.branch(branch_name, &head, false)?;
    Ok(())
}

/// Delete a local branch.
pub fn delete_branch(repo_path: &Path, branch_name: &str) -> anyhow::Result<()> {
    let repo = Repository::open(repo_path)?;
    let mut branch = repo.find_branch(branch_name, BranchType::Local)?;
    branch.delete()?;
    Ok(())
}

/// Stage a file.
pub fn stage_file(repo_path: &Path, file_path: &Path) -> anyhow::Result<()> {
    let repo = Repository::open(repo_path)?;
    let mut index = repo.index()?;
    let rel_path = file_path.strip_prefix(repo_path).unwrap_or(file_path);
    index.add_path(rel_path)?;
    index.write()?;
    Ok(())
}

/// Unstage a file (reset HEAD).
pub fn unstage_file(repo_path: &Path, file_path: &Path) -> anyhow::Result<()> {
    let repo = Repository::open(repo_path)?;
    let head = repo.head()?.peel_to_commit()?;
    let head_tree = head.tree()?;
    let rel_path = file_path.strip_prefix(repo_path).unwrap_or(file_path);
    repo.reset_default(Some(head_tree.as_object()), std::iter::once(rel_path))?;
    Ok(())
}

/// Commit staged changes with a message.
pub fn commit(repo_path: &Path, message: &str) -> anyhow::Result<String> {
    let repo = Repository::open(repo_path)?;
    let sig = repo.signature()?;
    let mut index = repo.index()?;
    let tree_id = index.write_tree()?;
    let tree = repo.find_tree(tree_id)?;

    let parent_commit = match repo.head() {
        Ok(head) => Some(head.peel_to_commit()?),
        Err(_) => None,
    };

    let parents: Vec<&git2::Commit> = parent_commit.as_ref().map(|c| vec![c]).unwrap_or_default();

    let oid = repo.commit(
        Some("HEAD"),
        &sig, &sig,
        message,
        &tree,
        &parents,
    )?;

    Ok(oid.to_string()[..7].to_string())
}
