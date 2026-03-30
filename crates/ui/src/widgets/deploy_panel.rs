#![allow(dead_code, unused_imports, unused_variables)]
use std::path::Path;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::Widget,
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Provider {
    Fly,
    Vercel,
    Netlify,
    Railway,
    Docker,
}

impl Provider {
    pub fn name(&self) -> &'static str {
        match self {
            Provider::Fly => "Fly.io",
            Provider::Vercel => "Vercel",
            Provider::Netlify => "Netlify",
            Provider::Railway => "Railway",
            Provider::Docker => "Docker",
        }
    }

    pub fn detect_config_file(&self) -> &'static str {
        match self {
            Provider::Fly => "fly.toml",
            Provider::Vercel => "vercel.json",
            Provider::Netlify => "netlify.toml",
            Provider::Railway => "railway.json",
            Provider::Docker => "Dockerfile",
        }
    }

    pub fn deploy_command(&self) -> &'static str {
        match self {
            Provider::Fly => "flyctl deploy",
            Provider::Vercel => "vercel --prod",
            Provider::Netlify => "netlify deploy --prod",
            Provider::Railway => "railway up",
            Provider::Docker => "docker build -t app . && docker push app",
        }
    }

    pub fn color(&self) -> Color {
        match self {
            Provider::Fly => Color::Cyan,
            Provider::Vercel => Color::White,
            Provider::Netlify => Color::Green,
            Provider::Railway => Color::Magenta,
            Provider::Docker => Color::Blue,
        }
    }
}

#[derive(Clone, Debug)]
pub struct DeployTarget {
    pub provider: Provider,
    pub detected: bool,
    pub last_deploy: Option<u64>,
    pub deploy_url: Option<String>,
}

pub struct DeployPanelState {
    pub open: bool,
    pub targets: Vec<DeployTarget>,
    pub selected: usize,
    pub log: Vec<String>,
    pub deploying: bool,
}

impl DeployPanelState {
    pub fn new(workspace_root: &Path) -> Self {
        let mut state = Self {
            open: false,
            targets: Vec::new(),
            selected: 0,
            log: Vec::new(),
            deploying: false,
        };
        state.detect_providers(workspace_root);
        state
    }

    pub fn detect_providers(&mut self, workspace_root: &Path) {
        let all_providers = [
            Provider::Fly,
            Provider::Vercel,
            Provider::Netlify,
            Provider::Railway,
            Provider::Docker,
        ];

        self.targets.clear();
        for provider in &all_providers {
            let config_file = workspace_root.join(provider.detect_config_file());
            let detected = config_file.exists();
            self.targets.push(DeployTarget {
                provider: *provider,
                detected,
                last_deploy: None,
                deploy_url: None,
            });
        }
    }
}

pub struct DeployPanelWidget<'a> {
    pub state: &'a DeployPanelState,
}

