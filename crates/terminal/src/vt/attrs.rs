/// SGR text attribute flags packed into a single byte.
use serde::{Deserialize, Serialize};

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
    pub struct Attrs: u16 {
        const BOLD          = 0b0000_0001;
        const DIM           = 0b0000_0010;
        const ITALIC        = 0b0000_0100;
        const UNDERLINE     = 0b0000_1000;
        const BLINK         = 0b0001_0000;
        const BLINK_RAPID   = 0b0010_0000;
        const REVERSE       = 0b0100_0000;
        const HIDDEN        = 0b1000_0000;
        const STRIKETHROUGH = 0b0001_0000_0000;
        const WIDE          = 0b0010_0000_0000; // double-width CJK
    }
}
