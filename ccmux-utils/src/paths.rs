//! Path utilities for ccmux
//!
//! Handles XDG Base Directory specification compliance for config,
//! state, cache, and runtime directories.

use std::path::PathBuf;
use directories::ProjectDirs;

/// Application identifier for XDG directories
const APP_NAME: &str = "ccmux";

/// Get project directories (cached)
fn project_dirs() -> Option<ProjectDirs> {
    ProjectDirs::from("", "", APP_NAME)
}

/// Get the Unix socket path for client-server communication
///
/// Location: `$XDG_RUNTIME_DIR/ccmux.sock` or `/tmp/ccmux-$UID/ccmux.sock`
pub fn socket_path() -> PathBuf {
    runtime_dir().join("ccmux.sock")
}

/// Get the runtime directory
///
/// Location: `$XDG_RUNTIME_DIR/ccmux` or `/tmp/ccmux-$UID`
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
/// Location: `$XDG_CONFIG_HOME/ccmux` or `~/.config/ccmux`
pub fn config_dir() -> PathBuf {
    project_dirs()
        .map(|p| p.config_dir().to_path_buf())
        .unwrap_or_else(fallback_config_dir)
}

/// Get the main configuration file path
///
/// Location: `$XDG_CONFIG_HOME/ccmux/config.toml`
pub fn config_file() -> PathBuf {
    config_dir().join("config.toml")
}

/// Get the state directory (persistent state like session data)
///
/// Location: `$XDG_STATE_HOME/ccmux` or `~/.local/state/ccmux`
pub fn state_dir() -> PathBuf {
    project_dirs()
        .and_then(|p| p.state_dir().map(|d| d.to_path_buf()))
        .unwrap_or_else(fallback_state_dir)
}

/// Get the data directory (persistent data like checkpoints)
///
/// Location: `$XDG_DATA_HOME/ccmux` or `~/.local/share/ccmux`
pub fn data_dir() -> PathBuf {
    project_dirs()
        .map(|p| p.data_local_dir().to_path_buf())
        .unwrap_or_else(fallback_data_dir)
}

/// Get the cache directory (temporary data, safe to delete)
///
/// Location: `$XDG_CACHE_HOME/ccmux` or `~/.cache/ccmux`
pub fn cache_dir() -> PathBuf {
    project_dirs()
        .map(|p| p.cache_dir().to_path_buf())
        .unwrap_or_else(fallback_cache_dir)
}

/// Get the log directory
///
/// Location: `$XDG_STATE_HOME/ccmux/log` or `~/.local/state/ccmux/log`
pub fn log_dir() -> PathBuf {
    state_dir().join("log")
}

/// Get the checkpoints directory (for persistence)
///
/// Location: `$XDG_DATA_HOME/ccmux/checkpoints`
pub fn checkpoints_dir() -> PathBuf {
    data_dir().join("checkpoints")
}

/// Get the WAL directory (for persistence)
///
/// Location: `$XDG_DATA_HOME/ccmux/wal`
pub fn wal_dir() -> PathBuf {
    data_dir().join("wal")
}

/// Get the PID file path (for daemon)
///
/// Location: `$XDG_RUNTIME_DIR/ccmux/ccmux.pid`
pub fn pid_file() -> PathBuf {
    runtime_dir().join("ccmux.pid")
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

    #[test]
    fn test_socket_path() {
        let path = socket_path();
        assert!(path.to_string_lossy().contains("ccmux.sock"));
    }

    #[test]
    fn test_config_dir() {
        let path = config_dir();
        assert!(path.to_string_lossy().contains("ccmux"));
    }

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
    fn test_config_file_is_toml() {
        let path = config_file();
        assert!(path.to_string_lossy().ends_with(".toml"));
    }
}
