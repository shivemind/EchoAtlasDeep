#![allow(dead_code, unused_imports, unused_variables)]
use std::path::{Path, PathBuf};
use parking_lot::RwLock;
use tracing::{info, warn};

#[derive(Clone, Debug)]
pub struct EnvEntry {
    pub key: String,
    pub value: String,
    pub masked: bool,
    pub missing: bool,
}

#[derive(Clone, Debug)]
pub struct EnvFile {
    pub path: PathBuf,
    pub name: String,
    pub entries: Vec<EnvEntry>,
}

pub struct EnvManager {
    pub files: RwLock<Vec<EnvFile>>,
    root: PathBuf,
}

impl EnvManager {
    pub fn new(workspace_root: &Path) -> Self {
        Self {
            files: RwLock::new(Vec::new()),
            root: workspace_root.to_path_buf(),
        }
    }

    /// Scan for .env, .env.local, .env.production, .env.example, etc.
    pub fn load(&self) {
        let env_patterns = [
            ".env",
            ".env.local",
            ".env.development",
            ".env.production",
            ".env.test",
            ".env.staging",
            ".env.example",
        ];

        let mut files = self.files.write();
        files.clear();

        for pattern in &env_patterns {
            let path = self.root.join(pattern);
            if path.exists() {
                let entries = parse_env_file(&path);
                let name = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or(pattern)
                    .to_string();
                files.push(EnvFile {
                    path: path.clone(),
                    name,
                    entries,
                });
            }
        }
    }

    pub fn get_all_files(&self) -> Vec<EnvFile> {
        self.files.read().clone()
    }

    pub fn set_entry(&self, file_name: &str, key: &str, value: &str) -> anyhow::Result<()> {
        let mut files = self.files.write();
        let file = files
            .iter_mut()
            .find(|f| f.name == file_name)
            .ok_or_else(|| anyhow::anyhow!("Env file '{}' not found", file_name))?;

        // Update in memory
        if let Some(entry) = file.entries.iter_mut().find(|e| e.key == key) {
            entry.value = value.to_string();
        } else {
            file.entries.push(EnvEntry {
                key: key.to_string(),
                value: value.to_string(),
                masked: is_sensitive_key(key),
                missing: false,
            });
        }

        // Write back to file
        write_env_file(&file.path, &file.entries)?;
        Ok(())
    }

    pub fn delete_entry(&self, file_name: &str, key: &str) -> anyhow::Result<()> {
        let mut files = self.files.write();
        let file = files
            .iter_mut()
            .find(|f| f.name == file_name)
            .ok_or_else(|| anyhow::anyhow!("Env file '{}' not found", file_name))?;

        file.entries.retain(|e| e.key != key);
        write_env_file(&file.path, &file.entries)?;
        Ok(())
    }

    /// Returns missing-key warnings (keys in .env.example not present elsewhere).
    pub fn validate(&self) -> Vec<String> {
        let files = self.files.read();
        let example = files.iter().find(|f| f.name == ".env.example");
        let Some(example_file) = example else {
            return Vec::new();
        };

        let required_keys: Vec<&str> = example_file.entries.iter().map(|e| e.key.as_str()).collect();

        // Collect all keys from non-example env files
        let defined_keys: std::collections::HashSet<&str> = files
            .iter()
            .filter(|f| f.name != ".env.example")
            .flat_map(|f| f.entries.iter().map(|e| e.key.as_str()))
            .collect();

        required_keys
            .iter()
            .filter(|k| !defined_keys.contains(*k))
            .map(|k| format!("Missing required env key: {}", k))
            .collect()
    }

    /// Returns export KEY=VALUE lines for the named file.
    pub fn export_shell(&self, file_name: &str) -> String {
        let files = self.files.read();
        let file = files.iter().find(|f| f.name == file_name);
        let Some(file) = file else {
            return String::new();
        };

        file.entries
            .iter()
            .map(|e| format!("export {}={}", e.key, shell_quote(&e.value)))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

fn shell_quote(value: &str) -> String {
    if value.chars().any(|c| " \t\n\"'$`\\".contains(c)) {
        format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
    } else {
        value.to_string()
    }
}

fn is_sensitive_key(key: &str) -> bool {
    let lower = key.to_lowercase();
    lower.contains("secret")
        || lower.contains("password")
        || lower.contains("passwd")
        || lower.contains("token")
        || lower.contains("api_key")
        || lower.contains("apikey")
        || lower.contains("private_key")
        || lower.contains("auth")
        || lower.contains("credential")
}

fn write_env_file(path: &Path, entries: &[EnvEntry]) -> anyhow::Result<()> {
    let content = entries
        .iter()
        .map(|e| format!("{}={}", e.key, e.value))
        .collect::<Vec<_>>()
        .join("\n");
    std::fs::write(path, content + "\n")?;
    Ok(())
}

/// Parse a .env file into key-value pairs.
pub fn parse_env_file(path: &Path) -> Vec<EnvEntry> {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };

    let mut entries = Vec::new();

    for line in content.lines() {
        let line = line.trim();

        // Skip blank lines and comments
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // Split on first '='
        if let Some(eq_pos) = line.find('=') {
            let key = line[..eq_pos].trim().to_string();
            let mut value = line[eq_pos + 1..].trim().to_string();

            // Strip surrounding quotes
            if (value.starts_with('"') && value.ends_with('"'))
                || (value.starts_with('\'') && value.ends_with('\''))
            {
                value = value[1..value.len() - 1].to_string();
            }

            if !key.is_empty() {
                let masked = is_sensitive_key(&key);
                entries.push(EnvEntry {
                    key,
                    value,
                    masked,
                    missing: false,
                });
            }
        }
    }

    entries
}
