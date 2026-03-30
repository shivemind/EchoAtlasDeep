# rmtide — 50-Point Build Plan
> Terminal emulator + full IDE + AI CLI integration, written in Rust.

---

## Phase 1 — Core Terminal + Editor Foundation (Points 1–10) ✅ COMPLETE

**1. Cargo workspace scaffold**
Nine crates (`core`, `terminal`, `editor`, `ui`, `lsp`, `mcp`, `ai`, `git`, `plugin`) + binary `bin/rmtide`. Shared dependency versions pinned at workspace level.

**2. Core: error types, typed IDs, IdGen**
`EchoError` enum with per-subsystem variants. Newtype IDs (`PaneId`, `BufferId`, `SessionId`, `RequestId`, `LanguageId`) backed by atomic `u32` generators to prevent cross-subsystem confusion.

**3. Core: AppEvent enum + EventBus**
Comprehensive `AppEvent` enum covers input, terminal, layout, editor, LSP, AI, MCP, git, config, and system events. `EventBus` wraps `tokio::broadcast` (capacity 1024) so every subsystem can publish and subscribe without direct coupling.

**4. Terminal: cross-platform PTY abstraction**
`PtyHandle` trait with Unix (`openpty` + `fork`/`execvpe` via `nix`) and Windows (ConPTY via `CreatePseudoConsole`) implementations. `PtyConfig` carries shell, args, env, initial size, working dir.

**5. Terminal: VT byte-stream parser**
Thin wrapper around the `vte` crate. Converts raw bytes into a typed `VtEvent` stream (Print, Execute, CsiDispatch, OscDispatch, EscDispatch, Hook/Put/Unhook).

**6. Terminal: VT100/220/xterm processor**
Full cursor movement (A–D, H, G, f, d), erase (J/K/X), scroll (L/M), SGR colors (8/16/256/RGB), text attributes (bold/dim/italic/underline/blink/reverse/hidden/strikethrough), alternate screen, bracketed paste, mouse modes (1000–1003), focus events, OSC title/hyperlinks, wide-character (CJK) support.

**7. Terminal: ScreenBuffer**
2D `Vec<Cell>` grid with per-row dirty tracking (`BitVec`), scrollback `VecDeque` (configurable limit), resize with content preservation, scroll-up/down, erase operations.

**8. Terminal: PtySession async lifecycle**
Glues PTY + ScreenBuffer + VtParser + VtProcessor into one async unit. Background read loop (16 KB buffer). `write_input()` via mpsc. `resize()` propagates to both PTY and screen. 10 000-line scrollback default.

**9. Editor: piece-table buffer + registry**
`PieceTable` with original + append buffers and O(log n) insert/delete. Line index rebuilt on mutation. `EditorBuffer` wraps it with `BufferId`, path, line-ending detection (LF/CRLF/CR), dirty flag. `BufferRegistry` holds `Arc<RwLock<EditorBuffer>>` per ID.

**10. UI: layout engine, 60 FPS renderer, crossterm input**
Binary-tree `LayoutNode` (Leaf | Split with direction + ratio). `HostTerminal` RAII guard for raw mode + alternate screen + mouse capture. Dedicated render task at 60 FPS with dirty-signal throttle. `spawn_input_task` translates crossterm events to `AppEvent`. Status bar with mode badge, file name, branch, AI backend, cursor position.

---

## Phase 2 — IDE Editing Layer (Points 11–18)

**11. Modal editing engine**
Normal / Insert / Visual (char, line, block) / Operator-pending / Replace modes. Full Vim motion vocabulary: `w`/`b`/`e`, `f`/`t`/`;`/`,`, `gg`/`G`, `%`, `{`/`}`, `0`/`^`/`$`, `H`/`M`/`L`. Operator + motion combinations (`d3w`, `ci"`, `ya(`). Repeat count prefix. `.` repeat last change. Register system (`"a`–`"z`, `"0`–`"9`, `""`, `"+`, `"*`).

