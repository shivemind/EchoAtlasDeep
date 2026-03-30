#![allow(dead_code, unused_imports, unused_variables)]
//! Prompt library with CRUD and variable interpolation.
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tracing::warn;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptTemplate {
    pub name: String,
    pub description: String,
    pub template: String,
    pub variables: Vec<String>,
    pub tags: Vec<String>,
}

impl PromptTemplate {
    /// Find all `{{var_name}}` patterns in a template string.
    pub fn extract_variables(template: &str) -> Vec<String> {
        let mut vars = Vec::new();
        let mut remaining = template;
        while let Some(start) = remaining.find("{{") {
            let after_open = &remaining[start + 2..];
            if let Some(end) = after_open.find("}}") {
                let var_name = after_open[..end].trim().to_string();
                if !var_name.is_empty() && !vars.contains(&var_name) {
                    vars.push(var_name);
                }
                remaining = &after_open[end + 2..];
            } else {
                break;
            }
        }
        vars
    }
}

pub struct PromptLibrary {
    pub templates: Vec<PromptTemplate>,
    user_dir: PathBuf,
    workspace_dir: Option<PathBuf>,
}

impl PromptLibrary {
    /// Load templates from `~/.config/rmtide/prompts/` and `.rmtide/prompts/`.
    pub fn load(workspace_root: Option<&Path>) -> Self {
        let user_dir = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("rmtide")
            .join("prompts");
        let workspace_dir = workspace_root.map(|r| r.join(".rmtide").join("prompts"));

        let mut templates = Self::built_in_templates();
        // Load user templates
        templates.extend(Self::load_dir(&user_dir));
        // Load workspace templates (may override user templates by name)
        if let Some(ref wd) = workspace_dir {
            for t in Self::load_dir(wd) {
                // Replace existing same-named template
                if let Some(pos) = templates.iter().position(|x| x.name == t.name) {
                    templates[pos] = t;
                } else {
                    templates.push(t);
                }
            }
        }

        Self { templates, user_dir, workspace_dir }
    }

    fn load_dir(dir: &Path) -> Vec<PromptTemplate> {
        let mut out = Vec::new();
        let rd = match std::fs::read_dir(dir) {
            Ok(r) => r,
            Err(_) => return out,
        };
        for entry in rd.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("toml") {
                continue;
            }
            match std::fs::read_to_string(&path) {
                Ok(s) => match toml_parse(&s) {
                    Ok(t) => out.push(t),
                    Err(e) => warn!("Failed to parse prompt {:?}: {e}", path),
                },
                Err(e) => warn!("Failed to read prompt {:?}: {e}", path),
            }
        }
        out
    }

    fn built_in_templates() -> Vec<PromptTemplate> {
        vec![
            PromptTemplate {
                name: "explain".to_string(),
                description: "Explain the selected code".to_string(),
                template: "Explain the following {{language}} code:\n\n```{{language}}\n{{selection}}\n```".to_string(),
                variables: vec!["language".to_string(), "selection".to_string()],
                tags: vec!["builtin".to_string(), "explain".to_string()],
            },
            PromptTemplate {
                name: "fix".to_string(),
                description: "Fix issues in the selection".to_string(),
                template: "Fix any bugs or issues in this {{language}} code:\n\n```{{language}}\n{{selection}}\n```\n\nProvide the corrected code and explain what was wrong.".to_string(),
                variables: vec!["language".to_string(), "selection".to_string()],
                tags: vec!["builtin".to_string(), "fix".to_string()],
            },
            PromptTemplate {
                name: "tests".to_string(),
                description: "Generate unit tests".to_string(),
                template: "Generate comprehensive unit tests for the following {{language}} code from {{file}}:\n\n```{{language}}\n{{selection}}\n```".to_string(),
                variables: vec!["language".to_string(), "file".to_string(), "selection".to_string()],
                tags: vec!["builtin".to_string(), "tests".to_string()],
            },
            PromptTemplate {
                name: "docstring".to_string(),
                description: "Generate documentation".to_string(),
                template: "Write thorough documentation/docstrings for this {{language}} code:\n\n```{{language}}\n{{selection}}\n```".to_string(),
                variables: vec!["language".to_string(), "selection".to_string()],
                tags: vec!["builtin".to_string(), "docs".to_string()],
            },
            PromptTemplate {
                name: "review".to_string(),
                description: "Code review the selection".to_string(),
                template: "Perform a thorough code review of the following {{language}} code in {{file}}.\nFocus on correctness, performance, security, and maintainability.\n\n```{{language}}\n{{selection}}\n```".to_string(),
                variables: vec!["language".to_string(), "file".to_string(), "selection".to_string()],
                tags: vec!["builtin".to_string(), "review".to_string()],
            },
        ]
    }

    /// Add a new template (does not auto-save; call `save` explicitly).
    pub fn add(&mut self, template: PromptTemplate) -> anyhow::Result<()> {
        if let Some(pos) = self.templates.iter().position(|t| t.name == template.name) {
            self.templates[pos] = template;
        } else {
            self.templates.push(template);
        }
        Ok(())
    }

    /// Remove a template by name (only removes in-memory; persisted file unchanged).
    pub fn remove(&mut self, name: &str) -> anyhow::Result<()> {
        self.templates.retain(|t| t.name != name);
        Ok(())
    }

    /// Get a template by name.
    pub fn get(&self, name: &str) -> Option<&PromptTemplate> {
        self.templates.iter().find(|t| t.name == name)
    }

    /// Save a template to the user directory as TOML.
    pub fn save(&self, template: &PromptTemplate) -> anyhow::Result<()> {
        std::fs::create_dir_all(&self.user_dir)?;
        let safe_name: String = template.name.chars()
            .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
            .collect();
        let path = self.user_dir.join(format!("{safe_name}.toml"));
        let content = toml_serialize(template)?;
        std::fs::write(&path, content)?;
        Ok(())
    }

    /// Interpolate variables in a named template.
    /// Returns `None` if the template is not found.
    pub fn render(&self, name: &str, vars: &HashMap<String, String>) -> Option<String> {
        let template = self.get(name)?;
        let mut out = template.template.clone();
        for (k, v) in vars {
            let placeholder = format!("{{{{{k}}}}}");
            out = out.replace(&placeholder, v);
        }
        Some(out)
    }

    /// Search templates by name, description, or tags (case-insensitive substring).
    pub fn search(&self, query: &str) -> Vec<&PromptTemplate> {
        let q = query.to_lowercase();
        self.templates.iter().filter(|t| {
            t.name.to_lowercase().contains(&q)
                || t.description.to_lowercase().contains(&q)
                || t.tags.iter().any(|tag| tag.to_lowercase().contains(&q))
        }).collect()
    }
}

