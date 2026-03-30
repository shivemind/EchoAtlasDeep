/// Typed ID newtypes to prevent mixing IDs across subsystems.
use serde::{Deserialize, Serialize};
use std::fmt;

macro_rules! define_id {
    ($name:ident) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
        pub struct $name(pub u32);

        impl $name {
            pub fn new(v: u32) -> Self {
                Self(v)
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}({})", stringify!($name), self.0)
            }
        }
    };
}

define_id!(PaneId);
define_id!(BufferId);
define_id!(SessionId);
define_id!(LanguageId);
define_id!(RequestId);

/// Global monotonic ID counter — each subsystem keeps its own instance.
#[derive(Debug, Default)]
pub struct IdGen(std::sync::atomic::AtomicU32);

impl IdGen {
    pub fn next_pane(&self) -> PaneId {
        PaneId(self.0.fetch_add(1, std::sync::atomic::Ordering::Relaxed))
    }
    pub fn next_buffer(&self) -> BufferId {
        BufferId(self.0.fetch_add(1, std::sync::atomic::Ordering::Relaxed))
    }
    pub fn next_session(&self) -> SessionId {
        SessionId(self.0.fetch_add(1, std::sync::atomic::Ordering::Relaxed))
    }
    pub fn next_request(&self) -> RequestId {
        RequestId(self.0.fetch_add(1, std::sync::atomic::Ordering::Relaxed))
    }
}
