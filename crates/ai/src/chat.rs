#![allow(dead_code, unused_imports, unused_variables)]
use crate::backend::{Message, Role};

/// A single rendered line in the chat pane.
#[derive(Debug, Clone)]
pub struct ChatLine {
    pub kind: ChatLineKind,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChatLineKind {
    UserMsg,
    AssistantMsg,
    AssistantStreaming,
    SystemInfo,
    CodeBlock,
    CodeBlockLang,
    Error,
}

/// Chat session: stores history + manages streaming state.
pub struct ChatSession {
    pub history: Vec<Message>,
    pub display_lines: Vec<ChatLine>,
    pub scroll_offset: usize,
    pub input: String,
    pub input_cursor: usize,
    pub streaming: bool,
    pub streaming_text: String,
    pub backend_display: String,
    pub last_input_tokens: u32,
    pub last_output_tokens: u32,
}

impl ChatSession {
    pub fn new(backend_display: String) -> Self {
        let welcome = ChatLine {
            kind: ChatLineKind::SystemInfo,
            text: format!(
                "AI Chat — {backend_display} | Ctrl-Enter to send, Escape to focus editor"
            ),
        };
        Self {
            history: Vec::new(),
            display_lines: vec![welcome],
            scroll_offset: 0,
            input: String::new(),
            input_cursor: 0,
            streaming: false,
            streaming_text: String::new(),
            backend_display,
            last_input_tokens: 0,
            last_output_tokens: 0,
        }
    }

    pub fn push_user_message(&mut self, text: String) {
        self.history.push(Message {
            role: Role::User,
            content: text.clone(),
        });
        self.display_lines.push(ChatLine {
            kind: ChatLineKind::SystemInfo,
            text: "─".repeat(40),
        });
        self.display_lines.push(ChatLine {
            kind: ChatLineKind::UserMsg,
            text: format!("You: {text}"),
        });
        self.display_lines.push(ChatLine {
            kind: ChatLineKind::AssistantStreaming,
            text: "Assistant: ".to_string(),
        });
        self.streaming = true;
        self.streaming_text.clear();
        self.scroll_to_bottom();
    }

    /// Feed a streaming chunk to the display.
    pub fn push_chunk(&mut self, text: &str) {
        self.streaming_text.push_str(text);
        if let Some(last) = self.display_lines.last_mut() {
            if last.kind == ChatLineKind::AssistantStreaming {
                last.text = format!("Assistant: {}", self.streaming_text);
            }
        }
    }

    /// Finalize the streaming response.
    pub fn finish_streaming(&mut self, input_tokens: u32, output_tokens: u32) {
        self.streaming = false;
        self.last_input_tokens = input_tokens;
        self.last_output_tokens = output_tokens;

        // Convert last streaming line to final
        if let Some(last) = self.display_lines.last_mut() {
            last.kind = ChatLineKind::AssistantMsg;
        }

        // Add the full response to history
        let text = self.streaming_text.clone();
        self.history.push(Message {
            role: Role::Assistant,
            content: text,
        });

        // Token usage info
        if input_tokens > 0 || output_tokens > 0 {
            self.display_lines.push(ChatLine {
                kind: ChatLineKind::SystemInfo,
                text: format!("  ↳ {input_tokens} in / {output_tokens} out tokens"),
            });
        }
        self.scroll_to_bottom();
    }

    pub fn push_error(&mut self, err: &str) {
        self.streaming = false;
        self.display_lines.push(ChatLine {
            kind: ChatLineKind::Error,
            text: format!("Error: {err}"),
        });
        self.scroll_to_bottom();
    }

    pub fn input_push(&mut self, c: char) {
        self.input.insert(self.input_cursor, c);
        self.input_cursor += c.len_utf8();
    }

    pub fn input_backspace(&mut self) {
        if self.input_cursor > 0 {
            let c = self.input.remove(self.input_cursor - 1);
            self.input_cursor -= c.len_utf8();
        }
    }

    pub fn input_confirm(&mut self) -> Option<String> {
        let text = self.input.trim().to_string();
        if text.is_empty() {
            return None;
        }
        self.input.clear();
        self.input_cursor = 0;
        Some(text)
    }

    pub fn scroll_up(&mut self, n: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(n);
    }

    pub fn scroll_down(&mut self, n: usize) {
        self.scroll_offset += n;
        self.scroll_to_bottom();
    }

    pub fn scroll_to_bottom(&mut self) {
        self.scroll_offset = usize::MAX;
    }
}
