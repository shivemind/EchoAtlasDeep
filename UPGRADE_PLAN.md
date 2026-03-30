# rmtide — 50-Point Upgrade Plan
> Next-generation features building on the v1.0 foundation.
> All 50 points of the original plan are complete. This document defines the v2.0 roadmap.

---

## Phase 8 — BYOK, Spend Tracking & Approvals (Points 1–10) ✅ Complete

**1. BYOK key management vault**
Secure in-app key manager for all AI providers (Anthropic, OpenAI, Google, Mistral, Cohere, Together, Groq, Perplexity). Keys stored in OS keychain (`keyring` crate — Keychain on macOS, Credential Manager on Windows, libsecret on Linux). UI overlay (`\K`) lets user add/edit/rotate/delete keys per provider. Per-key labels ("work", "personal"), last-used timestamps, and masked display (shows last 4 chars). Keys hot-swapped without restart.

**2. Model spend tracker**
Per-session and cumulative token accounting. Maps (provider, model) → (input_price_per_mtok, output_price_per_mtok) from an embedded pricing table (updated via a bundled JSON, refreshable with `:AiPricing update`). Displays running cost in status bar (`$0.042`). `:AiSpend` opens a breakdown pane: table of (model, requests, input_tokens, output_tokens, cost) sorted by cost descending. Budget alert: user sets `ai.budget_usd` in config; a warning banner appears at 80% and a hard stop prompt at 100%.

**3. Human-in-the-loop approval workflow**
Before any AI-initiated action that mutates state (file write, shell execute, network call, git commit), the editor pauses and presents a diff-style approval modal: action type, affected paths, preview of change. User presses `y` to approve, `n` to deny, `a` to approve all remaining in the batch, `d` to see full diff. Timeout configurable (`ai.approval_timeout_secs`; 0 = wait forever). Approval log written to `.rmtide/approvals.jsonl` for audit. Per-tool granularity: `read_file` auto-approved, `delete_file` always requires approval.

**4. Spend budget controls**
Companion to point 2. Hard budget cap per session and per day. When session budget is hit: AI streaming pauses, a budget-exceeded banner replaces the status bar, and the user can extend, reset, or switch to a cheaper model. Daily rolling budget resets at midnight local time. Per-project budget override in `.rmtide.toml`. `[ai] daily_budget_usd = 5.00`.

**5. Credential rotation reminders**
Track key age from creation timestamp stored alongside the key. Surface a non-blocking notification if any key is older than a user-defined threshold (`keyring.rotation_days = 90`). `:AiKeys` panel shows a `⚠ 95 days old` badge on stale keys. One-click "rotate now" opens the provider's API key page in the system browser and clears the old key from vault.

**6. Multi-provider fallback chains**
Define ordered fallback lists: if the primary backend errors (rate limit, quota exceeded, network timeout), automatically retry on the next provider in the chain. Config: `ai.fallback_chain = ["claude", "openai", "ollama"]`. Each fallback attempt is logged and reflected in the spend tracker. Fallback events emit a brief status-bar notification: `⚡ Fell back to openai (claude rate-limited)`.

**7. Model capability matrix**
`:AiModels` opens a full-screen table showing all available models across all configured providers. Columns: model name, context window, input $/MTok, output $/MTok, supports vision, supports function-calling, supports streaming, latency tier. Rows sortable by any column. Selecting a model switches the active backend. A `(*)` marks the current active model. Filtered by `ai.enabled_providers`.

**8. API key scoping per project**
`.rmtide.toml` can specify `[ai] provider_key = "proj_XXXX"` to override the global vault key for that workspace. Useful for team repos with shared project API keys. Workspace-scoped keys shown with a `[proj]` badge in the key manager. Never written to git (`.rmtide.toml` auto-added to `.gitignore` by the editor on first write if it contains keys).

**9. Usage analytics dashboard**
`:AiAnalytics` pane renders a time-series bar chart (using ratatui sparklines and bar charts) of daily token usage and cost over the past 30 days. Separate series per provider. Shows totals, averages, and peak-day. Data persisted to `~/.local/share/rmtide/analytics.db` (SQLite via `rusqlite`). Export to CSV with `:AiAnalytics export`.