**12. Command-line mode**
`:` command prompt with history (up/down). Built-in commands: `:w [file]`, `:q[!]`, `:wq`, `:e <path>`, `:sp`/`:vs`, `:tabnew`, `:bn`/`:bp`, `:bd`, `:set`, `:map`/`:noremap`, `:!<shell>`, `:r !<shell>`. Tab-completion for paths and command names.

**13. Syntax highlighting via tree-sitter**
`tree-sitter` + language grammars compiled as Rust crates (Rust, Python, JavaScript, TypeScript, Go, C, C++, JSON, TOML, YAML, Markdown, Bash, SQL, HTML, CSS). Incremental re-parse on buffer mutation. Map capture names to theme token types. Injected languages (e.g., SQL in Rust strings).

**14. File picker and directory tree**
Fuzzy-search file picker (Ctrl-P): scans workspace, scores with Smith-Waterman, renders ranked list with path highlighting. Directory tree pane (`:Ex`): lazy expansion, file/dir icons (Nerd Font), create/rename/delete operations. Recent files list.

**15. Search and replace**
In-buffer `/`/`?` with regex (via `regex` crate), `n`/`N` navigation, match highlighting. `:%s/pat/rep/gci` with preview and confirm-per-match. Global search (`:grep`/`:rg`) populates a quickfix list pane. `]q`/`[q` quickfix navigation.

**16. Multi-cursor and multiple selections**
`Ctrl-D` to add cursor at next match of current word. `Alt-click` to place additional cursors. Column-select via Visual-block. All editing operations (insert, delete, change, surround) apply to every cursor simultaneously. Collapse to single cursor on Escape.

**17. Undo/redo tree**
Persistent branching undo history (never loses a change). `:undolist` shows tree. `u`/`Ctrl-R` traverse linear path. `:earlier`/`:later` with time-based navigation (`5m`, `3h`). Undo history survives buffer close if undo file is enabled. Written to `.rmtide/undo/`.

**18. Code folding**
Indentation-based folding (always available). Syntax-aware folding from tree-sitter ranges (functions, classes, blocks, imports). `za`/`zc`/`zo`/`zR`/`zM` keybindings. Fold markers shown in sign column. Fold state persisted per file in session.

---

## Phase 3 — LSP Client (Points 19–25) ✅ COMPLETE

**19. JSON-RPC 2.0 transport**
Framed `Content-Length` header protocol over async stdio pipes (primary) and TCP sockets (optional). Pending-request map with `RequestId` keys. Notification dispatch. Cancellation via `$/cancelRequest`. Timeout and retry logic. Per-server send/receive queues backed by `tokio::mpsc`.

**20. Language server process manager**
Auto-detect server binaries from `$PATH` and project config (`.rmtide.toml`). Supported out of the box: `rust-analyzer`, `pyright`/`pylsp`, `typescript-language-server`, `gopls`, `clangd`, `lua-language-server`. One server instance per workspace root. Restart-on-crash with back-off. Capability negotiation on `initialize`.

**21. Diagnostics engine**
`textDocument/publishDiagnostics` notifications stored per URI. Inline virtual text at end of line (truncated). Sign-column markers (error ✖, warning ▲, info ●, hint ·). Diagnostics panel pane listing all issues sorted by severity then file. `]d`/`[d` jump between diagnostics. Hover tooltip shows full message.

**22. Completion engine**
Trigger on configured characters and `Ctrl-Space`. `textDocument/completion` request with cancellation of stale requests. Popup menu widget (label, kind icon, detail, source). Snippet expansion (LSP snippet syntax) with tab-stop navigation. `textDocument/completionItem/resolve` for lazy-loaded documentation. Score-sorted with prefix filter.

**23. Hover documentation**
`textDocument/hover` on cursor-dwell (configurable delay) and `K` keymap. Floating window rendered with markdown → ratatui spans (bold, italic, code, fenced code blocks with syntax highlighting). Scrollable if content exceeds viewport. Dismiss on cursor move.

