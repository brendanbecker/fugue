//! File watcher for configuration hot-reload

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use arc_swap::ArcSwap;
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use notify_debouncer_full::{new_debouncer, DebounceEventResult, Debouncer, FileIdMap};
use tokio::sync::mpsc;

use fugue_utils::{config_dir, CcmuxError, Result};

use super::{AppConfig, ConfigLoader};

/// Watches configuration files for changes
pub struct ConfigWatcher {
    /// Directory being watched
    config_dir: PathBuf,
    /// Channel receiver for events
    rx: mpsc::UnboundedReceiver<Result<Vec<Event>>>,
    /// Debouncer handle (kept alive)
    _debouncer: Debouncer<RecommendedWatcher, FileIdMap>,
}

impl ConfigWatcher {
    /// Create a new config watcher
    pub fn new() -> Result<Self> {
        let config_dir = config_dir();

        // Ensure directory exists
        if !config_dir.exists() {
            std::fs::create_dir_all(&config_dir).map_err(|e| CcmuxError::FileWrite {
                path: config_dir.clone(),
                source: e,
            })?;
        }

        let (tx, rx) = mpsc::unbounded_channel();

        // Create debounced watcher
        let mut debouncer = new_debouncer(
            Duration::from_millis(100),
            None,
            move |result: DebounceEventResult| {
                let events = result
                    .map(|events| events.into_iter().map(|e| e.event).collect())
                    .map_err(|errs| CcmuxError::config(format!("Watch error: {:?}", errs)));
                let _ = tx.send(events);
            },
        )
        .map_err(|e| CcmuxError::config(format!("Failed to create watcher: {}", e)))?;

        // Watch the config directory
        debouncer
            .watcher()
            .watch(&config_dir, RecursiveMode::NonRecursive)
            .map_err(|e| CcmuxError::config(format!("Failed to watch: {}", e)))?;

        Ok(Self {
            config_dir,
            rx,
            _debouncer: debouncer,
        })
    }

    /// Run the watcher loop, updating config on changes
    pub async fn run(mut self, config: Arc<ArcSwap<AppConfig>>) {
        tracing::info!("Config watcher started for {:?}", self.config_dir);

        while let Some(result) = self.rx.recv().await {
            match result {
                Ok(events) => {
                    for event in events {
                        if Self::is_config_change(&event) {
                            self.handle_change(&config).await;
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("Config watch error: {}", e);
                }
            }
        }
    }

    /// Check if an event is a config file change
    fn is_config_change(event: &Event) -> bool {
        matches!(
            event.kind,
            EventKind::Create(_) | EventKind::Modify(_)
        ) && event.paths.iter().any(|p| {
            p.file_name()
                .map(|n| n == "config.toml")
                .unwrap_or(false)
        })
    }

    /// Handle a configuration change
    async fn handle_change(&self, config: &Arc<ArcSwap<AppConfig>>) {
        tracing::info!("Config file changed, reloading...");

        match ConfigLoader::load_and_validate() {
            Ok(new_config) => {
                let old_config = config.load();

                // Log significant changes
                if old_config.general.prefix_key != new_config.general.prefix_key {
                    tracing::warn!("prefix_key changed - will apply after reattach");
                }

                // Atomically swap config
                config.store(Arc::new(new_config));
                tracing::info!("Configuration reloaded successfully");
            }
            Err(e) => {
                tracing::error!("Config reload failed (keeping previous): {}", e);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_config_change() {
        let event = Event {
            kind: EventKind::Modify(notify::event::ModifyKind::Data(
                notify::event::DataChange::Content,
            )),
            paths: vec![PathBuf::from("/home/user/.config/fugue/config.toml")],
            attrs: Default::default(),
        };

        assert!(ConfigWatcher::is_config_change(&event));
    }

    #[test]
    fn test_is_not_config_change() {
        let event = Event {
            kind: EventKind::Modify(notify::event::ModifyKind::Data(
                notify::event::DataChange::Content,
            )),
            paths: vec![PathBuf::from("/home/user/.config/fugue/other.toml")],
            attrs: Default::default(),
        };

        assert!(!ConfigWatcher::is_config_change(&event));
    }
}