**10. Offline mode and cache**
Toggle offline mode with `:AiOffline`. In offline mode, all AI requests are served from a response cache (last N responses per prompt hash stored in `~/.local/share/rmtide/cache/`). New requests that miss the cache show a clear `[OFFLINE — no cached response]` placeholder. Ghost text disabled. LSP and all local tools continue operating. Ideal for flights, low-connectivity environments.

---

## Phase 9 — Agentic Loop & Agent Chat (Points 11–20) ✅ Complete

**11. Full agentic task loop**
`\A` opens the Agent panel. User types a high-level task ("add authentication to this Express app"). The agent: (1) emits a **Plan** (numbered steps shown in a collapsible tree), (2) executes steps one at a time, each requiring approval if it mutates state (see point 3), (3) verifies results (runs tests, checks LSP diagnostics), (4) reports completion with a summary diff. The loop continues until the task is done or the user interrupts. Interrupt with `Ctrl-C`; resume with `:AgentResume`. Agent state persisted to `.rmtide/agent-session.json`.

**12. Tool-call trace panel**
When an agent is running, the right sidebar shows a live **tool call trace**: each tool invoked (read_file, write_file, run_command, lsp_hover, etc.) appears as a collapsible row with arguments, result snippet, duration, and token cost. Errors highlighted in red. The trace is scrollable and searchable. Exportable as JSON with `:AgentTrace export`.

**13. Multi-agent orchestration**
`:AgentSpawn <role>` spawns a named sub-agent with a specific persona (e.g. `--role reviewer`, `--role tester`, `--role architect`). A coordinator agent decomposes the task and routes sub-tasks to specialised agents running in parallel tokio tasks. Results are aggregated. Sub-agent status shown in a multi-row agent panel with individual progress bars. Max concurrent agents configurable (`ai.max_agents = 4`).

**14. Agent memory and context persistence**
Each agent session maintains a structured memory store (key-value, backed by a local SQLite DB). Agent can call `memory_set(key, value)` and `memory_get(key)`. Memory persisted across sessions so long-running projects accumulate project knowledge. `:AgentMemory` panel shows all stored keys; user can inspect and delete entries. Memory injected into system prompt automatically (budget-aware truncation).

**15. Agent code review**
`\R` on a diff or selection sends it to the code review agent. Returns structured feedback: a list of issues each with severity (critical/major/minor/nit), location (file + line range), explanation, and suggested fix. Rendered in a dedicated **Review** pane with file-grouped sections. `]i`/`[i` jumps between issues. `<leader>rf` applies the suggested fix (goes through approval). `<leader>ra` accepts all minor/nit issues at once.

**16. Autonomous test-fix loop**
`:AgentTestFix` runs the project's test suite (auto-detected: `cargo test`, `pytest`, `npm test`, `go test`, etc.), captures failures, sends failing tests + relevant source to the AI, applies fixes, re-runs tests, loops until all pass or a retry limit is hit (default 5). Each iteration shown in a loop panel with pass/fail delta. The final diff (all changes combined) shown for approval before writing.

**17. Plan editing before execution**
When the agent emits a Plan (step 11), the user can edit the plan text directly in the agent panel before confirming execution. Add, delete, or reorder steps. The agent re-validates the modified plan (checking for consistency) and then proceeds. Plan edits are logged alongside approvals in `.rmtide/approvals.jsonl`.

**18. Agent conversation history**
Full multi-turn agent conversation stored per project in `.rmtide/agent-history/`. `:AgentHistory` opens a searchable list of past sessions (date, task summary, outcome, cost). Select a session to view the full trace. Resume a past session with `<Enter>`. Useful for auditing what the agent changed over time.

**19. Context injection controls**
`:AiContext` overlay (already scaffolded) becomes fully interactive. Checkboxes for each context source: current file, open buffers, LSP diagnostics, git diff (staged/unstaged), git log (last N commits), clipboard, terminal scrollback, custom snippets. Token budget bar updates live as sources are toggled. Saved per-project in `.rmtide.toml`. Agent uses the same context sources.

**20. Prompt library**
`:AiPrompts` panel — a personal/team prompt library. CRUD for named prompts with variables (`{{selection}}`, `{{file}}`, `{{language}}`). Prompts stored in `~/.config/rmtide/prompts/` as TOML files. Share prompts with the team by committing to `.rmtide/prompts/`. Quick-access via `\p` fuzzy picker. Prompts usable in agent chat, inline AI, and via MCP `prompts/get`.