**24. Go-to-definition / references / rename**
`gd` → `textDocument/definition` (peek or jump). `gr` → `textDocument/references` → quickfix list. `gD` → `textDocument/declaration`. `gi` → `textDocument/implementation`. `<leader>rn` → `textDocument/rename` with workspace-wide preview before apply. `<leader>ca` → `textDocument/codeAction` popup.

**25. Code actions and auto-fix**
`textDocument/codeAction` triggered on cursor position. Rendered as a floating menu. Applies `WorkspaceEdit` (create/rename/delete files + text edits across multiple URIs). Auto-fix-on-save for `source.fixAll` actions. Format-on-save via `textDocument/formatting` / `textDocument/rangeFormatting`.

---

## Phase 4 — Git Integration (Points 26–30) ✅ COMPLETE

**26. Repository detection and status**
`git2-rs` to open repo from working directory (walks up). Background watcher (`notify` crate) on `.git/index` and working tree for live updates. Status map: `Modified`, `Staged`, `Untracked`, `Conflicted`, `Renamed`, `Deleted` per file. Emits `GitStatusChanged` event on change.

**27. Inline diff gutter**
Compare working-tree buffer content to `HEAD` blob on every save. Compute line-level diff (Myers algorithm). Sign-column markers: `+` (added, green), `~` (modified, yellow), `_` (deleted below, red). `]h`/`[h` jump between hunks. `<leader>hp` preview hunk in floating diff pane. `<leader>hs`/`<leader>hu` stage/unstage hunk.

**28. Git blame annotations**
`:Gblame` or `<leader>gb` opens blame virtual-text mode: each line prefixed with short SHA + author + relative date (e.g., `a1b2c3d  shive  2 days ago`). Click/enter on blame entry opens full commit in floating window. Toggle on/off. Uses `git2` `blame` API, cached per file.

**29. Interactive git panel**
Side-panel pane listing staged changes, unstaged changes, untracked files. `s`/`u` stage/unstage file or hunk. `=` expand inline diff. `cc` open commit message buffer (with template). `<Enter>` on commit launches editor. `P` push. `F` fetch. `gl` show log graph. `<leader>gd` open diff split for file under cursor.

**30. Branch and remote operations**
`:Gbranch` panel: list local + remote branches, `<Enter>` to checkout, `n` to create, `D` to delete. Merge/rebase picker. Conflict resolution: 3-way diff view (ours | base | theirs) with `<leader>co`/`<leader>ct` to pick side, `<leader>cb` to keep both. Push/pull with credential helper support (ssh-agent, libsecret, keychain).

---

## Phase 5 — AI CLI Backends (Points 31–40) ✅ COMPLETE

**31. AI backend trait**
`AiBackend` async trait: `stream_completion(messages, opts) -> Stream<AiStreamChunk>`, `list_models() -> Vec<ModelInfo>`, `health_check() -> bool`, `name() -> &str`. `AiStreamChunk` carries delta text, finish reason, token counts. Backend registry keyed by name. Hot-swap without restart.

**32. Claude CLI backend**
Detect `claude` binary on `$PATH`. Invoke `claude --output-format stream-json --model <model>` with prompt piped to stdin. Parse streaming JSON lines into `AiStreamChunk`. Support `--system`, `--max-tokens`, `--temperature`. Fallback to Anthropic REST API (`https://api.anthropic.com/v1/messages`) if CLI absent and `ANTHROPIC_API_KEY` set. Models: claude-opus-4-6, claude-sonnet-4-6, claude-haiku-4-5.

**33. Gemini CLI backend**
Detect `gemini` binary (Google AI Studio CLI). Invoke with prompt, parse streaming output. Fallback to `https://generativelanguage.googleapis.com/v1beta/models/{model}:streamGenerateContent` with `GEMINI_API_KEY`. Models: gemini-2.0-flash, gemini-2.0-pro, gemini-1.5-pro, gemini-1.5-flash.

