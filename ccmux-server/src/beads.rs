//! Beads integration for ccmux
//!
//! Provides passive beads awareness with automatic detection
//! of .beads/ directories and environment configuration.

use std::path::{Path, PathBuf};

/// Result of beads detection
#[derive(Debug, Clone)]
pub struct BeadsDetection {
    /// Path to the .beads/ directory
    pub beads_dir: PathBuf,
    /// Root of the repository (parent of .beads/)
    pub repo_root: PathBuf,
}

/// Detect the beads root directory by searching up from the given path
///
/// Walks up the directory tree from `cwd` looking for a `.beads/` directory.
/// Returns the path to the `.beads/` directory if found, along with the repo root.
///
/// # Arguments
/// * `cwd` - The starting directory to search from
///
/// # Returns
/// * `Some(BeadsDetection)` - If a .beads/ directory is found
/// * `None` - If no .beads/ directory is found
///
/// # Example
/// ```ignore
/// use std::path::Path;
/// use ccmux_server::beads::detect_beads_root;
///
/// if let Some(detection) = detect_beads_root(Path::new("/home/user/project/src")) {
///     println!("Found beads at: {:?}", detection.beads_dir);
/// }
/// ```
pub fn detect_beads_root(cwd: &Path) -> Option<BeadsDetection> {
    let mut current = cwd.to_path_buf();
    loop {
        let beads_dir = current.join(".beads");
        if beads_dir.is_dir() {
            return Some(BeadsDetection {
                beads_dir,
                repo_root: current,
            });
        }
        if !current.pop() {
            return None;
        }
    }
}

/// Check if a path is within a beads-tracked repository
///
/// Convenience function that returns true if the path or any parent
/// contains a .beads/ directory.
pub fn is_beads_tracked(cwd: &Path) -> bool {
    detect_beads_root(cwd).is_some()
}

/// Beads metadata keys for session storage
pub mod metadata_keys {
    /// The root path of the beads directory
    pub const BEADS_ROOT: &str = "beads.root";
    /// Whether beads was detected
    pub const BEADS_DETECTED: &str = "beads.detected";
    /// Repository root path
    pub const BEADS_REPO_ROOT: &str = "beads.repo_root";
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_detect_beads_root_found() {
        let temp = TempDir::new().unwrap();
        let beads_dir = temp.path().join(".beads");
        fs::create_dir(&beads_dir).unwrap();

        let detection = detect_beads_root(temp.path()).unwrap();
        assert_eq!(detection.beads_dir, beads_dir);
        assert_eq!(detection.repo_root, temp.path());
    }

    #[test]
    fn test_detect_beads_root_nested() {
        let temp = TempDir::new().unwrap();
        let beads_dir = temp.path().join(".beads");
        fs::create_dir(&beads_dir).unwrap();

        // Create nested directories
        let nested = temp.path().join("src").join("lib").join("deep");
        fs::create_dir_all(&nested).unwrap();

        let detection = detect_beads_root(&nested).unwrap();
        assert_eq!(detection.beads_dir, beads_dir);
        assert_eq!(detection.repo_root, temp.path());
    }

    #[test]
    fn test_detect_beads_root_not_found() {
        let temp = TempDir::new().unwrap();
        let nested = temp.path().join("no_beads_here");
        fs::create_dir_all(&nested).unwrap();

        assert!(detect_beads_root(&nested).is_none());
    }

    #[test]
    fn test_detect_beads_root_file_not_dir() {
        let temp = TempDir::new().unwrap();
        // Create .beads as a file, not a directory
        let beads_file = temp.path().join(".beads");
        fs::write(&beads_file, "not a directory").unwrap();

        assert!(detect_beads_root(temp.path()).is_none());
    }

    #[test]
    fn test_is_beads_tracked_true() {
        let temp = TempDir::new().unwrap();
        let beads_dir = temp.path().join(".beads");
        fs::create_dir(&beads_dir).unwrap();

        assert!(is_beads_tracked(temp.path()));
    }

    #[test]
    fn test_is_beads_tracked_false() {
        let temp = TempDir::new().unwrap();
        assert!(!is_beads_tracked(temp.path()));
    }

    #[test]
    fn test_is_beads_tracked_nested() {
        let temp = TempDir::new().unwrap();
        let beads_dir = temp.path().join(".beads");
        fs::create_dir(&beads_dir).unwrap();

        let nested = temp.path().join("deep").join("nested").join("path");
        fs::create_dir_all(&nested).unwrap();

        assert!(is_beads_tracked(&nested));
    }

    #[test]
    fn test_metadata_keys() {
        assert_eq!(metadata_keys::BEADS_ROOT, "beads.root");
        assert_eq!(metadata_keys::BEADS_DETECTED, "beads.detected");
        assert_eq!(metadata_keys::BEADS_REPO_ROOT, "beads.repo_root");
    }
}