impl<'a> Widget for DeployPanelWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Background
        let bg_style = Style::default().bg(Color::Rgb(12, 18, 18));
        for y in area.y..area.y + area.height {
            for x in area.x..area.x + area.width {
                buf.get_mut(x, y).set_char(' ').set_style(bg_style);
            }
        }

        if area.height < 4 {
            return;
        }

        // Border
        let border_style = Style::default().fg(Color::Cyan);
        buf.get_mut(area.x, area.y).set_char('┌').set_style(border_style);
        buf.get_mut(area.x + area.width - 1, area.y).set_char('┐').set_style(border_style);
        buf.get_mut(area.x, area.y + area.height - 1).set_char('└').set_style(border_style);
        buf.get_mut(area.x + area.width - 1, area.y + area.height - 1).set_char('┘').set_style(border_style);
        for x in (area.x + 1)..(area.x + area.width - 1) {
            buf.get_mut(x, area.y).set_char('─').set_style(border_style);
            buf.get_mut(x, area.y + area.height - 1).set_char('─').set_style(border_style);
        }
        for y in (area.y + 1)..(area.y + area.height - 1) {
            buf.get_mut(area.x, y).set_char('│').set_style(border_style);
            buf.get_mut(area.x + area.width - 1, y).set_char('│').set_style(border_style);
        }

        // Title
        let deploy_indicator = if self.state.deploying { " [DEPLOYING...] " } else { "" };
        let title = format!(" Deploy Panel{}", deploy_indicator);
        for (i, ch) in title.chars().enumerate() {
            if area.x + 1 + i as u16 >= area.x + area.width - 1 {
                break;
            }
            let style = if self.state.deploying {
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD | Modifier::SLOW_BLINK)
            } else {
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
            };
            buf.get_mut(area.x + 1 + i as u16, area.y)
                .set_char(ch)
                .set_style(style);
        }

        let inner_x = area.x + 1;
        let inner_w = area.width.saturating_sub(2);

        // Split: top for provider list, bottom for log
        let list_height = (area.height / 2).max(4);
        let log_height = area.height.saturating_sub(list_height + 2);

        // Provider list
        let list_y_start = area.y + 1;
        for (i, target) in self.state.targets.iter().enumerate() {
            let row_y = list_y_start + i as u16;
            if row_y >= area.y + list_height {
                break;
            }

            let is_selected = i == self.state.selected;
            let bg = if is_selected { Color::Rgb(20, 30, 30) } else { Color::Reset };

            let detected_icon = if target.detected { "✓" } else { "○" };
            let last_deploy = target
                .last_deploy
                .map(|t| format!("deployed"))
                .unwrap_or_else(|| "never".to_string());
            let url = target.deploy_url.as_deref().unwrap_or("-");

            let row = format!(
                "{} {:<10} {:<12} {}",
                detected_icon,
                target.provider.name(),
                last_deploy,
                url.chars().take(30).collect::<String>(),
            );

            let provider_color = target.provider.color();
            let style = Style::default()
                .bg(bg)
                .fg(if target.detected { provider_color } else { Color::DarkGray });

            for (j, ch) in row.chars().enumerate() {
                let x = inner_x + j as u16;
                if x >= inner_x + inner_w {
                    break;
                }
                buf.get_mut(x, row_y).set_char(ch).set_style(style);
            }
        }

        // Log separator
        let log_sep_y = area.y + list_height;
        if log_sep_y < area.y + area.height - 1 {
            let sep_title = " Deploy Log ";
            buf.get_mut(inner_x, log_sep_y)
                .set_char('─')
                .set_style(Style::default().fg(Color::DarkGray));
            for (i, ch) in sep_title.chars().enumerate() {
                let x = inner_x + 1 + i as u16;
                if x >= inner_x + inner_w {
                    break;
                }
                buf.get_mut(x, log_sep_y)
                    .set_char(ch)
                    .set_style(Style::default().fg(Color::DarkGray));
            }
        }

        // Log entries
        let log_y_start = area.y + list_height + 1;
        let visible_log: Vec<&String> = self
            .state
            .log
            .iter()
            .rev()
            .take(log_height as usize)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();

        for (i, line) in visible_log.iter().enumerate() {
            let row_y = log_y_start + i as u16;
            if row_y >= area.y + area.height - 1 {
                break;
            }
            let style = Style::default().fg(Color::Gray);
            for (j, ch) in line.chars().enumerate() {
                let x = inner_x + j as u16;
                if x >= inner_x + inner_w {
                    break;
                }
                buf.get_mut(x, row_y).set_char(ch).set_style(style);
            }
        }

        // Key hints
        let hints = " [Enter]=Deploy  [o]=Open URL  [l]=View Logs  [q]=Close ";
        let hints_y = area.y + area.height - 1;
        for (i, ch) in hints.chars().enumerate() {
            let x = inner_x + i as u16;
            if x >= inner_x + inner_w {
                break;
            }
            buf.get_mut(x, hints_y)
                .set_char(ch)
                .set_style(Style::default().fg(Color::DarkGray));
        }
    }
}
