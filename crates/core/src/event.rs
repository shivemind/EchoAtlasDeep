/// Central AppEvent enum — every inter-subsystem message flows through here.
/// Distributed via tokio::sync::broadcast so multiple consumers can subscribe.
use serde::{Deserialize, Serialize};

use crate::ids::{BufferId, PaneId, RequestId, SessionId};

// ─── Input types ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyEvent {
    pub code: KeyCode,
    pub modifiers: KeyModifiers,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum KeyCode {
    Char(char),
    Enter,
    Backspace,
    Delete,
    Escape,
    Tab,
    BackTab,
    Up,
    Down,
    Left,
    Right,
    Home,
    End,
    PageUp,
    PageDown,
    Insert,
    F(u8),
    Null,
}

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
    pub struct KeyModifiers: u8 {
        const NONE    = 0b0000;
        const SHIFT   = 0b0001;
        const CONTROL = 0b0010;
        const ALT     = 0b0100;
        const SUPER   = 0b1000;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MouseEvent {
    pub kind: MouseEventKind,
    pub col: u16,
    pub row: u16,
    pub modifiers: KeyModifiers,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MouseEventKind {
    Down(MouseButton),
    Up(MouseButton),
    Drag(MouseButton),
    Moved,
    ScrollDown,
    ScrollUp,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

// ─── Terminal output ─────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct TerminalOutput {
    pub session_id: SessionId,
    pub data: Vec<u8>,
}

// ─── Resize ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct TermSize {
    pub cols: u16,
    pub rows: u16,
}

// ─── AI streaming ────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct AiStreamChunk {
    pub pane_id: PaneId,
    pub text: String,
    pub is_final: bool,
}

// ─── Completion ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct CompletionReady {
    pub buffer_id: BufferId,
    pub request_seq: u64,
    pub items: Vec<CompletionItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionItem {
    pub label: String,
    pub detail: Option<String>,
    pub kind: Option<CompletionKind>,
    pub insert_text: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CompletionKind {
    Text,
    Method,
    Function,
    Constructor,
    Field,
    Variable,
    Class,
    Interface,
    Module,
    Property,
    Unit,
    Value,
    Enum,
    Keyword,
    Snippet,
    Color,
    File,
    Reference,
    Folder,
    EnumMember,
    Constant,
    Struct,
    Event,
    Operator,
    TypeParameter,
}

// ─── The master event enum ───────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum AppEvent {
    // Input
    KeyInput(KeyEvent),
    MouseInput(MouseEvent),
    FocusGained,
    FocusLost,

    // Terminal
    TerminalOutput(TerminalOutput),
    TerminalDirty(PaneId),
    TerminalExit { session_id: SessionId, exit_code: i32 },

    // Layout
    Resize(TermSize),
    LayoutChanged,
    PaneFocused(PaneId),

    // Editor
    BufferOpened(BufferId),
    BufferClosed(BufferId),
    BufferModified(BufferId),
    FileSaved(BufferId),

    // LSP
    LspDiagnostics { buffer_id: BufferId, count: usize },
    CompletionReady(CompletionReady),
    HoverReady { pane_id: PaneId, content: String },

    // AI
    AiStreamChunk(AiStreamChunk),
    AiRequestStarted { pane_id: PaneId, request_id: RequestId },
    AiRequestFinished { pane_id: PaneId, request_id: RequestId },
    AiError { pane_id: PaneId, message: String },

    // MCP
    McpToolCall { tool: String, session_id: SessionId },
    McpToolResult { session_id: SessionId, success: bool },

    // Git
    GitStatusChanged,

    // File system
    FileTreeChanged,

    // Config
    ConfigReloaded,

    // System
    Quit,

    // Editor mode
    ModeChanged(String),

    // Editor command line (confirmed :command string)
    EditorCmdLine(String),

    // Search pattern update
    SearchPatternSet { pattern: String, forward: bool },
}
