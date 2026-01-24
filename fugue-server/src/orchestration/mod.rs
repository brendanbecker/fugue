//! Orchestration - worktree-aware session coordination
//!
//! Provides detection and tracking of git worktrees for coordinating
//! parallel development workflows, plus messaging infrastructure for
//! cross-session communication.

mod router;
mod worktree;

#[allow(unused_imports)]
pub use router::{MessageReceiver, MessageRouter, MessageSender, RouterError};
#[allow(unused_imports)]
pub use worktree::{WorktreeDetector, WorktreeInfo};
