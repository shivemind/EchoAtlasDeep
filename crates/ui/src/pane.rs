use core::ids::{BufferId, PaneId, SessionId};
use serde::{Deserialize, Serialize};

/// What kind of content a pane displays.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PaneKind {
    /// Embedded terminal emulator.
    Terminal { session_id: SessionId },
    /// Text editor (one buffer, one view).
    Editor { buffer_id: BufferId },
    /// File system tree.
    FileTree,
    /// AI chat conversation.
    AiChat,
    /// Git diff / status panel.
    GitPanel,
    /// Diagnostics list.
    Diagnostics,
    /// Built-in help browser.
    Help,
    /// Placeholder / empty pane.
    Empty,
    /// Fuzzy file picker overlay.
    FilePicker,
    /// Quickfix / diagnostics list.
    Quickfix,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pane {
    pub id: PaneId,
    pub kind: PaneKind,
}

impl Pane {
    pub fn new(id: PaneId, kind: PaneKind) -> Self {
        Self { id, kind }
    }
}
