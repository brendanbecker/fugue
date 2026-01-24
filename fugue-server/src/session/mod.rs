//! Session management for fugue server
//!
//! Provides the session/window/pane hierarchy that organizes
//! terminal processes.

// Allow unused items during development - will be used when integrated
#![allow(dead_code, unused_imports)]
// Allow session/session.rs naming - matches the Session struct inside
#![allow(clippy::module_inception)]

mod manager;
mod mirror;
mod pane;
#[allow(clippy::module_inception)]
mod session;
mod window;

pub use manager::SessionManager;
pub use mirror::MirrorRegistry;
pub use pane::Pane;
pub use session::Session;
pub use window::Window;