**34. OpenAI / Codex backend**
Detect `openai` CLI or direct REST API via `OPENAI_API_KEY`. Stream from `https://api.openai.com/v1/chat/completions` (SSE). Models: gpt-4o, gpt-4o-mini, o1, o3-mini. Codex-specific: pass file context as system message with repo map.

**35. Ollama backend**
HTTP client to `http://localhost:11434/api/chat` (streaming NDJSON). Auto-discover running Ollama instance. `list_models()` fetches from `/api/tags`. Supports any locally installed model (llama3, mistral, deepseek-coder, qwen2.5-coder, phi-4, etc.). Configurable endpoint for remote Ollama.

**36. AI chat pane widget**
Dedicated `AiChat` pane kind. Conversation history rendered as scrollable list: user messages (right-aligned, cyan), assistant messages (left-aligned, white), system messages (dimmed). Code blocks in assistant responses syntax-highlighted. Streaming updates character-by-character as chunks arrive. `i`/`a` to enter compose mode. `Ctrl-Enter` to send. `yy` to yank code block at cursor.

**37. Inline AI completion (ghost text)**
On idle (configurable delay, default 800 ms), send cursor context (preceding N lines + file type) to active backend. Render returned suggestion as dimmed ghost text after cursor. `Tab` to accept full suggestion. `Ctrl-Right` to accept word-by-word. `Escape` to dismiss. Cancels in-flight request on any keystroke. Disable per-buffer with `:set noaighost`.

**38. Context injection**
When sending to AI: automatically attach current file content (truncated to token budget), visual selection if active, active LSP diagnostics, current git diff (`HEAD` → working tree), open buffers list, project language summary. Context window budget managed per backend (configurable `max_context_tokens`). User can toggle each context source with `:AiContext` picker.

**39. AI command palette**
`:Ai <prompt>` sends prompt with current context and streams response into a new split. `<leader>ae` explain selection. `<leader>af` fix diagnostics in selection. `<leader>at` generate tests for function under cursor. `<leader>ad` generate docstring. `<leader>ar` refactor selection (describe change in command line). All commands stream diff into a preview buffer with `<leader>ay` to apply.

**40. Model switcher and status bar**
`<leader>am` opens model picker: lists all backends + their available models. Selecting switches active backend live. Status bar right segment shows `[claude:sonnet-4-6]` or `[ollama:llama3]`. Per-buffer backend override (e.g., use cheap model for ghost text, expensive model for chat). Token usage shown in status bar after each response. Cost estimate for API-key backends.

---

## Phase 6 — MCP Server (Points 41–45) ✅ COMPLETE

**41. MCP JSON-RPC server**
`rmtide --mcp` or background mode. Listens on configurable `bind_addr:port` (default `127.0.0.1:7878`) and stdio. Implements MCP protocol: `initialize`, `tools/list`, `tools/call`, `resources/list`, `resources/read`, `prompts/list`, `prompts/get`. TLS optional. Auth via shared secret header. Registered with Claude Desktop / other MCP clients via config.

**42. MCP filesystem tools**
`read_file(path)` → file contents. `write_file(path, content)` → write with diff preview. `list_directory(path)` → entries with metadata. `search_files(pattern, root)` → glob/regex search. `move_file` / `delete_file` / `create_directory`. All paths sandboxed to workspace root. Dangerous operations require confirmation event back to UI.

**43. MCP editor tools**
`open_file(path, line?)` → opens buffer in editor pane, scrolls to line. `get_diagnostics(path?)` → LSP diagnostics for file or entire workspace. `get_selection()` → current visual selection text + range. `apply_edit(path, range, new_text)` → applies text edit with undo entry. `run_command(cmd)` → executes editor command (`:w`, `:bd`, etc.).

