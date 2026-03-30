#![allow(dead_code, unused_imports, unused_variables)]
//! Phase 10 — Point 21: Full sidebar file tree widget.

use std::path::{Path, PathBuf};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::Widget,
};

#[derive(Clone, Debug)]
pub struct TreeNode {
    pub path: PathBuf,
    pub name: String,
    pub is_dir: bool,
    pub expanded: bool,
    pub depth: usize,
    pub git_status: Option<char>, // 'M', 'A', '?', '!'
    pub size: Option<u64>,
}

pub struct FileTreeState {
    pub open: bool,
    pub nodes: Vec<TreeNode>,
    pub selected: usize,
    pub root: PathBuf,
    pub filter: String,
    pub show_hidden: bool,
    pub respect_gitignore: bool,
    pub yank_path: Option<PathBuf>,
    pub cut_path: Option<PathBuf>,
}

impl FileTreeState {
    pub fn new(root: &Path) -> Self {
        let mut s = Self {
            open: false,
            nodes: Vec::new(),
            selected: 0,
            root: root.to_path_buf(),
            filter: String::new(),
            show_hidden: false,
            respect_gitignore: true,
            yank_path: None,
            cut_path: None,
        };
        s.load();
        s
    }

    pub fn load(&mut self) {
        self.nodes.clear();
        self.scan_dir(&self.root.clone(), 0);
    }

    fn scan_dir(&mut self, dir: &Path, depth: usize) {
        let Ok(entries) = std::fs::read_dir(dir) else { return };
        let mut paths: Vec<std::fs::DirEntry> = entries.flatten().collect();
        paths.sort_by(|a, b| {
            let a_is_dir = a.path().is_dir();
            let b_is_dir = b.path().is_dir();
            match (a_is_dir, b_is_dir) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.file_name().cmp(&b.file_name()),
            }
        });

        for entry in paths {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();

            // Hide dotfiles unless show_hidden
            if !self.show_hidden && name.starts_with('.') {
                continue;
            }

            let is_dir = path.is_dir();
            let size = if is_dir {
                None
            } else {
                std::fs::metadata(&path).ok().map(|m| m.len())
            };

            let node = TreeNode {
                path: path.clone(),
                name,
                is_dir,
                expanded: false,
                depth,
                git_status: None,
                size,
            };
            self.nodes.push(node);
        }
    }

    /// Expand or collapse the selected directory node.
    pub fn toggle_expand(&mut self) {
        if self.selected >= self.nodes.len() {
            return;
        }
        let node = &self.nodes[self.selected];
        if !node.is_dir {
            return;
        }
        let expanded = node.expanded;
        let depth = node.depth;
        let path = node.path.clone();

        if expanded {
            // Collapse: remove all children (nodes with depth > current depth after this index)
            self.nodes[self.selected].expanded = false;
            let remove_start = self.selected + 1;
            let remove_end = self.nodes[remove_start..]
                .iter()
                .position(|n| n.depth <= depth)
                .map(|p| remove_start + p)
                .unwrap_or(self.nodes.len());
            self.nodes.drain(remove_start..remove_end);
        } else {
            // Expand: insert children after selected
            self.nodes[self.selected].expanded = true;
            let mut children: Vec<TreeNode> = Vec::new();
            if let Ok(entries) = std::fs::read_dir(&path) {
                let mut paths: Vec<std::fs::DirEntry> = entries.flatten().collect();
                paths.sort_by(|a, b| {
                    let a_is_dir = a.path().is_dir();
                    let b_is_dir = b.path().is_dir();
                    match (a_is_dir, b_is_dir) {
                        (true, false) => std::cmp::Ordering::Less,
                        (false, true) => std::cmp::Ordering::Greater,
                        _ => a.file_name().cmp(&b.file_name()),
                    }
                });
                for entry in paths {
                    let child_path = entry.path();
                    let child_name = entry.file_name().to_string_lossy().to_string();
                    if !self.show_hidden && child_name.starts_with('.') {
                        continue;
                    }
                    let child_is_dir = child_path.is_dir();
                    let child_size = if child_is_dir {
                        None
                    } else {
                        std::fs::metadata(&child_path).ok().map(|m| m.len())
                    };
                    children.push(TreeNode {
                        path: child_path,
                        name: child_name,
                        is_dir: child_is_dir,
                        expanded: false,
                        depth: depth + 1,
                        git_status: None,
                        size: child_size,
                    });
                }
            }
            let insert_pos = self.selected + 1;
            for (i, child) in children.into_iter().enumerate() {
                self.nodes.insert(insert_pos + i, child);
            }
        }
    }

    pub fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    pub fn move_down(&mut self) {
        if self.selected + 1 < self.nodes.len() {
            self.selected += 1;
        }
    }

    pub fn selected_path(&self) -> Option<&Path> {
        self.nodes.get(self.selected).map(|n| n.path.as_path())
    }

    pub fn filter(&mut self, q: &str) {
        self.filter = q.to_string();
    }
}

pub struct FileTreeWidget<'a> {
    pub state: &'a FileTreeState,
    pub focused: bool,
}

