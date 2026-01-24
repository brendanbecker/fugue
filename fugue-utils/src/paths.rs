//! Path utilities for fugue
//!
//! Handles XDG Base Directory specification compliance for config,
//! state, cache, and runtime directories.

use std::path::PathBuf;
use directories::ProjectDirs;

/// Application identifier for XDG directories
const APP_NAME: &str = "fugue";

/// Get project directories (cached)
fn project_dirs() -> Option<ProjectDirs> {
    ProjectDirs::from("", "", APP_NAME)
}

/// Get the Unix socket path for client-server communication
///
/// Location: `$XDG_RUNTIME_DIR/fugue.sock` or `/tmp/fugue-$UID/fugue.sock`
pub fn socket_path() -> PathBuf {
    runtime_dir().join("fugue.sock")
}

/// Get the runtime directory
///
/// Location: `$XDG_RUNTIME_DIR/fugue` or `/tmp/fugue-$UID`
pub fn runtime_dir() -> PathBuf {
    if let Ok(xdg_runtime) = std::env::var("XDG_RUNTIME_DIR") {
        PathBuf::from(xdg_runtime).join(APP_NAME)
    } else {
        // Fallback to /tmp with UID for security
        // SAFETY: getuid() is always safe to call
        let uid = unsafe { libc::getuid() };
        PathBuf::from(format!("/tmp/{}-{}", APP_NAME, uid))
    }
}

/// Get the configuration directory
///
/// Location: `$XDG_CONFIG_HOME/fugue` or `~/.config/fugue`
pub fn config_dir() -> PathBuf {
    project_dirs()
        .map(|p| p.config_dir().to_path_buf())
        .unwrap_or_else(fallback_config_dir)
}

/// Get the main configuration file path
///
/// Location: `$XDG_CONFIG_HOME/fugue/config.toml`
pub fn config_file() -> PathBuf {
    config_dir().join("config.toml")
}

/// Get the state directory (persistent state like session data)
///
/// Location: `$XDG_STATE_HOME/fugue` or `~/.local/state/fugue`
pub fn state_dir() -> PathBuf {
    project_dirs()
        .and_then(|p| p.state_dir().map(|d| d.to_path_buf()))
        .unwrap_or_else(fallback_state_dir)
}

/// Get the data directory (persistent data like checkpoints)
///
/// Location: `$XDG_DATA_HOME/fugue` or `~/.local/share/fugue`
pub fn data_dir() -> PathBuf {
    project_dirs()
        .map(|p| p.data_local_dir().to_path_buf())
        .unwrap_or_else(fallback_data_dir)
}

/// Get the cache directory (temporary data, safe to delete)
///
/// Location: `$XDG_CACHE_HOME/fugue` or `~/.cache/fugue`
pub fn cache_dir() -> PathBuf {
    project_dirs()
        .map(|p| p.cache_dir().to_path_buf())
        .unwrap_or_else(fallback_cache_dir)
}

/// Get the log directory
///
/// Location: `$XDG_STATE_HOME/fugue/log` or `~/.local/state/fugue/log`
pub fn log_dir() -> PathBuf {
    state_dir().join("log")
}

/// Get the session-specific log directory
///
/// Location: `$XDG_STATE_HOME/fugue/log/{session_id}`
pub fn session_log_dir(session_id: uuid::Uuid) -> PathBuf {
    log_dir().join(session_id.to_string())
}

/// Get the checkpoints directory (for persistence)
///
/// Location: `$XDG_DATA_HOME/fugue/checkpoints`
pub fn checkpoints_dir() -> PathBuf {
    data_dir().join("checkpoints")
}

/// Get the WAL directory (for persistence)
///
/// Location: `$XDG_DATA_HOME/fugue/wal`
pub fn wal_dir() -> PathBuf {
    data_dir().join("wal")
}

