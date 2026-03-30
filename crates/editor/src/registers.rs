#![allow(dead_code)]
//! Vim-like named registers for yank/paste.
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RegisterKind {
    Named(char),   // 'a'-'z'
    Number(u8),    // 0-9
    Unnamed,       // "" (default)
    Clipboard,     // "+ system clipboard
    Primary,       // "* X11 primary selection
}

#[derive(Debug, Clone, Default)]
pub struct Register {
    pub text: String,
    /// True if yanked as whole lines (affects paste behaviour).
    pub is_line: bool,
}

pub struct Registers {
    map: HashMap<RegisterKind, Register>,
}

impl Registers {
    pub fn new() -> Self {
        Self { map: HashMap::new() }
    }

    pub fn get(&self, kind: &RegisterKind) -> Option<&Register> {
        self.map.get(kind)
    }

    pub fn set(&mut self, kind: RegisterKind, text: String, is_line: bool) {
        // Also always update the unnamed register.
        if kind != RegisterKind::Unnamed {
            self.map.insert(RegisterKind::Unnamed, Register { text: text.clone(), is_line });
        }
        self.map.insert(kind, Register { text, is_line });
    }

    pub fn get_unnamed(&self) -> Option<&Register> {
        self.map.get(&RegisterKind::Unnamed)
    }

    pub fn set_unnamed(&mut self, text: String, is_line: bool) {
        self.set(RegisterKind::Unnamed, text, is_line);
    }
}

impl Default for Registers {
    fn default() -> Self {
        Self::new()
    }
}
