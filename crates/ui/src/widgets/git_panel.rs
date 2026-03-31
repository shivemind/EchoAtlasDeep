#![allow(dead_code, unused_imports, unused_variables)]
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, StatefulWidget, Widget},
};
use git::{FileEntry, FileStatus, RepoStatus};

pub struct GitPanelState {
    pub status: Option<RepoStatus>,
    pub selected: usize,
    pub section: GitSection,    // which section is selected
    pub commit_mode: bool,
    pub commit_msg: String,
    pub show_diff: bool,
}

#[derive(Clone, PartialEq, Eq)]
pub enum GitSection {
    Staged,
    Unstaged,
    Untracked,
}

impl GitPanelState {
    pub fn new() -> Self {
        Self {
            status: None,
            selected: 0,
            section: GitSection::Staged,
            commit_mode: false,
            commit_msg: String::new(),
            show_diff: false,
        }
    }

    pub fn update_status(&mut self, status: RepoStatus) {
        self.status = Some(status);
        self.clamp_selection();
    }

    fn section_len(&self) -> usize {
        let s = match &self.status { Some(s) => s, None => return 0 };
        match self.section {
            GitSection::Staged    => s.staged.len(),
            GitSection::Unstaged  => s.unstaged.len(),
            GitSection::Untracked => s.untracked.len(),
        }
    }

    pub fn move_up(&mut self) {
        if self.selected > 0 { self.selected -= 1; }
    }

    pub fn move_down(&mut self) {
        let max = self.section_len().saturating_sub(1);
        if self.selected < max { self.selected += 1; }
    }

    pub fn next_section(&mut self) {
        self.section = match self.section {
            GitSection::Staged    => GitSection::Unstaged,
            GitSection::Unstaged  => GitSection::Untracked,
            GitSection::Untracked => GitSection::Staged,
        };
        self.selected = 0;
    }

    pub fn selected_file(&self) -> Option<&FileEntry> {
        let s = self.status.as_ref()?;
        let files = match self.section {
            GitSection::Staged    => &s.staged,
            GitSection::Unstaged  => &s.unstaged,
            GitSection::Untracked => &s.untracked,
        };
        files.get(self.selected)
    }

    fn clamp_selection(&mut self) {
        let max = self.section_len().saturating_sub(1);
        if self.selected > max { self.selected = max; }
    }
}

impl Default for GitPanelState {
    fn default() -> Self {
        Self::new()
    }
}

pub struct GitPanelWidget<'a> {
    pub state: &'a GitPanelState,
}

impl<'a> Widget for GitPanelWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let branch = self.state.status.as_ref()
            .map(|s| format!(" Git: {} ", s.head_branch))
            .unwrap_or_else(|| " Git ".to_string());

        let block = Block::default()
            .title(branch)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray));

        let inner = block.inner(area);
        block.render(area, buf);

        if inner.height == 0 { return; }

        let s = match &self.state.status {
            Some(s) => s,
            None => {
                let no_repo = Line::from(Span::styled("  No git repository", Style::default().fg(Color::DarkGray)));
                no_repo.render(inner, buf);
                return;
            }
        };

        let mut items: Vec<ListItem> = Vec::new();

        // Staged section
        let staged_header = format!("Staged ({}):", s.staged.len());
        items.push(ListItem::new(Line::from(vec![
            Span::styled(staged_header, Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        ])));
        for (i, entry) in s.staged.iter().enumerate() {
            let name = entry.path.file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            let prefix = status_prefix(&entry.status);
            let selected = self.state.section == GitSection::Staged && self.state.selected == i;
            let style = if selected {
                Style::default().bg(Color::DarkGray).fg(Color::White)
            } else {
                Style::default().fg(Color::Green)
            };
            items.push(ListItem::new(Line::from(vec![
                Span::styled(format!("  {prefix} {name}"), style),
            ])));
        }

        // Unstaged section
        items.push(ListItem::new(Line::from("")));
        let unstaged_header = format!("Unstaged ({}):", s.unstaged.len());
        items.push(ListItem::new(Line::from(vec![
            Span::styled(unstaged_header, Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        ])));
        for (i, entry) in s.unstaged.iter().enumerate() {
            let name = entry.path.file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            let prefix = status_prefix(&entry.status);
            let selected = self.state.section == GitSection::Unstaged && self.state.selected == i;
            let style = if selected {
                Style::default().bg(Color::DarkGray).fg(Color::White)
            } else {
                Style::default().fg(Color::Red)
            };
            items.push(ListItem::new(Line::from(vec![
                Span::styled(format!("  {prefix} {name}"), style),
            ])));
        }

        // Untracked section
        items.push(ListItem::new(Line::from("")));
        let untracked_header = format!("Untracked ({}):", s.untracked.len());
        items.push(ListItem::new(Line::from(vec![
            Span::styled(untracked_header, Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        ])));
        for (i, entry) in s.untracked.iter().enumerate() {
            let name = entry.path.file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            let selected = self.state.section == GitSection::Untracked && self.state.selected == i;
            let style = if selected {
                Style::default().bg(Color::DarkGray).fg(Color::White)
            } else {
                Style::default().fg(Color::DarkGray)
            };
            items.push(ListItem::new(Line::from(vec![
                Span::styled(format!("  ? {name}"), style),
            ])));
        }

        // Commit mode prompt
        if self.state.commit_mode {
            items.push(ListItem::new(Line::from("")));
            items.push(ListItem::new(Line::from(vec![
                Span::styled("Commit: ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                Span::raw(self.state.commit_msg.clone()),
                Span::styled("█", Style::default().fg(Color::White)),
            ])));
        } else {
            // Hint bar
            items.push(ListItem::new(Line::from("")));
            items.push(ListItem::new(Line::from(vec![
                Span::styled(" s", Style::default().fg(Color::Cyan)),
                Span::styled(" stage  ", Style::default().fg(Color::DarkGray)),
                Span::styled("u", Style::default().fg(Color::Cyan)),
                Span::styled(" unstage  ", Style::default().fg(Color::DarkGray)),
                Span::styled("cc", Style::default().fg(Color::Cyan)),
                Span::styled(" commit  ", Style::default().fg(Color::DarkGray)),
                Span::styled("Tab", Style::default().fg(Color::Cyan)),
                Span::styled(" section", Style::default().fg(Color::DarkGray)),
            ])));
        }

        let list = List::new(items);
        Widget::render(list, inner, buf);
    }
}

fn status_prefix(status: &FileStatus) -> &'static str {
    match status {
        FileStatus::Added         => "A",
        FileStatus::Modified      => "M",
        FileStatus::Deleted       => "D",
        FileStatus::Renamed       => "R",
        FileStatus::Copied        => "C",
        FileStatus::Untracked     => "?",
        FileStatus::Conflicted    => "!",
        FileStatus::Staged        => "S",
        FileStatus::StagedModified => "M",
        FileStatus::Unmodified    => " ",
    }
}
