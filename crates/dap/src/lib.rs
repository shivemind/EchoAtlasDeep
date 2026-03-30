#![allow(dead_code, unused_imports, unused_variables)]
//! `dap` — Debug Adapter Protocol client and types for EchoAtlasDeep.
//!
//! Phase 10 — Point 25.

pub mod protocol;
pub mod client;
pub mod breakpoints;

pub use client::{DapClient, DapStatus};
pub use breakpoints::{Breakpoint, BreakpointManager};
