//! Session isolation for Claude Code instances
//!
//! Provides per-pane isolation for Claude Code by setting unique `CLAUDE_CONFIG_DIR`
//! environment variables. This prevents multiple concurrent Claude instances from
//! corrupting each other's session state.
//!
//! Claude writes to `~/.claude/.claude.json` at ~1.5 writes/second during active use.
//! Multiple Claude instances cause conflicts when they share the same config directory.
//!
//! Each Claude pane gets its own config directory:
//! ```text
//! ~/.local/state/ccmux/claude-configs/
//! ├── pane-<uuid1>/
//! │   └── .claude.json
//! ├── pane-<uuid2>/
//! │   └── .claude.json
//! └── ...
//! ```

use std::path::PathBuf;

use tracing::{debug, info, warn};
use uuid::Uuid;

/// Environment variable name for Claude config directory
pub const CLAUDE_CONFIG_DIR_ENV: &str = "CLAUDE_CONFIG_DIR";

/// Environment variable for tracking pane ID in Claude processes
pub const CCMUX_PANE_ID_ENV: &str = "CCMUX_PANE_ID";

/// Subdirectory name for isolation configs under state_dir
const ISOLATION_DIR_NAME: &str = "claude-configs";

/// Prefix for pane isolation directories
const PANE_DIR_PREFIX: &str = "pane-";

/// Get the base directory for all isolation config directories
fn isolation_base_dir() -> PathBuf {
    ccmux_utils::state_dir().join(ISOLATION_DIR_NAME)
}

/// Get the isolation config directory path for a specific pane
///
/// Returns the path where Claude's config files will be stored for this pane.
/// Does not create the directory - use `ensure_config_dir` for that.
pub fn pane_config_dir(pane_id: Uuid) -> PathBuf {
    isolation_base_dir().join(format!("{}{}", PANE_DIR_PREFIX, pane_id))
}

/// Ensure the isolation config directory exists for a pane
///
/// Creates the directory if it doesn't exist. Returns the path to the directory.
///
/// # Errors
///
/// Returns an error if the directory cannot be created.
pub fn ensure_config_dir(pane_id: Uuid) -> std::io::Result<PathBuf> {
    let dir = pane_config_dir(pane_id);
    if !dir.exists() {
        debug!("Creating isolation directory: {}", dir.display());
        std::fs::create_dir_all(&dir)?;
    }
    Ok(dir)
}

/// Clean up the isolation config directory for a pane
///
/// Removes the directory and all its contents. Safe to call even if the
/// directory doesn't exist.
///
/// # Errors
///
/// Returns an error if the directory exists but cannot be removed.
pub fn cleanup_config_dir(pane_id: Uuid) -> std::io::Result<()> {
    let dir = pane_config_dir(pane_id);
    if dir.exists() {
        debug!("Cleaning up isolation directory: {}", dir.display());
        std::fs::remove_dir_all(&dir)?;
    }
    Ok(())
}

/// List all existing isolation config directories
///
/// Returns a list of (pane_id, path) tuples for all valid isolation directories.
/// Invalid directories (those that don't match the expected format) are skipped.
pub fn list_config_dirs() -> std::io::Result<Vec<(Uuid, PathBuf)>> {
    let base = isolation_base_dir();
    if !base.exists() {
        return Ok(Vec::new());
    }

    let mut dirs = Vec::new();
    for entry in std::fs::read_dir(&base)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }

        let name = entry.file_name();
        let name_str = name.to_string_lossy();

        if let Some(id_str) = name_str.strip_prefix(PANE_DIR_PREFIX) {
            if let Ok(id) = Uuid::parse_str(id_str) {
                dirs.push((id, entry.path()));
            }
        }
    }
    Ok(dirs)
}

/// Clean up orphaned config directories
///
/// Removes isolation directories for panes that no longer exist.
/// Returns the number of directories cleaned up.
///
/// # Arguments
///
/// * `active_panes` - List of currently active pane IDs
pub fn cleanup_orphaned(active_panes: &[Uuid]) -> std::io::Result<usize> {
    let dirs = list_config_dirs()?;
    let mut cleaned = 0;

    for (id, path) in dirs {
        if !active_panes.contains(&id) {
            info!("Cleaning orphaned config dir: {}", path.display());
            if let Err(e) = std::fs::remove_dir_all(&path) {
                warn!("Failed to remove orphaned dir {}: {}", path.display(), e);
            } else {
                cleaned += 1;
            }
        }
    }

    Ok(cleaned)
}

