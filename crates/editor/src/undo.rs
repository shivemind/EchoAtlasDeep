#![allow(dead_code)]
//! Branching undo/redo tree for the editor buffer.

#[derive(Debug, Clone)]
pub enum Change {
    Insert { offset: usize, text: Vec<u8> },
    Delete { offset: usize, text: Vec<u8> },
    Group(Vec<Change>),
}

impl Change {
    /// Return the inverse of this change (for undo).
    pub fn inverse(&self) -> Change {
        match self {
            Change::Insert { offset, text } => Change::Delete { offset: *offset, text: text.clone() },
            Change::Delete { offset, text } => Change::Insert { offset: *offset, text: text.clone() },
            Change::Group(changes) => {
                let inverted: Vec<Change> = changes.iter().rev().map(|c| c.inverse()).collect();
                Change::Group(inverted)
            }
        }
    }
}

#[derive(Debug)]
pub struct UndoNode {
    pub id: usize,
    pub change: Change,
    pub parent: Option<usize>,
    pub children: Vec<usize>,
}

pub struct UndoTree {
    /// Index 0 is a sentinel root node.
    nodes: Vec<UndoNode>,
    current: usize,
    /// Accumulating group changes.
    group_stack: Vec<Vec<Change>>,
}

impl UndoTree {
    pub fn new() -> Self {
        let root = UndoNode {
            id: 0,
            change: Change::Group(vec![]),
            parent: None,
            children: vec![],
        };
        Self {
            nodes: vec![root],
            current: 0,
            group_stack: Vec::new(),
        }
    }

    /// Push a new change as a child of `current`, making it the new current.
    pub fn push_change(&mut self, change: Change) {
        if !self.group_stack.is_empty() {
            // Accumulate into the current group
            let top = self.group_stack.last_mut().unwrap();
            top.push(change);
            return;
        }
        let id = self.nodes.len();
        let node = UndoNode {
            id,
            change,
            parent: Some(self.current),
            children: vec![],
        };
        self.nodes[self.current].children.push(id);
        self.nodes.push(node);
        self.current = id;
    }

    /// Start accumulating changes into a group.
    pub fn begin_group(&mut self) {
        self.group_stack.push(Vec::new());
    }

    /// End the current group and push it as a single Change::Group.
    pub fn end_group(&mut self) {
        if let Some(changes) = self.group_stack.pop() {
            if !changes.is_empty() {
                // Push directly, bypassing group_stack check
                let id = self.nodes.len();
                let node = UndoNode {
                    id,
                    change: Change::Group(changes),
                    parent: Some(self.current),
                    children: vec![],
                };
                self.nodes[self.current].children.push(id);
                self.nodes.push(node);
                self.current = id;
            }
        }
    }

    /// Undo: returns the inverse change to apply, moves current to parent.
    pub fn undo(&mut self) -> Option<Change> {
        let parent = self.nodes[self.current].parent?;
        let inv = self.nodes[self.current].change.inverse();
        self.current = parent;
        Some(inv)
    }

    /// Redo: moves to the most recently added child of current.
    pub fn redo(&mut self) -> Option<Change> {
        let children = &self.nodes[self.current].children;
        if children.is_empty() {
            return None;
        }
        let child_id = *children.last().unwrap();
        self.current = child_id;
        Some(self.nodes[self.current].change.clone())
    }

    pub fn current_node(&self) -> &UndoNode {
        &self.nodes[self.current]
    }
}

impl Default for UndoTree {
    fn default() -> Self {
        Self::new()
    }
}
