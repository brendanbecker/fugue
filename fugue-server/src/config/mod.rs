//! Configuration management for fugue server
//!
//! Provides hot-reloading configuration with lock-free access
//! using ArcSwap for 60fps rendering compatibility.

// Allow unused items during development - will be used when integrated
#![allow(unused)]

mod defaults;
mod loader;
mod schema;
mod watcher;

pub use loader::ConfigLoader;
pub use schema::*;
pub use watcher::ConfigWatcher;

use arc_swap::ArcSwap;
use std::sync::Arc;

/// Global configuration handle
pub type ConfigHandle = Arc<ArcSwap<AppConfig>>;

/// Create a new config handle with defaults
pub fn new_config_handle() -> ConfigHandle {
    Arc::new(ArcSwap::from_pointee(AppConfig::default()))
}
