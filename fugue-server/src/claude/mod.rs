//! Claude Code detection and state management
//!
//! This module provides functionality for detecting Claude Code running in terminal
//! panes and tracking its activity state (Idle, Thinking, Coding, ToolUse,
//! AwaitingConfirmation).
//!
//! # Architecture
//!
//! The detection works by analyzing PTY output patterns:
//!
//! 1. **Presence Detection**: Identifies Claude Code from startup messages,
//!    prompt patterns, or spinner indicators.
//!
//! 2. **State Detection**: Matches text patterns to activity states:
//!    - Idle: Prompt visible (`> ` or `❯ `)
//!    - Thinking: "Thinking", "Processing", etc.
//!    - Coding: "Writing", "Generating", etc.
//!    - ToolUse: "Running:", tool names like "Read(", etc.
//!    - AwaitingConfirmation: "[Y/n]", "Allow?", etc.
//!
//! 3. **Session Info**: Extracts session ID, model name when visible in output.
//!
//! # Usage
//!
//! ```rust,ignore
//! use fugue_server::claude::{ClaudeDetector, ClaudeStateChange};
//!
//! let mut detector = ClaudeDetector::new();
//!
//! // Process PTY output
//! if let Some(change) = detector.analyze("⠋ Thinking...") {
//!     println!("State changed: {}", change.description);
//! }
//!
//! // Check current state
//! if detector.is_claude() {
//!     println!("Activity: {:?}", detector.activity());
//! }
//! ```

mod command;
mod detector;
mod state;

pub use command::{create_resume_command, inject_session_id, is_claude_command};
pub use detector::ClaudeDetector;
// These types are part of the public API for external consumers
#[allow(unused_imports)]
pub use state::{ClaudeSessionInfo, ClaudeStateChange, DetectorConfig};

// Re-export protocol types for convenience
// Allow unused since these are part of the public API even if not used internally
#[allow(unused_imports)]
pub use fugue_protocol::{ClaudeActivity, ClaudeState};
