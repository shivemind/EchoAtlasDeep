#![allow(dead_code, unused_imports, unused_variables)]
//! Notebook mode for .ipynb files — Phase 12 Point 45.
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::Widget,
};

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CellKind {
    Code,
    Markdown,
    Raw,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct CellOutput {
    pub output_type: String,
    #[serde(default)]
    pub text: Vec<String>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct NotebookCell {
    #[serde(default = "default_cell_id")]
    pub id: String,
    pub cell_type: CellKind,
    #[serde(default)]
    pub source: Vec<String>,
    #[serde(default)]
    pub outputs: Vec<CellOutput>,
    pub execution_count: Option<u32>,
}

fn default_cell_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct NotebookMetadata {
    #[serde(default)]
    pub kernelspec: KernelSpec,
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct KernelSpec {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub display_name: String,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct NotebookFile {
    #[serde(default)]
    pub cells: Vec<NotebookCell>,
    #[serde(default)]
    pub metadata: NotebookMetadata,
    pub nbformat: Option<u32>,
}

pub struct Notebook {
    pub cells: Vec<NotebookCell>,
    pub kernel_name: String,
}

impl Notebook {
    /// Parse a .ipynb JSON file.
    pub fn from_json(json: &str) -> anyhow::Result<Self> {
        let raw: NotebookFile = serde_json::from_str(json)?;
        Ok(Self {
            cells: raw.cells,
            kernel_name: raw.metadata.kernelspec.name,
        })
    }

    /// Serialize back to .ipynb JSON.
    pub fn to_json(&self) -> anyhow::Result<String> {
        let raw = NotebookFile {
            cells: self.cells.clone(),
            metadata: NotebookMetadata {
                kernelspec: KernelSpec {
                    name: self.kernel_name.clone(),
                    display_name: self.kernel_name.clone(),
                },
            },
            nbformat: Some(4),
        };
        Ok(serde_json::to_string_pretty(&raw)?)
    }
}

pub struct NotebookState {
    pub open: bool,
    pub notebook: Option<Notebook>,
    pub file_path: Option<String>,
    pub selected_cell: usize,
    pub executing: bool,
    pub edit_mode: bool,
}

impl NotebookState {
    pub fn new() -> Self {
        Self {
            open: false,
            notebook: None,
            file_path: None,
            selected_cell: 0,
            executing: false,
            edit_mode: false,
        }
    }

    pub fn load_file(&mut self, path: &str) -> anyhow::Result<()> {
        let content = std::fs::read_to_string(path)?;
        let notebook = Notebook::from_json(&content)?;
        self.notebook = Some(notebook);
        self.file_path = Some(path.to_string());
        self.selected_cell = 0;
        Ok(())
    }

    pub fn next_cell(&mut self) {
        if let Some(nb) = &self.notebook {
            if self.selected_cell + 1 < nb.cells.len() {
                self.selected_cell += 1;
            }
        }
    }

    pub fn prev_cell(&mut self) {
        if self.selected_cell > 0 {
            self.selected_cell -= 1;
        }
    }

    pub fn add_cell_below(&mut self, kind: CellKind) {
        if let Some(nb) = &mut self.notebook {
            let idx = (self.selected_cell + 1).min(nb.cells.len());
            nb.cells.insert(idx, NotebookCell {
                id: uuid::Uuid::new_v4().to_string(),
                cell_type: kind,
                source: Vec::new(),
                outputs: Vec::new(),
                execution_count: None,
            });
            self.selected_cell = idx;
        }
    }

    pub fn delete_cell(&mut self) {
        if let Some(nb) = &mut self.notebook {
            if !nb.cells.is_empty() && self.selected_cell < nb.cells.len() {
                nb.cells.remove(self.selected_cell);
                if self.selected_cell >= nb.cells.len() && !nb.cells.is_empty() {
                    self.selected_cell = nb.cells.len() - 1;
                }
            }
        }
    }
}

impl Default for NotebookState {
    fn default() -> Self {
        Self::new()
    }
}

pub struct NotebookWidget<'a> {
    pub state: &'a NotebookState,
}

impl Widget for NotebookWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 10 || area.height < 6 {
            return;
        }

        let bg = Style::default().bg(Color::Rgb(14, 16, 22));
        let border_style = Style::default().fg(Color::Yellow);
        let title_style = Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD);
        let label_style = Style::default().fg(Color::DarkGray);
        let code_style = Style::default().fg(Color::White).bg(Color::Rgb(20, 22, 32));
        let md_style = Style::default().fg(Color::LightGreen);
        let output_style = Style::default().fg(Color::Gray).bg(Color::Rgb(12, 14, 18));
        let selected_border = Style::default().fg(Color::Blue);
        let prompt_style = Style::default().fg(Color::Magenta);
        let spinner_chars = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];

        // Fill background
        for row in area.y..area.y + area.height {
            for col in area.x..area.x + area.width {
                buf.get_mut(col, row).set_char(' ').set_style(bg);
            }
        }

        // Title bar
        for col in area.x..area.x + area.width {
            buf.get_mut(col, area.y).set_char('─').set_style(border_style);
        }
        let title = " Notebook ";
        let kernel_info = if let Some(nb) = &self.state.notebook {
            format!(" [kernel: {}] ", if nb.kernel_name.is_empty() { "python3" } else { &nb.kernel_name })
        } else {
            String::new()
        };
        let file_info = self.state.file_path.as_deref().unwrap_or("(no file)");
        let header = format!("{}{} — {}", title, kernel_info, file_info);
        for (i, ch) in header.chars().take((area.width as usize).saturating_sub(2)).enumerate() {
            buf.get_mut(area.x + 1 + i as u16, area.y).set_char(ch).set_style(title_style);
        }

        let Some(nb) = &self.state.notebook else {
            let msg = " No notebook loaded. Open a .ipynb file with :Notebook <path>";
            for (i, ch) in msg.chars().enumerate() {
                let col = area.x + 1 + i as u16;
                if col >= area.x + area.width { break; }
                buf.get_mut(col, area.y + 2).set_char(ch).set_style(label_style);
            }
            return;
        };

        let mut row_y = area.y + 1;

        for (cell_idx, cell) in nb.cells.iter().enumerate() {
            if row_y >= area.y + area.height.saturating_sub(1) { break; }

            let is_selected = cell_idx == self.state.selected_cell;
            let cell_border = if is_selected { selected_border } else { border_style };

            // Cell number & type prompt
            let (type_char, type_color) = match cell.cell_type {
                CellKind::Code => ('▶', Color::Cyan),
                CellKind::Markdown => ('M', Color::Green),
                CellKind::Raw => ('R', Color::Gray),
            };
            let exec_str = match (cell.execution_count, &cell.cell_type) {
                (Some(n), CellKind::Code) => format!("[{}]", n),
                (None, CellKind::Code) => "[ ]".to_string(),
                _ => "   ".to_string(),
            };

            // Cell top border
            for col in area.x..area.x + area.width {
                buf.get_mut(col, row_y).set_char('─').set_style(cell_border);
            }
            buf.get_mut(area.x, row_y).set_char('┌').set_style(cell_border);
            buf.get_mut(area.x + area.width - 1, row_y).set_char('┐').set_style(cell_border);

            // Cell header on top border line
            let cell_header = format!("{} {}", exec_str, cell_idx + 1);
            for (i, ch) in cell_header.chars().take(12).enumerate() {
                let col = area.x + 2 + i as u16;
                if col >= area.x + area.width - 1 { break; }
                buf.get_mut(col, row_y).set_char(ch).set_style(
                    Style::default().fg(type_color)
                );
            }

            // Executing spinner
            if is_selected && self.state.executing {
                let spin_ch = spinner_chars[0];
                buf.get_mut(area.x + 15, row_y)
                    .set_char(spin_ch)
                    .set_style(Style::default().fg(Color::Yellow));
            }

            row_y += 1;

            // Source lines
            let max_source_lines = 8usize;
            let source_lines: Vec<&str> = cell.source.iter()
                .flat_map(|s| s.lines().collect::<Vec<_>>())
                .collect();

            for (li, src_line) in source_lines.iter().enumerate().take(max_source_lines) {
                if row_y >= area.y + area.height - 1 { break; }
                buf.get_mut(area.x, row_y).set_char('│').set_style(cell_border);
                buf.get_mut(area.x + area.width - 1, row_y).set_char('│').set_style(cell_border);

                let line_style = match cell.cell_type {
                    CellKind::Code => code_style,
                    CellKind::Markdown => md_style,
                    CellKind::Raw => Style::default().fg(Color::Gray),
                };

                let display: String = src_line.chars().take((area.width as usize).saturating_sub(4)).collect();
                for (i, ch) in display.chars().enumerate() {
                    let col = area.x + 2 + i as u16;
                    if col >= area.x + area.width - 1 { break; }
                    buf.get_mut(col, row_y).set_char(ch).set_style(line_style);
                }
                row_y += 1;
            }

            if source_lines.len() > max_source_lines && row_y < area.y + area.height - 1 {
                let more = format!("  ... {} more lines", source_lines.len() - max_source_lines);
                buf.get_mut(area.x, row_y).set_char('│').set_style(cell_border);
                buf.get_mut(area.x + area.width - 1, row_y).set_char('│').set_style(cell_border);
                for (i, ch) in more.chars().take((area.width as usize).saturating_sub(4)).enumerate() {
                    buf.get_mut(area.x + 2 + i as u16, row_y).set_char(ch).set_style(label_style);
                }
                row_y += 1;
            }

            // Outputs (code cells only)
            if matches!(cell.cell_type, CellKind::Code) && !cell.outputs.is_empty() {
                // Output separator
                if row_y < area.y + area.height - 1 {
                    for col in area.x + 1..area.x + area.width - 1 {
                        buf.get_mut(col, row_y).set_char('·').set_style(label_style);
                    }
                    buf.get_mut(area.x, row_y).set_char('│').set_style(cell_border);
                    buf.get_mut(area.x + area.width - 1, row_y).set_char('│').set_style(cell_border);
                    row_y += 1;
                }

                for output in &cell.outputs {
                    let max_out_lines = 4usize;
                    let out_lines: Vec<&str> = output.text.iter()
                        .flat_map(|s| s.lines().collect::<Vec<_>>())
                        .collect();

                    let out_color = if output.output_type == "error" { Color::Red } else { Color::LightGreen };

                    for line in out_lines.iter().take(max_out_lines) {
                        if row_y >= area.y + area.height - 1 { break; }
                        buf.get_mut(area.x, row_y).set_char('│').set_style(cell_border);
                        buf.get_mut(area.x + area.width - 1, row_y).set_char('│').set_style(cell_border);
                        let display: String = line.chars().take((area.width as usize).saturating_sub(4)).collect();
                        for (i, ch) in display.chars().enumerate() {
                            let col = area.x + 2 + i as u16;
                            if col >= area.x + area.width - 1 { break; }
                            buf.get_mut(col, row_y).set_char(ch).set_style(
                                Style::default().fg(out_color).bg(Color::Rgb(12, 14, 18))
                            );
                        }
                        row_y += 1;
                    }
                }
            }

            // Cell bottom border
            if row_y < area.y + area.height {
                for col in area.x..area.x + area.width {
                    buf.get_mut(col, row_y).set_char('─').set_style(cell_border);
                }
                buf.get_mut(area.x, row_y).set_char('└').set_style(cell_border);
                buf.get_mut(area.x + area.width - 1, row_y).set_char('┘').set_style(cell_border);
                row_y += 1;
            }

            row_y += 1; // spacing between cells
        }

        // Hint bar
        let hint_y = area.y + area.height - 1;
        let hint = " [↑↓] cells  [Enter] exec  [i] edit  [o] add  [dd] delete  [Esc] close";
        for (i, ch) in hint.chars().take((area.width as usize).saturating_sub(2)).enumerate() {
            buf.get_mut(area.x + 1 + i as u16, hint_y).set_char(ch).set_style(label_style);
        }
    }
}
