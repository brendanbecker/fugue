//! PTY management for ccmux server
//!
//! Provides pseudo-terminal creation and lifecycle management
//! using portable-pty for cross-platform compatibility.

// Allow unused items during development - will be used when integrated
#![allow(dead_code, unused_imports)]

mod buffer;
mod config;
mod handle;
mod manager;
mod output;

pub use buffer::{
    check_memory_status, check_memory_status_with_thresholds, format_memory_usage,
    global_scrollback_bytes, MemoryStatus, ScrollbackBuffer, DEFAULT_MEMORY_CRITICAL_BYTES,
    DEFAULT_MEMORY_WARNING_BYTES,
};
pub use config::PtyConfig;
pub use handle::PtyHandle;
pub use manager::PtyManager;
pub use output::{OutputPollerConfig, PaneClosedNotification, PollerHandle, PollerManager, PtyOutputPoller};
