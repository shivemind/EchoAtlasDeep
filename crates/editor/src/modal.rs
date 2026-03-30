#![allow(dead_code)]
//! Vim-like modal editing engine.
use core::event::{KeyCode, KeyEvent, KeyModifiers};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Mode {
    Normal,
    Insert,
    Visual(VisualKind),
    OperatorPending(Op),
    Replace,
    SearchForward,
    SearchBackward,
    CommandLine,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VisualKind {
    Char,
    Line,
    Block,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Op {
    Delete,
    Change,
    Yank,
    Indent,
    Outdent,
}

/// The output of key processing — what the editor engine should do.
#[derive(Debug, Clone)]
pub enum EditorCommand {
    // Mode transitions
    EnterInsert,
    EnterInsertAppend,
    EnterInsertBOL,
    EnterInsertEOL,
    EnterInsertNewlineBelow,
    EnterInsertNewlineAbove,
    EnterNormal,
    EnterVisual(VisualKind),
    EnterReplace,
    EnterSearch(SearchDir),
    EnterCommandLine,

    // Cursor motion
    MoveLeft(usize),
    MoveRight(usize),
    MoveUp(usize),
    MoveDown(usize),
    MoveWordForward(usize),
    MoveWordBackward(usize),
    MoveWordEnd(usize),
    MoveLineStart,
    MoveLineFirstNonWs,
    MoveLineEnd,
    MoveFileStart,
    MoveFileEnd,
    MoveToLine(usize),
    ScrollHalfPageDown,
    ScrollHalfPageUp,

    // Buffer edits (Insert mode)
    InsertChar(char),
    InsertNewline,
    DeleteCharForward,
    DeleteCharBackward,
    DeleteWordBackward,
    DeleteToLineStart,
    ReplaceChar(char),

    // Normal mode operations
    DeleteLine(usize),
    DeleteMotion(Motion),
    DeleteSelection,
    ChangeLine(usize),
    ChangeMotion(Motion),
    ChangeSelection,
    YankLine(usize),
    YankMotion(Motion),
    YankSelection,
    PasteAfter,
    PasteBefore,

    // Undo/redo
    Undo,
    Redo,
    RepeatLast,

    // Search
    SearchNext,
    SearchPrev,
    SearchInput(char),
    SearchBackspace,
    SearchConfirm,
    SearchCancel,

    // Command line
    CmdInput(char),
    CmdBackspace,
    CmdConfirm,
    CmdCancel,
    CmdHistoryUp,
    CmdHistoryDown,

    // File ops (handled by main loop)
    SaveFile,
    OpenFile(String),
    Quit,
    ForceQuit,
    SplitH,
    SplitV,
    OpenFilePicker,

    // Folds
    ToggleFold,
    FoldAll,
    UnfoldAll,

    // Multi-cursor
    AddCursorNextMatch,

    // File picker
    FilePickerUp,
    FilePickerDown,
    FilePickerConfirm,
    FilePickerCancel,
    FilePickerInput(char),
    FilePickerBackspace,

    // LSP (Phase 3)
    LspHover,
    LspComplete,
    LspGotoDef,
    LspGotoRef,
    LspRename,
    LspCodeAction,
    LspFormat,
    LspDiagNext,
    LspDiagPrev,
    // Completion popup navigation
    CompletionSelectNext,
    CompletionSelectPrev,
    CompletionConfirm,
    CompletionCancel,

    // AI commands (Phase 4)
    AiChat,
    AiExplain,
    AiFix,
    AiTests,
    AiDocstring,
    AiRefactor,
    AiModelPicker,
    AiSend,
    AiChatInput(char),
    AiChatBackspace,
    AiGhostAccept,
    AiGhostAcceptWord,
    AiGhostDismiss,

    // ── Git commands ──────────────────────────────────────────────────────────
    GitPanel,
    GitBlame,
    GitBranchPanel,
    GitStageFile,
    GitUnstageFile,
    GitRefreshStatus,
    GitCommit(String),
    GitPanelUp,
    GitPanelDown,
    GitPanelNextSection,
    GitBranchCheckout,
    GitBranchCreate(String),
    GitBranchDelete,

    // Phase 8 — BYOK + Spend + Approvals:
    KeyVaultOpen,
    SpendPanelOpen,
    ModelMatrixOpen,
    ToggleOffline,
    ApprovalApprove,
    ApprovalDeny,
    ApprovalApproveAll,
    ApprovalDenyAll,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Motion {
    Word,
    WordBack,
    WordEnd,
    Line,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SearchDir {
    Forward,
    Backward,
}

pub struct ModalState {
    pub mode: Mode,
    count: String,
    pending_g: bool,
    pending_z: bool,
    pending_leader: bool,
    pending_leader_g: bool,
    pub last_change: Vec<EditorCommand>,
}

impl ModalState {
    pub fn new() -> Self {
        Self {
            mode: Mode::Normal,
            count: String::new(),
            pending_g: false,
            pending_z: false,
            pending_leader: false,
            pending_leader_g: false,
            last_change: Vec::new(),
        }
    }

    pub fn mode_str(&self) -> &str {
        match &self.mode {
            Mode::Normal => "NORMAL",
            Mode::Insert => "INSERT",
            Mode::Visual(VisualKind::Char) => "VISUAL",
            Mode::Visual(VisualKind::Line) => "V-LINE",
            Mode::Visual(VisualKind::Block) => "V-BLOCK",
            Mode::OperatorPending(_) => "PENDING",
            Mode::Replace => "REPLACE",
            Mode::SearchForward => "SEARCH",
            Mode::SearchBackward => "SEARCH",
            Mode::CommandLine => "COMMAND",
        }
    }

    fn take_count(&mut self) -> usize {
        if self.count.is_empty() {
            1
        } else {
            let n = self.count.parse().unwrap_or(1);
            self.count.clear();
            n
        }
    }

    /// Main key dispatch. Returns list of commands to execute.
    pub fn handle_key(&mut self, key: &KeyEvent) -> Vec<EditorCommand> {
        match self.mode.clone() {
            Mode::Normal => self.handle_normal(key),
            Mode::Insert => self.handle_insert(key),
            Mode::Visual(_) => self.handle_visual(key),
            Mode::OperatorPending(op) => self.handle_operator_pending(op, key),
            Mode::Replace => self.handle_replace(key),
            Mode::SearchForward | Mode::SearchBackward => self.handle_search(key),
            Mode::CommandLine => self.handle_cmdline(key),
        }
    }

    fn handle_normal(&mut self, key: &KeyEvent) -> Vec<EditorCommand> {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);

        // Handle <leader>g<x> two-key sequences
        if self.pending_leader_g {
            self.pending_leader_g = false;
            return match &key.code {
                KeyCode::Char('b') if !ctrl => vec![EditorCommand::GitBlame],
                KeyCode::Char('p') if !ctrl => vec![EditorCommand::GitPanel],
                KeyCode::Char('s') if !ctrl => vec![EditorCommand::GitRefreshStatus],
                KeyCode::Char('B') if !ctrl => vec![EditorCommand::GitBranchPanel],
                _ => vec![],
            };
        }

        // Handle <leader> (\) prefix sequences
        if self.pending_leader {
            self.pending_leader = false;
            return match &key.code {
                KeyCode::Char('a') if !ctrl => {
                    // \a → AI chat
                    vec![EditorCommand::AiChat]
                }
                KeyCode::Char('e') if !ctrl => vec![EditorCommand::AiExplain],
                KeyCode::Char('f') if !ctrl => vec![EditorCommand::AiFix],
                KeyCode::Char('t') if !ctrl => vec![EditorCommand::AiTests],
                KeyCode::Char('d') if !ctrl => vec![EditorCommand::AiDocstring],
                KeyCode::Char('r') if !ctrl => vec![EditorCommand::AiRefactor],
                KeyCode::Char('m') if !ctrl => vec![EditorCommand::AiModelPicker],
                KeyCode::Char('g') if !ctrl => {
                    // \g → enter leader-g sub-mode for git commands
                    self.pending_leader_g = true;
                    vec![]
                }
                KeyCode::Char('K') if !ctrl => vec![EditorCommand::KeyVaultOpen],
                KeyCode::Char('$') if !ctrl => vec![EditorCommand::SpendPanelOpen],
                KeyCode::Char('M') if !ctrl => vec![EditorCommand::ModelMatrixOpen],
                KeyCode::Char('o') if !ctrl => vec![EditorCommand::ToggleOffline],
                _ => vec![],
            };
        }

        // Handle 'g' prefix sequences
        if self.pending_g {
            self.pending_g = false;
            return match &key.code {
                KeyCode::Char('g') => vec![EditorCommand::MoveFileStart],
                KeyCode::Char('e') => vec![EditorCommand::MoveWordEnd(self.take_count())],
                KeyCode::Char('d') => vec![EditorCommand::LspGotoDef],
                KeyCode::Char('r') => vec![EditorCommand::LspGotoRef],
                _ => vec![],
            };
        }

        // Handle 'z' prefix sequences
        if self.pending_z {
            self.pending_z = false;
            return match &key.code {
                KeyCode::Char('a') => vec![EditorCommand::ToggleFold],
                KeyCode::Char('R') => vec![EditorCommand::UnfoldAll],
                KeyCode::Char('M') => vec![EditorCommand::FoldAll],
                _ => vec![],
            };
        }

        match &key.code {
            // Count accumulation
            KeyCode::Char(c) if (*c >= '1' && *c <= '9') && !ctrl => {
                self.count.push(*c);
                vec![]
            }
            KeyCode::Char('0') if !self.count.is_empty() && !ctrl => {
                self.count.push('0');
                vec![]
            }

            // Enter Insert mode variants
            KeyCode::Char('i') if !ctrl => {
                self.mode = Mode::Insert;
                vec![EditorCommand::EnterInsert]
            }
            KeyCode::Char('a') if !ctrl => {
                self.mode = Mode::Insert;
                vec![EditorCommand::EnterInsertAppend]
            }
            KeyCode::Char('I') if !ctrl => {
                self.mode = Mode::Insert;
                vec![EditorCommand::EnterInsertBOL]
            }
            KeyCode::Char('A') if !ctrl => {
                self.mode = Mode::Insert;
                vec![EditorCommand::EnterInsertEOL]
            }
            KeyCode::Char('o') if !ctrl => {
                self.mode = Mode::Insert;
                vec![EditorCommand::EnterInsertNewlineBelow]
            }
            KeyCode::Char('O') if !ctrl => {
                self.mode = Mode::Insert;
                vec![EditorCommand::EnterInsertNewlineAbove]
            }
            KeyCode::Char('R') if !ctrl => {
                self.mode = Mode::Replace;
                vec![EditorCommand::EnterReplace]
            }

            // Motion keys
            KeyCode::Char('h') | KeyCode::Left if !ctrl => {
                vec![EditorCommand::MoveLeft(self.take_count())]
            }
            KeyCode::Char('l') | KeyCode::Right if !ctrl => {
                vec![EditorCommand::MoveRight(self.take_count())]
            }
            KeyCode::Char('j') | KeyCode::Down if !ctrl => {
                vec![EditorCommand::MoveDown(self.take_count())]
            }
            KeyCode::Char('k') | KeyCode::Up if !ctrl => {
                vec![EditorCommand::MoveUp(self.take_count())]
            }
            KeyCode::Char('w') if !ctrl => {
                vec![EditorCommand::MoveWordForward(self.take_count())]
            }
            KeyCode::Char('b') if !ctrl => {
                vec![EditorCommand::MoveWordBackward(self.take_count())]
            }
            KeyCode::Char('e') if !ctrl => {
                vec![EditorCommand::MoveWordEnd(self.take_count())]
            }
            KeyCode::Char('0') if !ctrl => vec![EditorCommand::MoveLineStart],
            KeyCode::Char('^') if !ctrl => vec![EditorCommand::MoveLineFirstNonWs],
            KeyCode::Char('$') if !ctrl => vec![EditorCommand::MoveLineEnd],
            KeyCode::Char('G') if !ctrl => {
                let n = self.take_count();
                if n == 1 {
                    vec![EditorCommand::MoveFileEnd]
                } else {
                    vec![EditorCommand::MoveToLine(n - 1)]
                }
            }
            KeyCode::Char('g') if !ctrl => {
                self.pending_g = true;
                self.take_count();
                vec![]
            }
            KeyCode::Char('d') if ctrl => vec![EditorCommand::ScrollHalfPageDown],
            KeyCode::Char('u') if ctrl => vec![EditorCommand::ScrollHalfPageUp],

            // Operators
            KeyCode::Char('d') if !ctrl => {
                self.mode = Mode::OperatorPending(Op::Delete);
                vec![]
            }
            KeyCode::Char('c') if !ctrl => {
                self.mode = Mode::OperatorPending(Op::Change);
                vec![]
            }
            KeyCode::Char('y') if !ctrl => {
                self.mode = Mode::OperatorPending(Op::Yank);
                vec![]
            }

            // Quick deletes
            KeyCode::Char('x') if !ctrl => {
                vec![EditorCommand::DeleteCharForward]
            }
            KeyCode::Char('X') if !ctrl => {
                vec![EditorCommand::DeleteCharBackward]
            }

            // Paste
            KeyCode::Char('p') if !ctrl => vec![EditorCommand::PasteAfter],
            KeyCode::Char('P') if !ctrl => vec![EditorCommand::PasteBefore],

            // Undo/redo
            KeyCode::Char('u') if !ctrl => vec![EditorCommand::Undo],
            KeyCode::Char('r') if ctrl => vec![EditorCommand::Redo],
            KeyCode::Char('.') if !ctrl => vec![EditorCommand::RepeatLast],

            // Visual mode
            KeyCode::Char('v') if !ctrl => {
                self.mode = Mode::Visual(VisualKind::Char);
                vec![EditorCommand::EnterVisual(VisualKind::Char)]
            }
            KeyCode::Char('V') if !ctrl => {
                self.mode = Mode::Visual(VisualKind::Line);
                vec![EditorCommand::EnterVisual(VisualKind::Line)]
            }

            // Search
            KeyCode::Char('/') if !ctrl => {
                self.mode = Mode::SearchForward;
                vec![EditorCommand::EnterSearch(SearchDir::Forward)]
            }
            KeyCode::Char('?') if !ctrl => {
                self.mode = Mode::SearchBackward;
                vec![EditorCommand::EnterSearch(SearchDir::Backward)]
            }
            KeyCode::Char('n') if !ctrl => vec![EditorCommand::SearchNext],
            KeyCode::Char('N') if !ctrl => vec![EditorCommand::SearchPrev],

            // Command line
            KeyCode::Char(':') if !ctrl => {
                self.mode = Mode::CommandLine;
                vec![EditorCommand::EnterCommandLine]
            }

            // Fold
            KeyCode::Char('z') if !ctrl => {
                self.pending_z = true;
                vec![]
            }

            // Multi-cursor (Ctrl-D)
            // Note: Ctrl-D is also ScrollHalfPageDown in some modes but Ctrl-D for multi-cursor is common in VSCode style
            // We already mapped Ctrl-D above to ScrollHalfPageDown. Use a different check here.

            // File picker (Ctrl-P)
            KeyCode::Char('p') if ctrl => vec![EditorCommand::OpenFilePicker],

            // Leader key (\)
            KeyCode::Char('\\') if !ctrl => {
                self.pending_leader = true;
                vec![]
            }

            // Ghost text accept (Tab in normal mode)
            KeyCode::Tab => vec![EditorCommand::AiGhostAccept],

            // LSP keybindings
            KeyCode::Char('K') if !ctrl => vec![EditorCommand::LspHover],
            KeyCode::Char(' ') if ctrl => vec![EditorCommand::LspComplete],

            // Diagnostic navigation (]d / [d implemented via 'g' sequences is complex;
            // use simpler direct bindings for now)

            _ => {
                self.count.clear();
                vec![]
            }
        }
    }

    fn handle_insert(&mut self, key: &KeyEvent) -> Vec<EditorCommand> {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        match &key.code {
            KeyCode::Escape => {
                self.mode = Mode::Normal;
                vec![EditorCommand::AiGhostDismiss, EditorCommand::EnterNormal]
            }
            // Ctrl-Enter sends AI chat message
            KeyCode::Enter if ctrl => vec![EditorCommand::AiSend],
            KeyCode::Enter => vec![EditorCommand::InsertNewline],
            KeyCode::Backspace => vec![EditorCommand::DeleteCharBackward],
            KeyCode::Delete => vec![EditorCommand::DeleteCharForward],
            KeyCode::Char('h') if ctrl => vec![EditorCommand::DeleteCharBackward],
            KeyCode::Char('w') if ctrl => vec![EditorCommand::DeleteWordBackward],
            KeyCode::Char('u') if ctrl => vec![EditorCommand::DeleteToLineStart],
            KeyCode::Char(' ') if ctrl => vec![EditorCommand::LspComplete],
            // Tab: accept ghost text if present, otherwise confirm completion
            KeyCode::Tab => vec![EditorCommand::AiGhostAccept],
            KeyCode::Right if ctrl => vec![EditorCommand::AiGhostAcceptWord],
            KeyCode::Left => vec![EditorCommand::MoveLeft(1)],
            KeyCode::Right => vec![EditorCommand::MoveRight(1)],
            KeyCode::Up => vec![EditorCommand::MoveUp(1)],
            KeyCode::Down => vec![EditorCommand::MoveDown(1)],
            KeyCode::Char(c) if !ctrl => vec![EditorCommand::InsertChar(*c)],
            _ => vec![],
        }
    }

    fn handle_visual(&mut self, key: &KeyEvent) -> Vec<EditorCommand> {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        match &key.code {
            KeyCode::Escape => {
                self.mode = Mode::Normal;
                vec![EditorCommand::EnterNormal]
            }
            KeyCode::Char('d') | KeyCode::Char('x') if !ctrl => {
                self.mode = Mode::Normal;
                vec![EditorCommand::DeleteSelection]
            }
            KeyCode::Char('c') if !ctrl => {
                self.mode = Mode::Insert;
                vec![EditorCommand::ChangeSelection]
            }
            KeyCode::Char('y') if !ctrl => {
                self.mode = Mode::Normal;
                vec![EditorCommand::YankSelection]
            }
            // Motions extend selection
            KeyCode::Char('h') | KeyCode::Left => {
                vec![EditorCommand::MoveLeft(self.take_count())]
            }
            KeyCode::Char('l') | KeyCode::Right => {
                vec![EditorCommand::MoveRight(self.take_count())]
            }
            KeyCode::Char('j') | KeyCode::Down => {
                vec![EditorCommand::MoveDown(self.take_count())]
            }
            KeyCode::Char('k') | KeyCode::Up => {
                vec![EditorCommand::MoveUp(self.take_count())]
            }
            KeyCode::Char('w') if !ctrl => {
                vec![EditorCommand::MoveWordForward(self.take_count())]
            }
            KeyCode::Char('b') if !ctrl => {
                vec![EditorCommand::MoveWordBackward(self.take_count())]
            }
            KeyCode::Char('$') if !ctrl => vec![EditorCommand::MoveLineEnd],
            KeyCode::Char('0') if !ctrl => vec![EditorCommand::MoveLineStart],
            KeyCode::Char('G') if !ctrl => vec![EditorCommand::MoveFileEnd],
            _ => vec![],
        }
    }

    fn handle_operator_pending(&mut self, op: Op, key: &KeyEvent) -> Vec<EditorCommand> {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        self.mode = Mode::Normal; // reset after operator
        match &key.code {
            KeyCode::Escape => vec![EditorCommand::EnterNormal],
            KeyCode::Char('d') if matches!(op, Op::Delete) && !ctrl => {
                let n = self.take_count();
                vec![EditorCommand::DeleteLine(n)]
            }
            KeyCode::Char('c') if matches!(op, Op::Change) && !ctrl => {
                let n = self.take_count();
                vec![EditorCommand::ChangeLine(n)]
            }
            KeyCode::Char('y') if matches!(op, Op::Yank) && !ctrl => {
                let n = self.take_count();
                vec![EditorCommand::YankLine(n)]
            }
            KeyCode::Char('w') if !ctrl => {
                match op {
                    Op::Delete => vec![EditorCommand::DeleteMotion(Motion::Word)],
                    Op::Change => vec![EditorCommand::ChangeMotion(Motion::Word)],
                    Op::Yank => vec![EditorCommand::YankMotion(Motion::Word)],
                    _ => vec![],
                }
            }
            KeyCode::Char('b') if !ctrl => {
                match op {
                    Op::Delete => vec![EditorCommand::DeleteMotion(Motion::WordBack)],
                    Op::Change => vec![EditorCommand::ChangeMotion(Motion::WordBack)],
                    Op::Yank => vec![EditorCommand::YankMotion(Motion::WordBack)],
                    _ => vec![],
                }
            }
            _ => vec![],
        }
    }

    fn handle_replace(&mut self, key: &KeyEvent) -> Vec<EditorCommand> {
        match &key.code {
            KeyCode::Escape => {
                self.mode = Mode::Normal;
                vec![EditorCommand::EnterNormal]
            }
            KeyCode::Char(c) => vec![EditorCommand::ReplaceChar(*c)],
            _ => vec![],
        }
    }

    fn handle_search(&mut self, key: &KeyEvent) -> Vec<EditorCommand> {
        match &key.code {
            KeyCode::Escape => {
                self.mode = Mode::Normal;
                vec![EditorCommand::SearchCancel]
            }
            KeyCode::Enter => {
                self.mode = Mode::Normal;
                vec![EditorCommand::SearchConfirm]
            }
            KeyCode::Backspace => vec![EditorCommand::SearchBackspace],
            KeyCode::Char(c) => vec![EditorCommand::SearchInput(*c)],
            _ => vec![],
        }
    }

    fn handle_cmdline(&mut self, key: &KeyEvent) -> Vec<EditorCommand> {
        match &key.code {
            KeyCode::Escape => {
                self.mode = Mode::Normal;
                vec![EditorCommand::CmdCancel]
            }
            KeyCode::Enter => {
                self.mode = Mode::Normal;
                vec![EditorCommand::CmdConfirm]
            }
            KeyCode::Backspace => vec![EditorCommand::CmdBackspace],
            KeyCode::Up => vec![EditorCommand::CmdHistoryUp],
            KeyCode::Down => vec![EditorCommand::CmdHistoryDown],
            KeyCode::Char(c) => vec![EditorCommand::CmdInput(*c)],
            _ => vec![],
        }
    }
}

impl Default for ModalState {
    fn default() -> Self {
        Self::new()
    }
}