---

## Phase 10 — File Tree, Tabs & Editor Power (Points 21–30) ✅ Complete

**21. Full sidebar file tree**
Persistent sidebar (toggle with `\e`) showing the workspace directory tree with Nerd Font file/folder icons, git status badges (`M` / `A` / `?` / `!`), file sizes, and last-modified times. Keyboard navigation: `j`/`k` move, `<Enter>` open, `o` open in split, `t` open in new tab, `r` rename, `d` delete (with confirmation), `a` create file, `A` create directory, `c` copy, `x` cut, `p` paste, `y` yank path, `/` filter. Right-click context menu via mouse. Lazy-loading of large directories. Respect `.gitignore` by default (toggle with `I`).

**22. Multi-tab buffer bar**
Full tab bar at the top of the editor area showing all open buffers. Each tab: file icon + short name + `[+]` if dirty. `gt`/`gT` cycle tabs. `<leader>1`–`<leader>9` jump to tab by index. `<leader>tc` close tab. `<leader>to` close all other tabs. Tabs reorderable via mouse drag. Middle-click to close. Tabs persist in session (restored on next launch). `:tabnew`, `:tabclose`, `:tabonly` commands. Tab overflow handled with `«` / `»` scroll arrows.

**23. Project-wide find and replace**
`:Rg` / `<leader>fg` opens a two-pane find-replace overlay: top pane is the search query (regex, with case/word-boundary toggles), bottom pane is the replace string. Results list shows all matches grouped by file with preview lines. `<Space>` to toggle individual matches, `a` to toggle all in file, `r` to execute replace on selected matches. Progress bar during indexing. Undo is a single `u` step (all replacements wrapped in one undo group). Powered by ripgrep subprocess.

**24. Symbol browser**
`<leader>ss` opens workspace-wide symbol search using LSP `workspace/symbol`. Results shown in a fuzzy-searchable picker with symbol kind icons (function ƒ, type τ, const K, variable v, module M). Selecting jumps to definition. `<leader>sS` searches only the current file (`textDocument/documentSymbol`). Results also shown in a persistent **Outline** sidebar panel (toggle `\o`) that tracks the cursor position and highlights the current symbol.

**25. Integrated debugger (DAP)**
Debug Adapter Protocol client in a new `crates/dap/` crate. Auto-detect debug configs from `.vscode/launch.json` or `.rmtide/debug.toml`. `<F5>` start debugging, `<F9>` toggle breakpoint, `<F10>` step over, `<F11>` step into, `<F12>` step out, `<F5>` continue, `<S-F5>` stop. Breakpoints shown as red dots in the sign column. Debug panels: Variables (locals + watch expressions), Call Stack, Breakpoints list, Debug Console (REPL). Supports: `codelldb` (Rust/C/C++), `debugpy` (Python), `node --inspect` (JS/TS), `dlv` (Go).

**26. Bookmarks and jump list**
`mm` toggle bookmark on current line. `]m`/`[m` jump to next/previous bookmark. `<leader>bm` open bookmarks picker. Bookmarks stored per file in `.rmtide/bookmarks.toml`. Jump list (Ctrl-O / Ctrl-I) tracks last 100 cursor positions across files. Change list (`g;` / `g,`) tracks edit positions. All persisted across sessions.

**27. Code minimap**
Toggleable minimap sidebar (`:Minimap` / `\mm`) showing a scaled-down rendering of the full file. Current viewport highlighted as a shaded region. Click or drag minimap to scroll. Highlights: LSP errors (red ticks), warnings (yellow), search matches (cyan), git change markers. Updates on every buffer change. Width configurable (default 12 cols).

**28. Macro recorder and manager**
`q<register>` starts recording, `q` stops. `@<register>` replays. `@@` replays last macro. `:Macros` panel lists all named macros with a preview of the keystrokes. Macros saved to `~/.config/rmtide/macros.toml`. Macros can be assigned names and descriptions. `<leader>M` opens macro picker. Macros can call Lua functions (bridged via the Lua API in the plugin system).