/// Get the PID file path (for daemon)
///
/// Location: `$XDG_RUNTIME_DIR/fugue/fugue.pid`
pub fn pid_file() -> PathBuf {
    runtime_dir().join("fugue.pid")
}

/// Ensure a directory exists, creating it if necessary
pub fn ensure_dir(path: &PathBuf) -> std::io::Result<()> {
    if !path.exists() {
        std::fs::create_dir_all(path)?;
    }
    Ok(())
}

/// Ensure all required directories exist
pub fn ensure_all_dirs() -> std::io::Result<()> {
    ensure_dir(&runtime_dir())?;
    ensure_dir(&config_dir())?;
    ensure_dir(&state_dir())?;
    ensure_dir(&data_dir())?;
    ensure_dir(&cache_dir())?;
    ensure_dir(&log_dir())?;
    Ok(())
}

// Fallback implementations when ProjectDirs is unavailable

fn home_dir() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/tmp"))
}

fn fallback_config_dir() -> PathBuf {
    home_dir().join(".config").join(APP_NAME)
}

fn fallback_state_dir() -> PathBuf {
    home_dir().join(".local").join("state").join(APP_NAME)
}

fn fallback_data_dir() -> PathBuf {
    home_dir().join(".local").join("share").join(APP_NAME)
}

