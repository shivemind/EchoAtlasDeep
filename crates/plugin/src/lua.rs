#![allow(dead_code, unused_imports, unused_variables)]
use std::path::Path;
use anyhow::Result;
use super::api::{PluginCommand, PluginEvent, LogLevel, Keymap};

/// Lua runtime state.
pub struct LuaRuntime {
    /// Registered keymaps from Lua init.lua / plugins.
    pub keymaps: Vec<Keymap>,
    /// User commands registered from Lua.
    pub user_commands: Vec<String>,

    #[cfg(feature = "lua")]
    lua: mlua::Lua,
}

impl LuaRuntime {
    pub fn new() -> Result<Self> {
        #[cfg(feature = "lua")]
        {
            let lua = mlua::Lua::new();
            setup_vim_api(&lua)?;
            Ok(Self { keymaps: Vec::new(), user_commands: Vec::new(), lua })
        }
        #[cfg(not(feature = "lua"))]
        Ok(Self { keymaps: Vec::new(), user_commands: Vec::new() })
    }

    /// Load and execute a Lua file.
    pub fn load_file(&mut self, path: &Path) -> Result<()> {
        #[cfg(feature = "lua")]
        {
            let content = std::fs::read_to_string(path)?;
            self.lua.load(&content).set_name(path.to_string_lossy().as_ref()).exec()?;
        }
        #[cfg(not(feature = "lua"))]
        {
            tracing::info!("Lua support not compiled in — skipping {}", path.display());
        }
        Ok(())
    }

    /// Load the user's init.lua if it exists.
    pub fn load_user_init(&mut self) {
        if let Some(config_dir) = dirs::config_dir() {
            let init_path = config_dir.join("rmtide").join("init.lua");
            if init_path.exists() {
                if let Err(e) = self.load_file(&init_path) {
                    tracing::warn!("Error loading init.lua: {e}");
                }
            }
        }
    }

    /// Execute a Lua expression and return the string result.
    pub fn eval(&self, code: &str) -> Result<String> {
        #[cfg(feature = "lua")]
        {
            let result: mlua::Value = self.lua.load(code).eval()?;
            Ok(format!("{result:?}"))
        }
        #[cfg(not(feature = "lua"))]
        anyhow::bail!("Lua support not compiled in")
    }
}

#[cfg(feature = "lua")]
fn setup_vim_api(lua: &mlua::Lua) -> Result<()> {
    use mlua::prelude::*;

    let vim_table = lua.create_table()?;

    // vim.keymap.set(mode, lhs, rhs, opts)
    let keymap_table = lua.create_table()?;
    keymap_table.set("set", lua.create_function(|_, (mode, lhs, rhs, _opts): (String, String, LuaValue, Option<LuaTable>)| {
        let rhs_str = match &rhs {
            LuaValue::String(s) => s.to_str().unwrap_or("").to_string(),
            _ => format!("{rhs:?}"),
        };
        tracing::debug!("vim.keymap.set({mode}, {lhs}, {rhs_str})");
        Ok(())
    })?)?;
    vim_table.set("keymap", keymap_table)?;

    // vim.cmd
    vim_table.set("cmd", lua.create_function(|_, cmd: String| {
        tracing::debug!("vim.cmd({cmd})");
        Ok(())
    })?)?;

    // vim.opt (options table)
    let opt_table = lua.create_table()?;
    vim_table.set("opt", opt_table)?;

    // vim.api stub table
    let api_table = lua.create_table()?;
    api_table.set("nvim_set_keymap", lua.create_function(|_, (_mode, _lhs, _rhs, _opts): (String, String, String, LuaTable)| Ok(()))?)?;
    api_table.set("nvim_buf_get_lines", lua.create_function(|_, (_buf, _start, _end, _strict): (i32, i32, i32, bool)| -> LuaResult<Vec<String>> { Ok(vec![]) })?)?;
    api_table.set("nvim_command", lua.create_function(|_, cmd: String| { tracing::debug!("nvim_command: {cmd}"); Ok(()) })?)?;
    vim_table.set("api", api_table)?;

    // vim.notify
    vim_table.set("notify", lua.create_function(|_, (msg, _level): (String, Option<i32>)| {
        tracing::info!("[Lua] {msg}");
        Ok(())
    })?)?;

    lua.globals().set("vim", vim_table)?;
    Ok(())
}