impl Widget for FileTreeWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 {
            return;
        }

        // Draw border
        let title = " Files ";
        let border_style = if self.focused {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        // Top border
        buf.get_mut(area.x, area.y)
            .set_char('┌')
            .set_style(border_style);
        buf.get_mut(area.x + area.width.saturating_sub(1), area.y)
            .set_char('┐')
            .set_style(border_style);
        for x in (area.x + 1)..(area.x + area.width.saturating_sub(1)) {
            buf.get_mut(x, area.y).set_char('─').set_style(border_style);
        }
        // Title
        let title_x = area.x + 1;
        for (i, ch) in title.chars().enumerate() {
            if title_x + i as u16 < area.x + area.width.saturating_sub(1) {
                buf.get_mut(title_x + i as u16, area.y)
                    .set_char(ch)
                    .set_style(Style::default().fg(Color::White).add_modifier(Modifier::BOLD));
            }
        }
        // Bottom border
        let bottom_y = area.y + area.height.saturating_sub(1);
        buf.get_mut(area.x, bottom_y)
            .set_char('└')
            .set_style(border_style);
        buf.get_mut(area.x + area.width.saturating_sub(1), bottom_y)
            .set_char('┘')
            .set_style(border_style);
        for x in (area.x + 1)..(area.x + area.width.saturating_sub(1)) {
            buf.get_mut(x, bottom_y)
                .set_char('─')
                .set_style(border_style);
        }
        // Side borders
        for y in (area.y + 1)..bottom_y {
            buf.get_mut(area.x, y).set_char('│').set_style(border_style);
            buf.get_mut(area.x + area.width.saturating_sub(1), y)
                .set_char('│')
                .set_style(border_style);
        }

        let inner = Rect {
            x: area.x + 1,
            y: area.y + 1,
            width: area.width.saturating_sub(2),
            height: area.height.saturating_sub(2),
        };

        if inner.width == 0 || inner.height == 0 {
            return;
        }

        let filter_active = !self.state.filter.is_empty();
        let content_height = if filter_active {
            inner.height.saturating_sub(1) as usize
        } else {
            inner.height as usize
        };

        // Determine scroll offset to keep selected visible
        let scroll = if self.state.selected >= content_height {
            self.state.selected + 1 - content_height
        } else {
            0
        };

        let visible_nodes: Vec<&TreeNode> = if filter_active {
            self.state
                .nodes
                .iter()
                .filter(|n| {
                    n.name
                        .to_lowercase()
                        .contains(&self.state.filter.to_lowercase())
                })
                .collect()
        } else {
            self.state.nodes.iter().collect()
        };

        for row in 0..content_height {
            let node_idx = scroll + row;
            let y = inner.y + row as u16;
            if node_idx >= visible_nodes.len() {
                break;
            }
            let node = visible_nodes[node_idx];
            let is_selected = if filter_active {
                // In filtered mode, selected tracks filtered index
                node_idx == self.state.selected
            } else {
                scroll + row == self.state.selected
            };

            let row_style = if is_selected {
                Style::default()
                    .bg(Color::Blue)
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD)
            } else if node.is_dir {
                Style::default().fg(Color::Cyan)
            } else {
                Style::default().fg(Color::White)
            };

            // Clear the row
            for x in 0..inner.width {
                buf.get_mut(inner.x + x, y)
                    .set_char(' ')
                    .set_style(row_style);
            }

            // Build the display string
            let indent = "  ".repeat(node.depth);
            let prefix = if node.is_dir {
                if node.expanded { "▼ " } else { "▶ " }
            } else {
                "  "
            };
            let git_badge = match node.git_status {
                Some('M') => " M",
                Some('A') => " A",
                Some('?') => " ?",
                Some('!') => " !",
                _ => "",
            };

            let full_text = format!("{}{}{}", indent, prefix, node.name);
            let max_name_width = inner.width.saturating_sub(git_badge.len() as u16) as usize;

            // Render name
            for (i, ch) in full_text.chars().enumerate() {
                if i >= max_name_width {
                    break;
                }
                buf.get_mut(inner.x + i as u16, y)
                    .set_char(ch)
                    .set_style(row_style);
            }

            // Render git badge right-aligned
            if !git_badge.is_empty() {
                let badge_x = inner.x + inner.width - git_badge.len() as u16;
                let badge_style = match node.git_status {
                    Some('M') => Style::default().fg(Color::Yellow),
                    Some('A') => Style::default().fg(Color::Green),
                    Some('?') => Style::default().fg(Color::Red),
                    _ => Style::default().fg(Color::DarkGray),
                };
                for (i, ch) in git_badge.chars().enumerate() {
                    buf.get_mut(badge_x + i as u16, y)
                        .set_char(ch)
                        .set_style(badge_style);
                }
            }
        }

        // Render filter bar at bottom
        if filter_active {
            let filter_y = inner.y + inner.height.saturating_sub(1);
            let filter_text = format!("/{}", self.state.filter);
            for x in 0..inner.width {
                buf.get_mut(inner.x + x, filter_y)
                    .set_char(' ')
                    .set_style(Style::default().bg(Color::DarkGray));
            }
            for (i, ch) in filter_text.chars().enumerate() {
                if i as u16 >= inner.width {
                    break;
                }
                buf.get_mut(inner.x + i as u16, filter_y)
                    .set_char(ch)
                    .set_style(Style::default().fg(Color::Yellow).bg(Color::DarkGray));
            }
        }
    }
}
