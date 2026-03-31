# rmtide

A terminal IDE built in Rust — modal editing, integrated terminal, AI assistance, Git, LSP, and a debug adapter, all in one TUI.

## Download

Pre-built binaries are attached to every [GitHub Release](../../releases/latest):

| Platform | File |
|----------|------|
| Windows x64 | `rmtide-windows-x86_64.exe` |
| Linux x64 (static musl) | `rmtide-linux-x86_64` |
| macOS ARM64 | `rmtide-macos-arm64` |

Download, make executable (Linux/macOS: `chmod +x rmtide`), and run.

## Quick Start

```
rmtide                    # open with live shell
rmtide path/to/file.rs    # open a file directly
```

On first launch, a terminal pane opens at your current directory. You can type shell commands immediately (`cd`, `ls`, `git status`, etc.).

Press `Ctrl+T` to toggle between **terminal mode** and **editor mode**.

## Modes

rmtide uses Vim-style modal editing:

| Mode | Description |
|------|-------------|
| `NORMAL` | Navigate, run commands |
| `INSERT` | Type text |
| `VISUAL` | Character selection |
| `V-LINE` | Line selection |
| `V-BLOCK` | Block selection |
| `REPLACE` | Overwrite characters |
| `SEARCH` | `/` or `?` search |
| `COMMAND` | `:` command line |
| `TERMINAL` | Live shell input (global) |

The current mode is shown in the left of the status bar.

## Key Bindings

### Global

| Key | Action |
|-----|--------|
| `Ctrl+Q` | Quit |
| `Ctrl+W` | Cycle pane focus |
| `Ctrl+T` | Toggle terminal mode (shell input) |
| `Ctrl+P` | Command palette |

### Panel Toggles

| Key | Panel |
|-----|-------|
| `Alt+E` | File tree |
| `Alt+A` | AI agent |
| `Alt+P` | Prompt library |
| `Alt+M` | Minimap |
| `Alt+B` | Bookmarks |
| `Alt+C` | Clipboard ring |
| `Alt+Q` | Macro panel |
| `Alt+T` | Task runner |
| `Alt+L` | Log viewer |
| `Alt+F` | Semantic search |
| `Alt+Shift+P` | AI pair programmer |
| `F5` | Debugger (DAP) |

### Normal Mode — Navigation

| Key | Action |
|-----|--------|
| `h` `j` `k` `l` | Left / down / up / right |
| `w` `b` `e` | Word forward / backward / end |
| `0` `^` `$` | Line start / first non-ws / end |
| `gg` | File start |
| `G` | File end |
| `Ctrl+D` / `Ctrl+U` | Half-page down / up |
| `/` | Search forward |
| `?` | Search backward |
| `n` / `N` | Next / previous match |

### Normal Mode — Editing

| Key | Action |
|-----|--------|
| `i` `I` | Insert at cursor / line start |
| `a` `A` | Append after cursor / line end |
| `o` `O` | New line below / above + insert |
| `R` | Replace mode |
| `x` / `X` | Delete char forward / backward |
| `dd` | Delete line |
| `cc` | Change line |
| `yy` | Yank line |
| `p` / `P` | Paste after / before |
| `u` | Undo |
| `Ctrl+R` | Redo |
| `.` | Repeat last change |
| `v` / `V` | Visual / visual-line mode |
| `za` | Toggle fold |
| `zM` / `zR` | Fold all / unfold all |

### Normal Mode — Leader Sequences (`\`)

| Key | Action |
|-----|--------|
| `\a` | AI chat |
| `\e` | AI explain selection |
| `\f` | AI fix diagnostics |
| `\t` | AI generate tests |
| `\d` | AI generate docstring |
| `\r` | AI refactor |
| `\m` | Model picker |
| `\K` | Key vault (API keys) |
| `\$` | Spend tracker |
| `\M` | Model matrix |
| `\o` | Toggle offline mode |
| `\gb` | Git blame |
| `\gp` | Git panel |
| `\gs` | Refresh git status |
| `\gB` | Git branch panel |
| `Alt+R` | AI code review (agent) |

### Insert Mode

| Key | Action |
|-----|--------|
| `Escape` | Return to normal mode |
| `Tab` | Accept ghost text / complete |
| `Ctrl+Right` | Accept ghost text word |
| `Ctrl+Space` | LSP autocomplete |
| `Ctrl+W` | Delete word backward |
| `Ctrl+U` | Delete to line start |
| `Ctrl+Enter` | Send AI chat message |

### Command Line (`:`)

| Command | Action |
|---------|--------|
| `:w` | Save |
| `:q` | Quit |
| `:q!` | Force quit |
| `:e <file>` | Open file |
| `:sp` / `:vsp` | Horizontal / vertical split |

### LSP

| Key | Action |
|-----|--------|
| `K` (normal) | Hover |
| `gd` | Go to definition |
| `gr` | Go to references |
| `Ctrl+Space` | Autocomplete |
| `Up` / `Down` | Navigate completions |
| `Enter` | Confirm completion |

