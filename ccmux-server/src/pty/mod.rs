//! PTY management for ccmux server
//!
//! Provides pseudo-terminal creation and lifecycle management
//! using portable-pty for cross-platform compatibility.

mod config;
mod handle;
mod manager;

pub use config::PtyConfig;
pub use handle::PtyHandle;
pub use manager::PtyManager;
