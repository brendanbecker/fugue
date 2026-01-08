//! Session management for ccmux server
//!
//! Provides the session/window/pane hierarchy that organizes
//! terminal processes.

mod manager;
mod pane;
mod session;
mod window;

pub use manager::SessionManager;
pub use pane::Pane;
pub use session::Session;
pub use window::Window;