## Features

### Terminal Emulator
- Integrated PTY-backed terminal (ConPTY on Windows, openpty on Unix)
- VT100/ANSI escape sequence processing
- 10,000-line scrollback buffer
- Spawns your default shell on startup
- `Ctrl+T` switches input routing between shell and editor

### Editor
- Vim-compatible modal editing with ~90 commands
- Multi-cursor editing
- Code folding
- Undo tree
- Syntax highlighting via tree-sitter
- Ghost text (AI inline completions)

### AI Integration
Supports four backends, switchable at runtime with `\m`:

| Backend | Config key | Requires |
|---------|-----------|---------|
| Claude (Anthropic) | `backend = "claude"` | `anthropic_api_key` |
| Gemini (Google) | `backend = "gemini"` | `google_api_key` |
| OpenAI (GPT) | `backend = "codex"` | `openai_api_key` |
| Ollama (local) | `backend = "ollama"` | Ollama running locally |

- Inline ghost text completions (accept with `Tab`)
- Multi-turn chat panel (`\a`)
- Code explanation, test gen, refactor, docstring generation
- Autonomous agent mode for code review
- Prompt library with templates
- Spend tracker and approval queue for cost control
- Response cache for offline / low-cost mode (`\o`)
- Multi-backend fallback chain

### Git
- Status, diff, stage/unstage from within the editor
- Per-line blame (`\gb`)
- Branch management panel (`\gB`)
- Integrated diff review panel

### LSP
- Language server client for any LSP-compatible language
- Hover, go-to-definition, references, diagnostics
- Autocomplete with fuzzy matching
- Code actions and formatting

### Debugger (DAP)
- Debug Adapter Protocol client
- Breakpoints, step-over/into/out
- Variable inspection
- Debug console

### Other Panels
- **Process Manager** — monitor running processes
- **Task Runner** — build / test tasks
- **Log Viewer** — application and process logs
- **HTTP Client** — REST API client built in
- **Semantic Search** — AI-powered codebase search
- **Bookmarks** — jump to saved locations
- **Clipboard Ring** — clipboard history
- **Macro Panel** — record and replay key sequences
- **Session Manager** — manage multiple terminal sessions

## Configuration

Config is loaded from (in order, later values win):

1. Built-in defaults
2. `~/.config/rmtide/config.toml` (user)
3. `.rmtide.toml` (project root)

Example `~/.config/rmtide/config.toml`:

```toml
shell = "bash"          # default shell (powershell.exe on Windows)
scrollback_lines = 10000
fps = 60
theme = "catppuccin-mocha"

[ai]
backend = "claude"
anthropic_api_key = "sk-ant-..."
google_api_key = "AIza..."
openai_api_key = "sk-..."
max_context_tokens = 100000

[mcp]
bind_addr = "127.0.0.1"
port = 0                # 0 = auto-assign

[editor]
tab_width = 4
expand_tabs = true
line_numbers = true
```

API keys can also be stored securely in the key vault (`\K`) backed by your system keyring.

## Building from Source

Requirements: Rust 1.70+, a C compiler (MSVC on Windows, gcc/clang on Unix).

```bash
git clone https://github.com/<owner>/EchoAtlasDeep
cd EchoAtlasDeep

# development build
cargo build -p rmtide

# release build
cargo build --release -p rmtide

# run directly
cargo run -p rmtide

# with debug logging
RUST_LOG=rmtide=debug cargo run -p rmtide
```

### Cross-compilation targets (GitHub Actions)

| Platform | Rust target |
|----------|------------|
| Windows x64 | `x86_64-pc-windows-msvc` |
| Linux x64 | `x86_64-unknown-linux-musl` |
| macOS ARM64 | `aarch64-apple-darwin` |

## Architecture

```
EchoAtlasDeep/
├── bin/rmtide/          # Binary entry point
│   └── src/
│       ├── main.rs      # Async event loop, input routing
│       ├── app.rs       # AppState (editor, terminal, layout)
│       └── config.rs    # Config loading
└── crates/
    ├── core/            # Shared types: IDs, events, traits
    ├── terminal/        # PTY session, VT parser, screen buffer
    ├── editor/          # Modal engine, buffer, search, undo
    ├── lsp/             # LSP client and manager
    ├── mcp/             # Model Context Protocol server
    ├── ai/              # AI backends, chat, ghost text, agents
    ├── git/             # Git integration (libgit2)
    ├── plugin/          # Plugin system
    ├── ui/              # TUI rendering (ratatui), all widgets
    ├── dap/             # Debug Adapter Protocol client
    └── runner/          # Task runner, processes, HTTP, DB
```

The event loop in `main.rs` handles crossterm input events, dispatches to the modal editor or terminal session based on the active mode, and triggers ratatui renders at the configured FPS.

## License

MIT
