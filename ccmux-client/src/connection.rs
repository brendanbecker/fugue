//! Client-server connection management
//!
//! Provides Unix socket connection to the ccmux server with
//! automatic message framing and async dispatch.

mod client;
mod handler;

pub use client::{Connection, ConnectionState};
pub use handler::{CallbackHandler, MessageHandler, MessageSender};
