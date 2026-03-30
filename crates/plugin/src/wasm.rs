#![allow(dead_code, unused_imports, unused_variables)]
use std::path::Path;
use anyhow::Result;
use super::api::{PluginMeta, PluginKind, PluginCommand, PluginEvent};

/// WASM plugin instance.
pub struct WasmPlugin {
    pub meta: PluginMeta,
}

#[cfg(feature = "wasm")]
struct PluginState {
    commands: Vec<PluginCommand>,
}

impl WasmPlugin {
    /// Load a WASM plugin from a `.wasm` file.
    #[cfg(feature = "wasm")]
    pub fn load(path: &Path) -> Result<Self> {
        use wasmtime::*;
        let engine = Engine::default();
        let mut store = Store::new(&engine, PluginState { commands: Vec::new() });
        let module = Module::from_file(&engine, path)?;

        let mut linker: Linker<PluginState> = Linker::new(&engine);

        // Define host imports the plugin can call
        linker.func_wrap("env", "echo_log", |mut caller: Caller<'_, PluginState>, msg_ptr: i32, msg_len: i32, level: i32| {
            let mem = caller.get_export("memory")
                .and_then(|e| e.into_memory())
                .expect("no memory export");
            let data = mem.data(&caller);
            let msg = std::str::from_utf8(&data[msg_ptr as usize..(msg_ptr + msg_len) as usize])
                .unwrap_or("[invalid utf8]")
                .to_string();
            let lvl = match level {
                0 => super::api::LogLevel::Debug,
                1 => super::api::LogLevel::Info,
                2 => super::api::LogLevel::Warn,
                _ => super::api::LogLevel::Error,
            };
            caller.data_mut().commands.push(PluginCommand::Log { level: lvl, message: msg });
        })?;

        linker.func_wrap("env", "echo_read_file", |_caller: Caller<'_, PluginState>, _ptr: i32, _len: i32| -> i32 {
            0
        })?;

        linker.func_wrap("env", "echo_write_file", |_caller: Caller<'_, PluginState>, _path_ptr: i32, _path_len: i32, _content_ptr: i32, _content_len: i32| -> i32 {
            0
        })?;

        linker.func_wrap("env", "echo_emit_event", |_caller: Caller<'_, PluginState>, _ptr: i32, _len: i32| {
        })?;

        linker.func_wrap("env", "echo_set_keymap", |_caller: Caller<'_, PluginState>, _mode_ptr: i32, _mode_len: i32, _lhs_ptr: i32, _lhs_len: i32, _rhs_ptr: i32, _rhs_len: i32| {
        })?;

        let instance = linker.instantiate(&mut store, &module)?;

        // Call plugin init if present
        if let Ok(init_fn) = instance.get_typed_func::<(), ()>(&mut store, "plugin_init") {
            init_fn.call(&mut store, ())?;
        }

        let name = path.file_stem().unwrap_or_default().to_string_lossy().into_owned();

        Ok(Self {
            meta: PluginMeta {
                name: name.clone(),
                version: "0.1.0".into(),
                description: String::new(),
                author: String::new(),
                kind: PluginKind::Wasm,
                path: path.to_path_buf(),
            },
        })
    }

    #[cfg(not(feature = "wasm"))]
    pub fn load(_path: &Path) -> Result<Self> {
        anyhow::bail!("WASM support not compiled in — rebuild with --features wasm")
    }

    /// Dispatch an event to the plugin.
    pub fn on_event(&mut self, _event: &PluginEvent) -> Vec<PluginCommand> {
        vec![]
    }
}