**29. Clipboard history ring**
Every yank is appended to a 100-entry clipboard ring (in-memory + persisted to `~/.local/share/rmtide/clipboard.json`). `<leader>p` opens a fuzzy-searchable clipboard picker showing entry preview, size, and time. Select to paste. Ring entries can be pinned (never evicted). Support for image clipboard entries (stored as base64, displayed as info). Integrates with the system clipboard on `"+p`.

**30. Session manager**
`:Sessions` panel: save, load, delete named workspace sessions. A session captures: open files + cursor positions, split layout, active theme, terminal content (scrollback snapshot), fold state per buffer, and search/replace history. Auto-save on quit (configurable). `:SessionSave <name>`, `:SessionLoad <name>`. Fuzzy-searchable picker. Sessions stored in `~/.local/share/rmtide/sessions/`.

---

## Phase 11 — Task Runner, Logs & Live Server (Points 31–40) ✅ Complete

**31. Task runner**
A `tasks.toml` file at workspace root defines named tasks:
```toml
[tasks.build]
command = "cargo build"
cwd = "."
env = { RUST_LOG = "debug" }
watch = ["src/**/*.rs"]

[tasks.test]
command = "cargo test"
depends = ["build"]
```
`<leader>tr` opens the task runner panel: list of tasks with last-run status (✅/❌/⏳), duration, and log snippet. `<Enter>` runs selected task. `<leader>tw` toggles file-watcher mode (re-runs on file change). Tasks run in a dedicated terminal pane. Multiple tasks can run in parallel (each in its own pane). Task output is searchable, filterable by log level.

**32. Integrated log viewer**
`:Logs` pane aggregates log output from all running tasks, terminals, LSP servers, and the editor itself. Structured log entries parsed from JSON (tracing-subscriber JSON format) and plain text. Real-time filtering: by level (error/warn/info/debug/trace), by source (task name, LSP server name, `editor`), by regex. Color-coded by level. Scrollback of 50,000 lines. `G` to follow tail, `gg` to stop following. Export to file with `:Logs export`.

**33. Built-in live server**
`:LiveServer` spawns a local HTTP server (using `hyper` or `axum`) serving the workspace root (or a configurable `serve.root` directory). Auto-detects the project type: for web projects, injects a `<script>` WebSocket snippet into HTML responses that triggers browser auto-reload when files change (via the `notify` watcher). For API projects, acts as a mock server using a `mock.toml` route table. Status shown in the process manager (point 35). URL displayed in status bar. `<leader>ls` open in system browser.

**34. Diff review panel**
`:DiffReview` or `<leader>dr` opens a dedicated two-column diff pane. Left: old version (HEAD or any git ref), right: current. Each hunk shown with context lines. Navigation: `]h`/`[h` jump hunks, `<leader>hs` stage hunk, `<leader>hr` revert hunk, `<leader>ha` accept AI suggestion for hunk. AI review mode (`<leader>dai`): sends each hunk to the AI, which annotates it with inline comments (shown as virtual text below the hunk). Comments can be accepted as code changes or dismissed.

**35. Process manager**
`:Procs` panel lists all background processes spawned by the editor (live servers, file watchers, LSP servers, task runner jobs, agent sub-processes). Columns: PID, name, status, CPU%, memory, uptime, port (if listening). `k` to kill, `r` to restart, `l` to open log pane for that process, `<Enter>` to attach terminal. Processes auto-restarted on crash if `restart = true` in their config. Port numbers are clickable (opens system browser).

**36. Port manager and tunnel**
Auto-discovers all listening ports on localhost via `/proc/net/tcp` (Linux) or `netstat` (Windows/macOS). `:Ports` panel shows process name, port, protocol (HTTP/HTTPS/TCP), and uptime. One-key actions: `o` open in browser, `t` create a public tunnel (via `cloudflared tunnel` or `ngrok` CLI if available), `k` kill the process. Tunnel URLs displayed in a banner with one-click copy. Tunnel status tracked in the process manager.

