#![allow(dead_code, unused_imports, unused_variables)]
use std::path::{Path, PathBuf};
use std::sync::Arc;
use parking_lot::RwLock;
use anyhow::Result;

use super::api::{PluginMeta, PluginKind, PluginCommand, PluginEvent, Keymap, Autocmd};
use super::wasm::WasmPlugin;
use super::lua::LuaRuntime;

pub struct PluginRegistry {
    pub wasm_plugins: Vec<WasmPlugin>,
    pub lua: LuaRuntime,
    pub keymaps: Vec<Keymap>,
    pub autocmds: Vec<Autocmd>,
    pub user_commands: Vec<String>,
}

impl PluginRegistry {
    pub fn new() -> Self {
        let lua = LuaRuntime::new().unwrap_or_else(|e| {
            tracing::warn!("Failed to init Lua runtime: {e}");
            LuaRuntime {
                keymaps: Vec::new(),
                user_commands: Vec::new(),
                #[cfg(feature = "lua")]
                lua: mlua::Lua::new(),
            }
        });
        Self {
            wasm_plugins: Vec::new(),
            lua,
            keymaps: Vec::new(),
            autocmds: Vec::new(),
            user_commands: Vec::new(),
        }
    }

    /// Load all plugins from the standard locations.
    pub fn load_all(&mut self) {
        // User config plugins: ~/.config/rmtide/plugins/
        if let Some(config_dir) = dirs::config_dir() {
            let plugin_dir = config_dir.join("rmtide").join("plugins");
            self.load_from_dir(&plugin_dir);

            // User init.lua
            self.lua.load_user_init();
        }

        // Project plugins: .rmtide/plugins/
        let project_plugin_dir = PathBuf::from(".rmtide").join("plugins");
        if project_plugin_dir.exists() {
            self.load_from_dir(&project_plugin_dir);
        }
    }

    fn load_from_dir(&mut self, dir: &Path) {
        if !dir.exists() { return; }

        for entry in walkdir::WalkDir::new(dir).max_depth(1).into_iter().flatten() {
            let path = entry.path();
            match path.extension().and_then(|e| e.to_str()) {
                Some("wasm") => {
                    match WasmPlugin::load(path) {
                        Ok(p) => {
                            tracing::info!("Loaded WASM plugin: {}", p.meta.name);
                            self.wasm_plugins.push(p);
                        }
                        Err(e) => tracing::warn!("Failed to load WASM plugin {}: {e}", path.display()),
                    }
                }
                Some("lua") => {
                    if let Err(e) = self.lua.load_file(path) {
                        tracing::warn!("Failed to load Lua plugin {}: {e}", path.display());
                    }
                }
                _ => {}
            }
        }
    }

    /// Dispatch an event to all plugins, collecting commands.
    pub fn dispatch(&mut self, event: &PluginEvent) -> Vec<PluginCommand> {
        let mut commands = Vec::new();
        for plugin in &mut self.wasm_plugins {
            commands.extend(plugin.on_event(event));
        }
        commands
    }

    /// Number of loaded plugins.
    pub fn count(&self) -> usize {
        self.wasm_plugins.len()
    }
}
