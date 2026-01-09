//! Git worktree detection and discovery

use std::path::{Path, PathBuf};
use std::process::Command;

/// Information about a git worktree
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorktreeInfo {
    /// Absolute path to the worktree
    pub path: PathBuf,
    /// Branch name (if any)
    pub branch: Option<String>,
    /// HEAD commit hash
    pub head: String,
    /// Whether this is the main worktree
    pub is_main: bool,
}

/// Worktree detection and discovery
pub struct WorktreeDetector;

impl WorktreeDetector {
    /// Check if a path is inside a git repository
    pub fn is_git_repo(path: &Path) -> bool {
        Command::new("git")
            .args(["-C", path.to_str().unwrap_or("."), "rev-parse", "--git-dir"])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    /// Get the root of the git worktree containing this path
    pub fn get_worktree_root(path: &Path) -> Option<PathBuf> {
        let output = Command::new("git")
            .args(["-C", path.to_str().unwrap_or("."), "rev-parse", "--show-toplevel"])
            .output()
            .ok()?;

        if output.status.success() {
            let root = String::from_utf8_lossy(&output.stdout).trim().to_string();
            Some(PathBuf::from(root))
        } else {
            None
        }
    }

    /// Get the main repository root (not the worktree root)
    pub fn get_main_repo_root(path: &Path) -> Option<PathBuf> {
        let output = Command::new("git")
            .args(["-C", path.to_str().unwrap_or("."), "rev-parse", "--git-common-dir"])
            .output()
            .ok()?;

        if output.status.success() {
            let git_dir = String::from_utf8_lossy(&output.stdout).trim().to_string();
            // git-common-dir returns the .git directory, get parent
            let git_path = PathBuf::from(&git_dir);
            if git_dir.ends_with(".git") {
                git_path.parent().map(|p| p.to_path_buf())
            } else {
                // Bare repo or worktree - resolve differently
                Some(git_path)
            }
        } else {
            None
        }
    }

    /// List all worktrees for the repository containing this path
    pub fn list_worktrees(path: &Path) -> Vec<WorktreeInfo> {
        let output = Command::new("git")
            .args(["-C", path.to_str().unwrap_or("."), "worktree", "list", "--porcelain"])
            .output();

        let output = match output {
            Ok(o) if o.status.success() => o,
            _ => return Vec::new(),
        };

        Self::parse_worktree_list(&String::from_utf8_lossy(&output.stdout))
    }

    /// Parse `git worktree list --porcelain` output
    fn parse_worktree_list(output: &str) -> Vec<WorktreeInfo> {
        let mut worktrees = Vec::new();
        let mut current_path: Option<PathBuf> = None;
        let mut current_head: Option<String> = None;
        let mut current_branch: Option<String> = None;
        let mut is_main = false;

        for line in output.lines() {
            if line.starts_with("worktree ") {
                // Save previous worktree if any
                if let (Some(path), Some(head)) = (current_path.take(), current_head.take()) {
                    worktrees.push(WorktreeInfo {
                        path,
                        branch: current_branch.take(),
                        head,
                        is_main,
                    });
                }
                current_path = Some(PathBuf::from(line.strip_prefix("worktree ").unwrap()));
                is_main = false;
            } else if line.starts_with("HEAD ") {
                current_head = Some(line.strip_prefix("HEAD ").unwrap().to_string());
            } else if line.starts_with("branch ") {
                let branch = line.strip_prefix("branch ").unwrap();
                // Strip refs/heads/ prefix
                current_branch = Some(
                    branch
                        .strip_prefix("refs/heads/")
                        .unwrap_or(branch)
                        .to_string(),
                );
            } else if line == "bare" {
                is_main = true;
            }
        }

        // Don't forget the last worktree
        if let (Some(path), Some(head)) = (current_path, current_head) {
            worktrees.push(WorktreeInfo {
                path,
                branch: current_branch,
                head,
                is_main,
            });
        }

        // Mark first worktree as main if none marked
        if !worktrees.is_empty() && !worktrees.iter().any(|w| w.is_main) {
            worktrees[0].is_main = true;
        }

        worktrees
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_parse_worktree_list_single() {
        let output = "worktree /home/user/project\nHEAD abc123\nbranch refs/heads/main\n";
        let worktrees = WorktreeDetector::parse_worktree_list(output);

        assert_eq!(worktrees.len(), 1);
        assert_eq!(worktrees[0].path, PathBuf::from("/home/user/project"));
        assert_eq!(worktrees[0].branch, Some("main".to_string()));
        assert_eq!(worktrees[0].head, "abc123");
        assert!(worktrees[0].is_main);
    }

    #[test]
    fn test_parse_worktree_list_multiple() {
        let output = "\
worktree /home/user/project
HEAD abc123
branch refs/heads/main

worktree /home/user/project-wt-feature
HEAD def456
branch refs/heads/feature/test
";
        let worktrees = WorktreeDetector::parse_worktree_list(output);

        assert_eq!(worktrees.len(), 2);
        assert_eq!(worktrees[0].branch, Some("main".to_string()));
        assert_eq!(worktrees[1].branch, Some("feature/test".to_string()));
    }

    #[test]
    fn test_is_git_repo_current_dir() {
        // This test runs in the ccmux repo
        let cwd = env::current_dir().unwrap();
        assert!(WorktreeDetector::is_git_repo(&cwd));
    }

    #[test]
    fn test_is_git_repo_non_repo() {
        assert!(!WorktreeDetector::is_git_repo(Path::new("/tmp")));
    }

    #[test]
    fn test_get_worktree_root() {
        let cwd = env::current_dir().unwrap();
        let root = WorktreeDetector::get_worktree_root(&cwd);
        assert!(root.is_some());
    }

    #[test]
    fn test_list_worktrees() {
        let cwd = env::current_dir().unwrap();
        let worktrees = WorktreeDetector::list_worktrees(&cwd);
        // Should find at least the main worktree
        assert!(!worktrees.is_empty());
    }
}