fn fallback_cache_dir() -> PathBuf {
    home_dir().join(".cache").join(APP_NAME)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    // ==================== Socket Path Tests ====================

    #[test]
    fn test_socket_path() {
        let path = socket_path();
        assert!(path.to_string_lossy().contains("fugue.sock"));
    }

    #[test]
    fn test_socket_path_is_in_runtime_dir() {
        let sock = socket_path();
        let runtime = runtime_dir();
        assert!(sock.starts_with(&runtime));
    }

    #[test]
    fn test_socket_path_has_correct_filename() {
        let path = socket_path();
        assert_eq!(path.file_name().unwrap().to_str().unwrap(), "fugue.sock");
    }

    // ==================== Runtime Dir Tests ====================

    #[test]
    fn test_runtime_dir_contains_fugue() {
        let path = runtime_dir();
        assert!(path.to_string_lossy().contains("fugue"));
    }

    #[test]
    fn test_runtime_dir_with_xdg_set() {
        // Save original value
        let original = env::var("XDG_RUNTIME_DIR").ok();

        // Set custom value
        env::set_var("XDG_RUNTIME_DIR", "/run/user/1000");
        let path = runtime_dir();
        assert_eq!(path, PathBuf::from("/run/user/1000/fugue"));

        // Restore original
        match original {
            Some(val) => env::set_var("XDG_RUNTIME_DIR", val),
            None => env::remove_var("XDG_RUNTIME_DIR"),
        }
    }

    #[test]
    fn test_runtime_dir_fallback() {
        // Save original value
        let original = env::var("XDG_RUNTIME_DIR").ok();

        // Remove env var
        env::remove_var("XDG_RUNTIME_DIR");
        let path = runtime_dir();

        // Should be /tmp/fugue-UID
        let path_str = path.to_string_lossy();
        assert!(path_str.starts_with("/tmp/fugue-"));

        // Restore original
        if let Some(val) = original {
            env::set_var("XDG_RUNTIME_DIR", val);
        }
    }

    // ==================== Config Dir Tests ====================

    #[test]
    fn test_config_dir() {
        let path = config_dir();
        assert!(path.to_string_lossy().contains("fugue"));
    }

    #[test]
    fn test_config_dir_xdg_compliance() {
        let path = config_dir();
        let path_str = path.to_string_lossy();
        // Should be in a config-related location
        assert!(
            path_str.contains(".config") || path_str.contains("config"),
            "Config dir should be in a config location: {:?}",
            path
        );
    }

    // ==================== Config File Tests ====================

    #[test]
    fn test_config_file_is_toml() {
        let path = config_file();
        assert!(path.to_string_lossy().ends_with(".toml"));
    }

    #[test]
    fn test_config_file_in_config_dir() {
        let file = config_file();
        let dir = config_dir();
        assert!(file.starts_with(&dir));
    }

    #[test]
    fn test_config_file_name() {
        let path = config_file();
        assert_eq!(path.file_name().unwrap().to_str().unwrap(), "config.toml");
    }

    // ==================== State Dir Tests ====================

    #[test]
    fn test_state_dir_contains_fugue() {
        let path = state_dir();
        assert!(path.to_string_lossy().contains("fugue"));
    }

    #[test]
    fn test_state_dir_xdg_compliance() {
        let path = state_dir();
        let path_str = path.to_string_lossy();
        // Should be in a state-related location
        assert!(
            path_str.contains("state") || path_str.contains(".local"),
            "State dir should be in a state location: {:?}",
            path
        );
    }

    // ==================== Data Dir Tests ====================

    #[test]
    fn test_data_dir_contains_fugue() {
        let path = data_dir();
        assert!(path.to_string_lossy().contains("fugue"));
    }

    #[test]
    fn test_data_dir_xdg_compliance() {
        let path = data_dir();
        let path_str = path.to_string_lossy();
        // Should be in a data-related location
        assert!(
            path_str.contains("share") || path_str.contains(".local"),
            "Data dir should be in a data location: {:?}",
            path
        );
    }

    // ==================== Cache Dir Tests ====================

    #[test]
    fn test_cache_dir_contains_fugue() {
        let path = cache_dir();
        assert!(path.to_string_lossy().contains("fugue"));
    }

    #[test]
    fn test_cache_dir_xdg_compliance() {
        let path = cache_dir();
        let path_str = path.to_string_lossy();
        // Should be in a cache-related location
        assert!(
            path_str.contains("cache"),
            "Cache dir should be in a cache location: {:?}",
            path
        );
    }

    // ==================== Log Dir Tests ====================

    #[test]
    fn test_log_dir_is_under_state() {
        let log = log_dir();
        let state = state_dir();
        assert!(log.starts_with(&state));
    }

    #[test]
    fn test_log_dir_name() {
        let path = log_dir();
        assert_eq!(path.file_name().unwrap().to_str().unwrap(), "log");
    }

    // ==================== Checkpoints Dir Tests ====================

    #[test]
    fn test_checkpoints_dir_is_under_data() {
        let checkpoints = checkpoints_dir();
        let data = data_dir();
        assert!(checkpoints.starts_with(&data));
    }

    #[test]
    fn test_checkpoints_dir_name() {
        let path = checkpoints_dir();
        assert_eq!(path.file_name().unwrap().to_str().unwrap(), "checkpoints");
    }

    // ==================== WAL Dir Tests ====================

    #[test]
    fn test_wal_dir_is_under_data() {
        let wal = wal_dir();
        let data = data_dir();
        assert!(wal.starts_with(&data));
    }

    #[test]
    fn test_wal_dir_name() {
        let path = wal_dir();
        assert_eq!(path.file_name().unwrap().to_str().unwrap(), "wal");
    }

    // ==================== PID File Tests ====================

    #[test]
    fn test_pid_file_is_in_runtime_dir() {
        let pid = pid_file();
        let runtime = runtime_dir();
        assert!(pid.starts_with(&runtime));
    }

    #[test]
    fn test_pid_file_name() {
        let path = pid_file();
        assert_eq!(path.file_name().unwrap().to_str().unwrap(), "fugue.pid");
    }

    // ==================== ensure_dir Tests ====================

    #[test]
    fn test_ensure_dir_creates_directory() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let test_dir = temp_dir.path().join("subdir");

        // Create it
        let result = ensure_dir(&test_dir);
        assert!(result.is_ok());
        assert!(test_dir.exists());
        assert!(test_dir.is_dir());
        // temp_dir auto-cleans on drop
    }

    #[test]
    fn test_ensure_dir_nested() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let test_dir = temp_dir.path().join("nested").join("deep");

        // Create nested directories
        let result = ensure_dir(&test_dir);
        assert!(result.is_ok());
        assert!(test_dir.exists());
        assert!(test_dir.is_dir());
        // temp_dir auto-cleans on drop
    }

    #[test]
    fn test_ensure_dir_already_exists() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let test_dir = temp_dir.path().join("existing");

        // Create it first
        std::fs::create_dir_all(&test_dir).unwrap();

        // ensure_dir should succeed even if it exists
        let result = ensure_dir(&test_dir);
        assert!(result.is_ok());
        // temp_dir auto-cleans on drop
    }

    // ==================== Path Consistency Tests ====================

    #[test]
    fn test_paths_are_absolute_or_relative_to_home() {
        // All paths should be under a known location
        let paths = [
            socket_path(),
            config_dir(),
            config_file(),
            state_dir(),
            data_dir(),
            cache_dir(),
            log_dir(),
            pid_file(),
        ];

        for path in paths {
            let path_str = path.to_string_lossy();
            assert!(
                path_str.starts_with('/') || path_str.starts_with('~'),
                "Path should be absolute: {:?}",
                path
            );
        }
    }

    #[test]
    fn test_all_paths_contain_fugue() {
        let paths = [
            socket_path(),
            runtime_dir(),
            config_dir(),
            config_file(),
            state_dir(),
            data_dir(),
            cache_dir(),
            log_dir(),
            checkpoints_dir(),
            wal_dir(),
            pid_file(),
        ];

        for path in paths {
            let path_str = path.to_string_lossy();
            assert!(
                path_str.contains("fugue"),
                "Path should contain 'fugue': {:?}",
                path
            );
        }
    }

    #[test]
    fn test_subdirs_are_under_parents() {
        // Log should be under state
        assert!(log_dir().starts_with(state_dir()));

        // Checkpoints and WAL should be under data
        assert!(checkpoints_dir().starts_with(data_dir()));
        assert!(wal_dir().starts_with(data_dir()));

        // Config file should be under config dir
        assert!(config_file().starts_with(config_dir()));

        // Socket and PID should be under runtime
        assert!(socket_path().starts_with(runtime_dir()));
        assert!(pid_file().starts_with(runtime_dir()));
    }

    // ==================== Fallback Tests ====================

    #[test]
    fn test_fallback_config_dir() {
        let path = fallback_config_dir();
        assert!(path.to_string_lossy().contains(".config"));
        assert!(path.to_string_lossy().contains("fugue"));
    }

    #[test]
    fn test_fallback_state_dir() {
        let path = fallback_state_dir();
        assert!(path.to_string_lossy().contains(".local/state"));
        assert!(path.to_string_lossy().contains("fugue"));
    }

    #[test]
    fn test_fallback_data_dir() {
        let path = fallback_data_dir();
        assert!(path.to_string_lossy().contains(".local/share"));
        assert!(path.to_string_lossy().contains("fugue"));
    }

    #[test]
    fn test_fallback_cache_dir() {
        let path = fallback_cache_dir();
        assert!(path.to_string_lossy().contains(".cache"));
        assert!(path.to_string_lossy().contains("fugue"));
    }

    #[test]
    fn test_home_dir_returns_path() {
        let home = home_dir();
        // Should return something (either HOME or /tmp fallback)
        assert!(!home.as_os_str().is_empty());
    }

    #[test]
    fn test_home_dir_with_home_set() {
        let original = env::var("HOME").ok();

        env::set_var("HOME", "/home/testuser");
        let home = home_dir();
        assert_eq!(home, PathBuf::from("/home/testuser"));

        // Restore
        match original {
            Some(val) => env::set_var("HOME", val),
            None => env::remove_var("HOME"),
        }
    }
}