**37. Deployment panel**
`:Deploy` panel integrates with popular hosting providers. Auto-detects project type and suggests providers:
- **Fly.io**: `fly deploy` with app name, region, config from `fly.toml`
- **Vercel**: `vercel deploy` with environment and preview/production toggle
- **Netlify**: `netlify deploy` with site ID from `netlify.toml`
- **Railway**: `railway up` with project/service selectors
- **Docker**: `docker build` + `docker push` with registry config
Each provider has a status panel: last deploy timestamp, deploy URL, environment variables editor (masked), deploy log stream. Secrets never logged. Rollback to previous deploy with `<leader>drb`.

**38. Environment manager**
`:Env` panel shows all `.env*` files in the workspace (`.env`, `.env.local`, `.env.production`, etc.). Tree view of all key-value pairs with secret masking (toggle with `<leader>ev`). Add/edit/delete entries with undo. Duplicate a key across env files. Validates that required keys (from `.env.example`) are present in all env files — missing keys shown in red. Masks secrets in terminal output (regex-based scrubbing of all PTY output matching known key patterns). Export env as shell `export` statements.

**39. HTTP client / REST console**
`:Http` pane — a full REST client. Create, save, and organize HTTP requests in a collection (`.rmtide/http/`). Request editor: method selector, URL bar, headers table, body (JSON/form/raw). Response viewer: status code, headers, pretty-printed JSON/HTML body, timing breakdown (DNS / connect / TTFB / total). Response history. Variables: `{{base_url}}`, `{{auth_token}}` resolved from the environment manager. `<leader>hs` send request. Collection importable from OpenAPI/Swagger specs (`:Http import openapi.yaml`). cURL command generation with `y`.

**40. Database client**
`:DB` panel connects to PostgreSQL, MySQL, SQLite, and Redis. Connection strings stored in the key vault (point 1). Schema browser: databases → tables → columns with types. SQL editor with LSP-like autocomplete (table/column names). Results rendered as a scrollable table. Syntax highlighting for SQL. Query history. Export results as CSV/JSON. Explain plan visualization for SELECT queries. Runs queries in a tokio task; cancellable with `Ctrl-C`. Multiple connections open simultaneously in tabs.

---

## Phase 12 — Intelligence, Security & Polish (Points 41–50) ✅ Complete

**41. Semantic code search**
`:SemanticSearch <query>` sends a natural language query to the AI which translates it to a structured search (symbol names, patterns, file paths) and executes it via ripgrep + LSP. Results ranked by semantic relevance, not just text match. "Find all places where we handle authentication errors" returns relevant code across all files. Caches embeddings locally (optional, requires local embedding model via Ollama). `<leader>fs` keyboard shortcut.

**42. AI commit message generator**
When the user runs `cc` in the git panel (stage-and-commit flow), the AI analyzes the staged diff, infers the intent, and drafts a conventional-commit message (`feat:`, `fix:`, `chore:`, etc.) with a concise description and optional body. Draft shown in an editable buffer. User reviews, edits, confirms with `<leader>gc`. The system learns from the user's edits to the draft — a local fine-tuning signal stored in `~/.local/share/rmtide/commit-feedback.jsonl`.

**43. Security scanner**
`:SecScan` runs a multi-layer security scan:
1. **Secret detection**: scans all tracked files for API keys, tokens, passwords using pattern matching (honoring `.secretignore`).
2. **Dependency audit**: runs `cargo audit`, `npm audit`, `pip-audit` depending on project type.
3. **AI code review for OWASP Top 10**: sends security-sensitive files (auth, crypto, input parsing) to the AI with a security-focused prompt.
Results shown in a severity-sorted list (Critical/High/Medium/Low/Info). Each finding links to the affected line. Auto-fix available for secret exposure (removes from history via `git filter-repo`).

**44. Workspace analytics**
`:Analytics` dashboard with multiple views:
- **Code health**: LOC per language, complexity metrics (cyclomatic via tree-sitter), dead code indicators (LSP unused warnings aggregated).
- **Churn**: git log analysis — files changed most frequently in last 90 days (high churn = refactoring candidate).
- **Coverage**: parse `lcov.info` / `coverage.xml` and render per-file coverage percentage in the file tree (colored bars).
- **Velocity**: commits/day sparkline, PRs merged, issues closed (from GitHub API if configured).
All visualized as ratatui bar charts, sparklines, and tables.

