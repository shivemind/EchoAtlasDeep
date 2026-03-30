/// Thin wrapper around the `vte` crate.
/// Converts raw bytes into VtEvent variants consumed by the VtProcessor.
use vte::{Params, Parser as VteParser, Perform};

/// Events emitted by the VT parser — fed into VtProcessor.
#[derive(Debug, Clone)]
pub enum VtEvent {
    /// Printable character (may be multi-codepoint grapheme cluster later).
    Print(char),
    /// C0/C1 control character (e.g. LF = 0x0A, CR = 0x0D, BEL = 0x07).
    Execute(u8),
    /// CSI sequence: ESC [ params intermediate final_byte.
    CsiDispatch {
        params: Vec<Vec<u16>>,
        intermediates: Vec<u8>,
        ignore: bool,
        action: char,
    },
    /// OSC sequence: ESC ] params ST.
    OscDispatch { params: Vec<Vec<u8>>, bell_terminated: bool },
    /// ESC sequence (non-CSI, non-OSC).
    EscDispatch { intermediates: Vec<u8>, ignore: bool, byte: u8 },
    /// DCS hook start.
    Hook { params: Vec<Vec<u16>>, intermediates: Vec<u8>, ignore: bool, action: char },
    /// DCS data byte.
    Put(u8),
    /// DCS hook end.
    Unhook,
}

// ─── Collector — bridges vte Perform trait to our VtEvent vec ────────────────

pub(crate) struct EventCollector(pub Vec<VtEvent>);

impl Perform for EventCollector {
    fn print(&mut self, c: char) {
        self.0.push(VtEvent::Print(c));
    }

    fn execute(&mut self, byte: u8) {
        self.0.push(VtEvent::Execute(byte));
    }

    fn csi_dispatch(&mut self, params: &Params, intermediates: &[u8], ignore: bool, action: char) {
        let params_vec: Vec<Vec<u16>> = params
            .iter()
            .map(|subparam| subparam.iter().copied().collect())
            .collect();
        self.0.push(VtEvent::CsiDispatch {
            params: params_vec,
            intermediates: intermediates.to_vec(),
            ignore,
            action,
        });
    }

    fn osc_dispatch(&mut self, params: &[&[u8]], bell_terminated: bool) {
        let params_vec: Vec<Vec<u8>> = params.iter().map(|p| p.to_vec()).collect();
        self.0.push(VtEvent::OscDispatch { params: params_vec, bell_terminated });
    }

    fn esc_dispatch(&mut self, intermediates: &[u8], ignore: bool, byte: u8) {
        self.0.push(VtEvent::EscDispatch {
            intermediates: intermediates.to_vec(),
            ignore,
            byte,
        });
    }

    fn hook(&mut self, params: &Params, intermediates: &[u8], ignore: bool, action: char) {
        let params_vec: Vec<Vec<u16>> = params
            .iter()
            .map(|s| s.iter().copied().collect())
            .collect();
        self.0.push(VtEvent::Hook {
            params: params_vec,
            intermediates: intermediates.to_vec(),
            ignore,
            action,
        });
    }

    fn put(&mut self, byte: u8) {
        self.0.push(VtEvent::Put(byte));
    }

    fn unhook(&mut self) {
        self.0.push(VtEvent::Unhook);
    }
}

// ─── Public parser handle ─────────────────────────────────────────────────────

pub struct VtParser {
    inner: VteParser,
}

impl VtParser {
    pub fn new() -> Self {
        Self { inner: VteParser::new() }
    }

    /// Feed raw bytes into the parser; returns the resulting events.
    pub fn advance(&mut self, bytes: &[u8]) -> Vec<VtEvent> {
        let mut collector = EventCollector(Vec::new());
        for &byte in bytes {
            self.inner.advance(&mut collector, byte);
        }
        collector.0
    }
}

impl Default for VtParser {
    fn default() -> Self {
        Self::new()
    }
}