/// Perform startup cleanup of isolation directories
///
/// Called on server startup to clean up directories left over from crashed
/// sessions. Uses the list of active panes from the session manager.
///
/// # Arguments
///
/// * `active_pane_ids` - List of pane IDs that are currently active
pub fn startup_cleanup(active_pane_ids: &[Uuid]) -> std::io::Result<()> {
    let cleaned = cleanup_orphaned(active_pane_ids)?;
    if cleaned > 0 {
        info!("Cleaned {} orphaned isolation directories", cleaned);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::fs::File;
    use std::sync::Mutex;

    /// Mutex to serialize tests that modify environment variables
    static ENV_MUTEX: Mutex<()> = Mutex::new(());

    /// Helper to run tests in a temporary state directory
    /// Uses a mutex to prevent parallel tests from interfering with each other
    fn with_temp_state_dir<F, R>(f: F) -> R
    where
        F: FnOnce() -> R,
    {
        let _guard = ENV_MUTEX.lock().unwrap();

        let temp_dir = env::temp_dir().join(format!("ccmux_test_{}", Uuid::new_v4()));
        let original_state = env::var("XDG_STATE_HOME").ok();

        // Set temporary state dir
        env::set_var("XDG_STATE_HOME", &temp_dir);

        let result = f();

        // Restore original state dir
        match original_state {
            Some(val) => env::set_var("XDG_STATE_HOME", val),
            None => env::remove_var("XDG_STATE_HOME"),
        }

        // Clean up temp dir
        if temp_dir.exists() {
            let _ = std::fs::remove_dir_all(&temp_dir);
        }

        result
    }

    // ==================== pane_config_dir Tests ====================

    #[test]
    fn test_pane_config_dir_format() {
        let pane_id = Uuid::parse_str("12345678-1234-1234-1234-123456789abc").unwrap();
        let dir = pane_config_dir(pane_id);
        let dir_str = dir.to_string_lossy();

        assert!(dir_str.contains("claude-configs"));
        assert!(dir_str.contains("pane-12345678-1234-1234-1234-123456789abc"));
    }

    #[test]
    fn test_pane_config_dir_unique() {
        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();

        let dir1 = pane_config_dir(id1);
        let dir2 = pane_config_dir(id2);

        assert_ne!(dir1, dir2);
    }

    #[test]
    fn test_pane_config_dir_consistent() {
        let pane_id = Uuid::new_v4();

        let dir1 = pane_config_dir(pane_id);
        let dir2 = pane_config_dir(pane_id);

        assert_eq!(dir1, dir2);
    }

    // ==================== ensure_config_dir Tests ====================

    #[test]
    fn test_ensure_config_dir_creates_directory() {
        with_temp_state_dir(|| {
            let pane_id = Uuid::new_v4();
            let dir = ensure_config_dir(pane_id).unwrap();

            assert!(dir.exists());
            assert!(dir.is_dir());

            // Clean up
            cleanup_config_dir(pane_id).unwrap();
        });
    }

    #[test]
    fn test_ensure_config_dir_idempotent() {
        with_temp_state_dir(|| {
            let pane_id = Uuid::new_v4();

            let dir1 = ensure_config_dir(pane_id).unwrap();
            let dir2 = ensure_config_dir(pane_id).unwrap();

            assert_eq!(dir1, dir2);
            assert!(dir1.exists());

            cleanup_config_dir(pane_id).unwrap();
        });
    }

    #[test]
    fn test_ensure_config_dir_returns_correct_path() {
        with_temp_state_dir(|| {
            let pane_id = Uuid::new_v4();

            let dir = ensure_config_dir(pane_id).unwrap();
            let expected = pane_config_dir(pane_id);

            assert_eq!(dir, expected);

            cleanup_config_dir(pane_id).unwrap();
        });
    }

    // ==================== cleanup_config_dir Tests ====================

    #[test]
    fn test_cleanup_config_dir_removes_directory() {
        with_temp_state_dir(|| {
            let pane_id = Uuid::new_v4();
            let dir = ensure_config_dir(pane_id).unwrap();

            assert!(dir.exists());

            cleanup_config_dir(pane_id).unwrap();

            assert!(!dir.exists());
        });
    }

    #[test]
    fn test_cleanup_config_dir_removes_contents() {
        with_temp_state_dir(|| {
            let pane_id = Uuid::new_v4();
            let dir = ensure_config_dir(pane_id).unwrap();

            // Create some files inside
            File::create(dir.join(".claude.json")).unwrap();
            File::create(dir.join("test.txt")).unwrap();

            cleanup_config_dir(pane_id).unwrap();

            assert!(!dir.exists());
        });
    }

    #[test]
    fn test_cleanup_config_dir_nonexistent_ok() {
        with_temp_state_dir(|| {
            let pane_id = Uuid::new_v4();

            // Should not error on nonexistent directory
            let result = cleanup_config_dir(pane_id);
            assert!(result.is_ok());
        });
    }

    // ==================== list_config_dirs Tests ====================

    #[test]
    fn test_list_config_dirs_empty() {
        with_temp_state_dir(|| {
            let dirs = list_config_dirs().unwrap();
            assert!(dirs.is_empty());
        });
    }

    #[test]
    fn test_list_config_dirs_finds_dirs() {
        with_temp_state_dir(|| {
            let id1 = Uuid::new_v4();
            let id2 = Uuid::new_v4();

            ensure_config_dir(id1).unwrap();
            ensure_config_dir(id2).unwrap();

            let dirs = list_config_dirs().unwrap();

            assert_eq!(dirs.len(), 2);
            assert!(dirs.iter().any(|(id, _)| *id == id1));
            assert!(dirs.iter().any(|(id, _)| *id == id2));

            cleanup_config_dir(id1).unwrap();
            cleanup_config_dir(id2).unwrap();
        });
    }

    #[test]
    fn test_list_config_dirs_ignores_invalid() {
        with_temp_state_dir(|| {
            let pane_id = Uuid::new_v4();
            ensure_config_dir(pane_id).unwrap();

            // Create invalid directories
            let base = isolation_base_dir();
            std::fs::create_dir_all(base.join("not-a-pane")).unwrap();
            std::fs::create_dir_all(base.join("pane-invalid")).unwrap();

            let dirs = list_config_dirs().unwrap();

            // Should only find the valid pane directory
            assert_eq!(dirs.len(), 1);
            assert_eq!(dirs[0].0, pane_id);

            // Clean up
            let _ = std::fs::remove_dir_all(&base);
        });
    }

    // ==================== cleanup_orphaned Tests ====================

    #[test]
    fn test_cleanup_orphaned_removes_orphans() {
        with_temp_state_dir(|| {
            let active = Uuid::new_v4();
            let orphan = Uuid::new_v4();

            ensure_config_dir(active).unwrap();
            ensure_config_dir(orphan).unwrap();

            let cleaned = cleanup_orphaned(&[active]).unwrap();

            assert_eq!(cleaned, 1);
            assert!(pane_config_dir(active).exists());
            assert!(!pane_config_dir(orphan).exists());

            cleanup_config_dir(active).unwrap();
        });
    }

    #[test]
    fn test_cleanup_orphaned_preserves_active() {
        with_temp_state_dir(|| {
            let id1 = Uuid::new_v4();
            let id2 = Uuid::new_v4();

            ensure_config_dir(id1).unwrap();
            ensure_config_dir(id2).unwrap();

            let cleaned = cleanup_orphaned(&[id1, id2]).unwrap();

            assert_eq!(cleaned, 0);
            assert!(pane_config_dir(id1).exists());
            assert!(pane_config_dir(id2).exists());

            cleanup_config_dir(id1).unwrap();
            cleanup_config_dir(id2).unwrap();
        });
    }

    #[test]
    fn test_cleanup_orphaned_empty_active_list() {
        with_temp_state_dir(|| {
            let id1 = Uuid::new_v4();
            let id2 = Uuid::new_v4();

            ensure_config_dir(id1).unwrap();
            ensure_config_dir(id2).unwrap();

            let cleaned = cleanup_orphaned(&[]).unwrap();

            assert_eq!(cleaned, 2);
            assert!(!pane_config_dir(id1).exists());
            assert!(!pane_config_dir(id2).exists());
        });
    }

    // ==================== startup_cleanup Tests ====================

    #[test]
    fn test_startup_cleanup_cleans_orphans() {
        with_temp_state_dir(|| {
            let active = Uuid::new_v4();
            let orphan = Uuid::new_v4();

            ensure_config_dir(active).unwrap();
            ensure_config_dir(orphan).unwrap();

            startup_cleanup(&[active]).unwrap();

            assert!(pane_config_dir(active).exists());
            assert!(!pane_config_dir(orphan).exists());

            cleanup_config_dir(active).unwrap();
        });
    }

    #[test]
    fn test_startup_cleanup_no_orphans() {
        with_temp_state_dir(|| {
            let id1 = Uuid::new_v4();
            let id2 = Uuid::new_v4();

            ensure_config_dir(id1).unwrap();
            ensure_config_dir(id2).unwrap();

            startup_cleanup(&[id1, id2]).unwrap();

            assert!(pane_config_dir(id1).exists());
            assert!(pane_config_dir(id2).exists());

            cleanup_config_dir(id1).unwrap();
            cleanup_config_dir(id2).unwrap();
        });
    }

    // ==================== Environment Variable Names ====================

    #[test]
    fn test_env_var_names() {
        assert_eq!(CLAUDE_CONFIG_DIR_ENV, "CLAUDE_CONFIG_DIR");
        assert_eq!(CCMUX_PANE_ID_ENV, "CCMUX_PANE_ID");
    }
}
