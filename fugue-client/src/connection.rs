//! Client-server connection management
//!
//! Provides Unix socket connection to the fugue server with
//! automatic message framing and async dispatch.

mod client;
mod handler;

pub use client::Connection;

// These are part of the public API for advanced use cases
#[allow(unused_imports)]
pub use client::ConnectionState;
#[allow(unused_imports)]
pub use handler::{CallbackHandler, MessageHandler, MessageSender};
