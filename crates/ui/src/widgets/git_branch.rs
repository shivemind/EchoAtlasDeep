#![allow(dead_code, unused_imports, unused_variables)]
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Widget},
};
use git::BranchInfo;

pub struct GitBranchWidget<'a> {
    pub branches: &'a [BranchInfo],
    pub selected: usize,
}

impl<'a> Widget for GitBranchWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let block = Block::default()
            .title(" Branches ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray));

        let inner = block.inner(area);
        block.render(area, buf);

        let items: Vec<ListItem> = self.branches.iter().enumerate().map(|(i, b)| {
            let is_selected = i == self.selected;
            let is_head = b.is_head;

            let prefix = if is_head { "* " } else if b.is_remote { "  " } else { "  " };
            let ahead_behind = if b.ahead > 0 || b.behind > 0 {
                format!(" \u{2191}{} \u{2193}{}", b.ahead, b.behind)
            } else {
                String::new()
            };
            let remote_prefix = if b.is_remote { "remote/" } else { "" };
            let text = format!("{prefix}{remote_prefix}{}{ahead_behind}", b.name);

            let style = if is_selected {
                Style::default().bg(Color::DarkGray).fg(Color::White)
            } else if is_head {
                Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
            } else if b.is_remote {
                Style::default().fg(Color::DarkGray)
            } else {
                Style::default().fg(Color::White)
            };

            ListItem::new(Line::from(Span::styled(text, style)))
        }).collect();

        let hint = ListItem::new(Line::from(vec![
            Span::styled(" Enter", Style::default().fg(Color::Cyan)),
            Span::styled(" checkout  ", Style::default().fg(Color::DarkGray)),
            Span::styled("n", Style::default().fg(Color::Cyan)),
            Span::styled(" new  ", Style::default().fg(Color::DarkGray)),
            Span::styled("D", Style::default().fg(Color::Cyan)),
            Span::styled(" delete", Style::default().fg(Color::DarkGray)),
        ]));

        let mut all_items = items;
        all_items.push(ListItem::new(Line::from("")));
        all_items.push(hint);

        let list = List::new(all_items);
        list.render(inner, buf);
    }
}
