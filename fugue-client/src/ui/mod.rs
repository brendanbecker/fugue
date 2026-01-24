//! UI components for ccmux client
//!
//! This module provides the Ratatui-based terminal user interface for managing
//! multiple Claude Code sessions.

mod app;
mod borders;
mod event;
mod layout;
mod pane;
mod render;
mod resize;
mod state;
mod status;
mod status_pane;
mod terminal;

pub use app::App;

// Border types are part of the public API
#[allow(unused_imports)]
pub use borders::{BorderConfig, BorderStyle, BorderTheme, TitleAlignment};

// Layout types are part of the public API for future features
#[allow(unused_imports)]
pub use layout::{LayoutManager, LayoutNode, LayoutPreset, SplitDirection};

// Pane types are part of the public API for future features
#[allow(unused_imports)]
pub use pane::{FocusState, Pane, PaneManager, PaneWidget};

// Resize handling types are part of the public API
#[allow(unused_imports)]
pub use resize::{MinimumSize, ResizeHandler};

// Status bar types are part of the public API
#[allow(unused_imports)]
pub use status::{ConnectionStatus, StatusBar, StatusBarWidget};