**44. MCP terminal tools**
`new_terminal(cwd?, shell?)` → spawns new PTY pane, returns pane_id. `send_input(pane_id, text)` → writes to PTY stdin. `get_output(pane_id, lines?)` → reads last N lines from scrollback. `resize_pane(pane_id, rows, cols)`. `kill_pane(pane_id)`. Enables external AI agents to drive shell sessions directly inside rmtide.

**45. MCP resources and prompts**
Resources: `workspace://files` (file tree), `workspace://git-status` (current git status), `workspace://diagnostics` (all LSP errors/warnings), `workspace://open-buffers` (list of open files + content). Prompts: pre-built prompt templates for explain-code, write-tests, fix-diagnostics, summarize-diff. Resources updated live via subscriptions (`resources/subscribe`).

---

## Phase 7 — Plugin System + Polish (Points 46–50) ✅ COMPLETE

**46. WASM plugin host**
`wasmtime` runtime in `crates/plugin`. Plugin API exposed as WASM imports: `echo_log`, `echo_emit_event`, `echo_subscribe_event`, `echo_read_file`, `echo_write_file`, `echo_set_keymap`, `echo_register_command`, `echo_register_autocmd`. Plugins loaded from `~/.config/rmtide/plugins/*.wasm` and `.rmtide/plugins/`. Sandboxed (no ambient authority). Example plugin: file-icons, zen-mode.

**47. Lua scripting layer**
`mlua` crate with Lua 5.4. `init.lua` at `~/.config/rmtide/init.lua` loaded on startup. Full API: `vim`-compatible `vim.keymap.set`, `vim.api.*`, `vim.opt`, `vim.cmd`, autocmds, user commands. Lua plugins in `~/.config/rmtide/lua/`. Async-safe: Lua callbacks scheduled on main event loop. Ships with a Lua LSP (lua-language-server auto-configured). Enables full Neovim-style extensibility.

**48. Theme engine**
`Theme` struct: 40+ named token slots (keyword, string, number, comment, type, function, variable, operator, punctuation, diff-add, diff-del, diff-change, UI chrome, statusbar, cursor, selection, etc.) mapped to `Color` (RGB or Indexed). Themes defined in TOML files. Built-in themes: `catppuccin-mocha`, `catppuccin-latte`, `tokyonight`, `gruvbox-dark`, `gruvbox-light`, `one-dark`, `solarized-dark`. Live-reload on file change. `:colorscheme <name>` to switch.

**49. Configuration and hot-reload**
Three-layer config: built-in defaults → `~/.config/rmtide/config.toml` (user) → `.rmtide.toml` (workspace). Per-language settings (`[language.rust]`, `[language.python]`). Key sections: `[editor]`, `[terminal]`, `[ai]`, `[lsp]`, `[git]`, `[ui]`, `[mcp]`. `notify` watcher on all config files — reload without restart. `:set` shows current values, `:set option=value` changes at runtime. JSON Schema published for editor autocomplete.

**50. Distribution and packaging**
`cargo build --release` produces single static binary (except dynamic libs on Linux). GitHub Actions CI: build matrix for `x86_64-unknown-linux-musl`, `x86_64-pc-windows-msvc`, `aarch64-apple-darwin`, `x86_64-apple-darwin`. Artifacts published to GitHub Releases. `cargo install rmtide` via crates.io. Homebrew formula. Winget manifest. Auto-update check on launch (opt-in). Man page generated from clap. Shell completions (bash/zsh/fish/powershell) via `clap_complete`.

---

## Summary Table

| Phase | Points | Theme | Status |
|-------|--------|-------|--------|
| 1 | 1–10 | Core terminal + editor foundation | ✅ Complete |
| 2 | 11–18 | IDE editing layer | ✅ Complete |
| 3 | 19–25 | LSP client | ✅ Complete |
| 4 | 26–30 | Git integration | ✅ Complete |
| 5 | 31–40 | AI CLI backends | ✅ Complete |
| 6 | 41–45 | MCP server | ✅ Complete |
| 7 | 46–50 | Plugin system + polish | ✅ Complete |