**45. Notebook mode**
Open `.ipynb` files (Jupyter notebooks) in a dedicated notebook layout: cells rendered as numbered blocks (markdown cells rendered, code cells with syntax highlighting). Execute code cells with `<Enter>` (requires running Jupyter kernel — auto-started via `jupyter kernel`). Output shown inline below the cell (text, error, rich output described as text). Add cell above/below with `a`/`b`. Change cell type with `m` (markdown) / `y` (code). Save with `:w`. Compatible with existing Jupyter workflows.

**46. Custom keybinding editor**
`:Keymaps` full-screen panel. Left column: all available commands grouped by category. Right column: current bindings for the selected command (multiple bindings allowed). Edit: press `e` to capture a new keystroke sequence. Conflict detection: shows all commands that share a binding with a warning. Reset individual binding to default with `<leader>kr`. Export all keymaps to `~/.config/rmtide/keymaps.toml`. `?` shows a filterable cheat-sheet of all current bindings in a floating overlay.

**47. Plugin marketplace**
`:PluginBrowse` opens a TUI marketplace backed by a curated registry (JSON file fetched from a GitHub-hosted index). List of plugins with name, description, author, star count, last-updated date, and kind (WASM/Lua). `<Enter>` opens the plugin's README in a floating viewer. `i` to install (downloads `.wasm` or `.lua` to `~/.config/rmtide/plugins/`, loads immediately). `u` to update all. `d` to uninstall. Installed plugins shown with a `✓` badge. Signed WASM plugins verified against author public key before execution.

**48. Collaborative editing (experimental)**
`:Collab share` starts a collaboration session using a CRDT (Conflict-free Replicated Data Type) layer over WebRTC. Generates a shareable session URL. Remote participants join via `rmtide --join <url>` or a web-based viewer. Each participant has a named cursor shown in a distinct color. All edits merged in real time via the Yjs CRDT algorithm (Rust port: `yrs`). Chat sidebar for session communication. Presence indicators in the tab bar (avatar initials). Session ends when host closes — no central server required.

**49. AI pair programmer mode**
`<leader>ap` activates pair programmer mode: the AI observes every keystroke in real time (batched into 2s windows) and proactively offers suggestions not just for the current line but for the surrounding architectural context. Suggestions appear in a non-intrusive side-panel (not ghost text) ranked by confidence. Each suggestion has a one-line rationale. User can apply with `Tab`, open a detailed explanation with `?`, or dismiss with `Escape`. The AI also proactively flags potential bugs as you type (before saving), separate from LSP diagnostics.

**50. Unified command palette**
`Ctrl-Shift-P` (or `\:`) opens a universal command palette combining: editor commands, file picker, symbol search, task runner actions, git operations, AI prompts, MCP tool calls, deployment actions, and settings. Fuzzy-searchable with icons per category. Recent commands floated to the top. Each command shows its keybinding shortcut on the right. Commands can be pinned as favorites. Palette history saved across sessions. Supports multi-word abbreviations (`gp` matches `Git: Push`). The palette is the single entry point — everything in rmtide is discoverable here.

---

## Summary Table

| Phase | Points | Theme | Status |
|-------|--------|-------|--------|
| 8  | 1–10  | BYOK, spend tracking & approvals       | 🔲 Planned |
| 9  | 11–20 | Agentic loop & agent chat              | 🔲 Planned |
| 10 | 21–30 | File tree, tabs & editor power         | 🔲 Planned |
| 11 | 31–40 | Task runner, logs & live server        | 🔲 Planned |
| 12 | 41–50 | Intelligence, security & polish        | 🔲 Planned |

---

## Priority Build Order (recommended)

If building incrementally, prioritise in this order based on user impact:

1. **Point 21** — File tree sidebar (highest daily-use value)
2. **Point 22** — Multi-tab buffer bar
3. **Point 1**  — BYOK key vault (unlocks real usage for all AI features)
4. **Point 2**  — Spend tracker (essential for cost visibility)
5. **Point 3**  — Approval workflow (required before agentic features are safe)
6. **Point 11** — Agentic task loop
7. **Point 34** — Diff review panel
8. **Point 31** — Task runner
9. **Point 32** — Log viewer
10. **Point 33** — Live server
11. **Point 37** — Deployment panel
12. **Point 25** — Debugger (DAP)
