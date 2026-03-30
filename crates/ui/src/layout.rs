/// Binary-tree layout engine.
/// Each node is either a Split (two children) or a Leaf (one pane).
/// Rects are computed fresh every frame from the tree — no cached state.
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use serde::{Deserialize, Serialize};

use core::ids::PaneId;
use crate::pane::{Pane, PaneKind};

// ─── Tree ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LayoutNode {
    Leaf(Pane),
    Split {
        direction: SplitDir,
        /// 0–100 — percentage of space given to `left/top` child.
        ratio: u8,
        left: Box<LayoutNode>,
        right: Box<LayoutNode>,
    },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum SplitDir {
    Horizontal, // left | right
    Vertical,   // top / bottom
}

// ─── LayoutTree ──────────────────────────────────────────────────────────────

pub struct LayoutTree {
    pub root: LayoutNode,
    pub focused: PaneId,
}

impl LayoutTree {
    pub fn new(pane: Pane) -> Self {
        let focused = pane.id;
        Self { root: LayoutNode::Leaf(pane), focused }
    }

    /// Compute all (PaneId, Rect) pairs for the given terminal area.
    /// Called once per render frame.
    pub fn rects(&self, area: Rect) -> Vec<(PaneId, Rect)> {
        let mut out = Vec::new();
        collect_rects(&self.root, area, &mut out);
        out
    }

    /// Split the pane with `target_id` in the given direction, inserting `new_pane`.
    pub fn split(&mut self, target_id: PaneId, dir: SplitDir, new_pane: Pane) {
        let new_id = new_pane.id;
        split_node(&mut self.root, target_id, dir, new_pane);
        self.focused = new_id;
    }

    /// Close the pane with `target_id`. If it has no sibling, nothing happens.
    pub fn close(&mut self, target_id: PaneId) {
        if let Some(new_focus) = close_node(&mut self.root, target_id) {
            self.focused = new_focus;
        }
    }

    /// Collect all pane IDs in order.
    pub fn pane_ids(&self) -> Vec<PaneId> {
        let mut ids = Vec::new();
        collect_ids(&self.root, &mut ids);
        ids
    }

    /// Focus the next pane in traversal order.
    pub fn focus_next(&mut self) {
        let ids = self.pane_ids();
        if ids.is_empty() { return; }
        let pos = ids.iter().position(|&id| id == self.focused).unwrap_or(0);
        self.focused = ids[(pos + 1) % ids.len()];
    }

    /// Focus the previous pane.
    pub fn focus_prev(&mut self) {
        let ids = self.pane_ids();
        if ids.is_empty() { return; }
        let pos = ids.iter().position(|&id| id == self.focused).unwrap_or(0);
        self.focused = ids[(pos + ids.len() - 1) % ids.len()];
    }
}

// ─── Recursive helpers ───────────────────────────────────────────────────────

fn collect_rects(node: &LayoutNode, area: Rect, out: &mut Vec<(PaneId, Rect)>) {
    match node {
        LayoutNode::Leaf(pane) => {
            out.push((pane.id, area));
        }
        LayoutNode::Split { direction, ratio, left, right } => {
            let pct = (*ratio).clamp(5, 95) as u16;
            let rest = 100 - pct;
            let chunks = Layout::default()
                .direction(match direction {
                    SplitDir::Horizontal => Direction::Horizontal,
                    SplitDir::Vertical   => Direction::Vertical,
                })
                .constraints([
                    Constraint::Percentage(pct),
                    Constraint::Percentage(rest),
                ])
                .split(area);
            collect_rects(left,  chunks[0], out);
            collect_rects(right, chunks[1], out);
        }
    }
}

fn collect_ids(node: &LayoutNode, out: &mut Vec<PaneId>) {
    match node {
        LayoutNode::Leaf(pane) => out.push(pane.id),
        LayoutNode::Split { left, right, .. } => {
            collect_ids(left,  out);
            collect_ids(right, out);
        }
    }
}

fn split_node(node: &mut LayoutNode, target: PaneId, dir: SplitDir, new_pane: Pane) {
    match node {
        LayoutNode::Leaf(pane) if pane.id == target => {
            let old = std::mem::replace(node, LayoutNode::Leaf(Pane::new(
                pane.id, PaneKind::Empty // temporary
            )));
            if let LayoutNode::Leaf(original_pane) = old {
                *node = LayoutNode::Split {
                    direction: dir,
                    ratio: 50,
                    left:  Box::new(LayoutNode::Leaf(original_pane)),
                    right: Box::new(LayoutNode::Leaf(new_pane)),
                };
            }
        }
        LayoutNode::Split { left, right, .. } => {
            split_node(left,  target, dir, new_pane.clone());
            split_node(right, target, dir, new_pane);
        }
        _ => {}
    }
}

fn close_node(node: &mut LayoutNode, target: PaneId) -> Option<PaneId> {
    match node {
        LayoutNode::Leaf(_) => None,
        LayoutNode::Split { left, right, .. } => {
            // Check if a direct child is the target.
            let left_is_target  = matches!(&**left,  LayoutNode::Leaf(p) if p.id == target);
            let right_is_target = matches!(&**right, LayoutNode::Leaf(p) if p.id == target);

            if left_is_target {
                let survivor = std::mem::replace(right.as_mut(), LayoutNode::Leaf(Pane::new(
                    PaneId::new(0), PaneKind::Empty
                )));
                let new_focus = first_pane_id(&survivor);
                *node = survivor;
                return Some(new_focus);
            }
            if right_is_target {
                let survivor = std::mem::replace(left.as_mut(), LayoutNode::Leaf(Pane::new(
                    PaneId::new(0), PaneKind::Empty
                )));
                let new_focus = first_pane_id(&survivor);
                *node = survivor;
                return Some(new_focus);
            }
            // Recurse.
            close_node(left, target).or_else(|| close_node(right, target))
        }
    }
}

fn first_pane_id(node: &LayoutNode) -> PaneId {
    match node {
        LayoutNode::Leaf(p) => p.id,
        LayoutNode::Split { left, .. } => first_pane_id(left),
    }
}