// ── Minimal TOML helpers (avoid new deps) ────────────────────────────────────

fn toml_parse(s: &str) -> anyhow::Result<PromptTemplate> {
    // Very minimal TOML parser for our known structure.
    // Falls back gracefully.
    let mut name = String::new();
    let mut description = String::new();
    let mut template = String::new();
    let mut variables: Vec<String> = Vec::new();
    let mut tags: Vec<String> = Vec::new();

    let mut in_template = false;
    let mut template_lines: Vec<String> = Vec::new();

    for line in s.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("name = ") {
            name = trim_toml_string(&trimmed["name = ".len()..]);
        } else if trimmed.starts_with("description = ") {
            description = trim_toml_string(&trimmed["description = ".len()..]);
        } else if trimmed.starts_with("template = \"\"\"") {
            in_template = true;
        } else if in_template {
            if trimmed == "\"\"\"" {
                in_template = false;
                template = template_lines.join("\n");
            } else {
                template_lines.push(line.to_string());
            }
        } else if trimmed.starts_with("template = ") {
            template = trim_toml_string(&trimmed["template = ".len()..]);
        } else if trimmed.starts_with("variables = ") {
            variables = parse_toml_string_array(&trimmed["variables = ".len()..]);
        } else if trimmed.starts_with("tags = ") {
            tags = parse_toml_string_array(&trimmed["tags = ".len()..]);
        }
    }

    if name.is_empty() {
        anyhow::bail!("Missing 'name' field in prompt template");
    }
    if variables.is_empty() {
        variables = PromptTemplate::extract_variables(&template);
    }

    Ok(PromptTemplate { name, description, template, variables, tags })
}

fn toml_serialize(t: &PromptTemplate) -> anyhow::Result<String> {
    let vars_str = t.variables.iter()
        .map(|v| format!("\"{}\"", v))
        .collect::<Vec<_>>()
        .join(", ");
    let tags_str = t.tags.iter()
        .map(|v| format!("\"{}\"", v))
        .collect::<Vec<_>>()
        .join(", ");
    // Escape the template for TOML using multiline string
    let content = format!(
        "name = \"{}\"\ndescription = \"{}\"\nvariables = [{}]\ntags = [{}]\ntemplate = \"\"\"\n{}\n\"\"\"\n",
        t.name.replace('"', "\\\""),
        t.description.replace('"', "\\\""),
        vars_str,
        tags_str,
        t.template,
    );
    Ok(content)
}

fn trim_toml_string(s: &str) -> String {
    let s = s.trim();
    if (s.starts_with('"') && s.ends_with('"')) || (s.starts_with('\'') && s.ends_with('\'')) {
        s[1..s.len() - 1].to_string()
    } else {
        s.to_string()
    }
}

fn parse_toml_string_array(s: &str) -> Vec<String> {
    let s = s.trim().trim_start_matches('[').trim_end_matches(']');
    s.split(',')
        .map(|item| trim_toml_string(item.trim()))
        .filter(|s| !s.is_empty())
        .collect()
}
