//! PTY manager for spawning and tracking PTY instances

use std::collections::HashMap;

use ccmux_utils::{CcmuxError, Result};
use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use uuid::Uuid;

use super::{PtyConfig, PtyHandle};

/// Manages PTY instances
#[derive(Debug, Default)]
pub struct PtyManager {
    /// Active PTY handles by pane ID
    handles: HashMap<Uuid, PtyHandle>,
}

impl PtyManager {
    /// Create a new PTY manager
    pub fn new() -> Self {
        Self::default()
    }

    /// Spawn a new PTY with the given configuration
    pub fn spawn(&mut self, pane_id: Uuid, config: PtyConfig) -> Result<&PtyHandle> {
        let pty_system = native_pty_system();

        // Create PTY pair
        let pair = pty_system
            .openpty(PtySize {
                rows: config.size.1,
                cols: config.size.0,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| CcmuxError::pty(format!("Failed to open PTY: {}", e)))?;

        // Build command
        let mut cmd = CommandBuilder::new(&config.command);
        cmd.args(&config.args);

        if let Some(cwd) = &config.cwd {
            cmd.cwd(cwd);
        }

        for (key, value) in &config.env {
            cmd.env(key, value);
        }

        for key in &config.env_remove {
            cmd.env_remove(key);
        }

        // Spawn child process
        let child = pair
            .slave
            .spawn_command(cmd)
            .map_err(|e| CcmuxError::ProcessSpawn(format!("Failed to spawn: {}", e)))?;

        // Get reader and writer
        let reader = pair
            .master
            .try_clone_reader()
            .map_err(|e| CcmuxError::pty(format!("Failed to clone reader: {}", e)))?;

        let writer = pair
            .master
            .take_writer()
            .map_err(|e| CcmuxError::pty(format!("Failed to get writer: {}", e)))?;

        let handle = PtyHandle::new(pair.master, child, reader, writer);
        self.handles.insert(pane_id, handle);

        Ok(self.handles.get(&pane_id).unwrap())
    }

    /// Get a PTY handle by pane ID
    pub fn get(&self, pane_id: Uuid) -> Option<&PtyHandle> {
        self.handles.get(&pane_id)
    }

    /// Remove and return a PTY handle
    pub fn remove(&mut self, pane_id: Uuid) -> Option<PtyHandle> {
        self.handles.remove(&pane_id)
    }

    /// Check if a PTY exists for a pane
    pub fn contains(&self, pane_id: Uuid) -> bool {
        self.handles.contains_key(&pane_id)
    }

    /// Get all pane IDs with active PTYs
    pub fn pane_ids(&self) -> Vec<Uuid> {
        self.handles.keys().copied().collect()
    }

    /// Get count of active PTYs
    pub fn count(&self) -> usize {
        self.handles.len()
    }

    /// Kill all PTYs
    pub fn kill_all(&mut self) {
        for handle in self.handles.values() {
            let _ = handle.kill();
        }
        self.handles.clear();
    }

    /// Check for exited processes and return their exit codes
    pub fn poll_exits(&mut self) -> Vec<(Uuid, i32)> {
        let mut exited = Vec::new();

        for (&pane_id, handle) in &self.handles {
            if let Ok(Some(code)) = handle.try_wait() {
                exited.push((pane_id, code));
            }
        }

        // Remove exited handles
        for (pane_id, _) in &exited {
            self.handles.remove(pane_id);
        }

        exited
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manager_new() {
        let manager = PtyManager::new();
        assert_eq!(manager.count(), 0);
    }

    #[test]
    fn test_manager_spawn_echo() {
        let mut manager = PtyManager::new();
        let pane_id = Uuid::new_v4();

        // Spawn echo command (quick, doesn't need shell)
        let config = PtyConfig::command("echo").with_arg("hello");

        let result = manager.spawn(pane_id, config);
        assert!(result.is_ok());
        assert_eq!(manager.count(), 1);
        assert!(manager.contains(pane_id));
    }

    #[test]
    fn test_manager_remove() {
        let mut manager = PtyManager::new();
        let pane_id = Uuid::new_v4();

        let config = PtyConfig::command("echo").with_arg("test");
        manager.spawn(pane_id, config).unwrap();

        let handle = manager.remove(pane_id);
        assert!(handle.is_some());
        assert_eq!(manager.count(), 0);
    }

    #[test]
    fn test_pty_read_write() {
        let mut manager = PtyManager::new();
        let pane_id = Uuid::new_v4();

        // Use cat which echoes input
        let config = PtyConfig::command("cat");
        manager.spawn(pane_id, config).unwrap();

        let handle = manager.get(pane_id).unwrap();

        // Write some data
        handle.write_all(b"test\n").unwrap();

        // Small delay for process
        std::thread::sleep(std::time::Duration::from_millis(100));

        // Read output
        let mut buf = [0u8; 1024];
        let n = handle.read(&mut buf).unwrap();
        assert!(n > 0);

        // Kill the cat process
        handle.kill().unwrap();
    }

    #[test]
    fn test_pty_resize() {
        let mut manager = PtyManager::new();
        let pane_id = Uuid::new_v4();

        let config = PtyConfig::command("echo").with_arg("resize test");
        manager.spawn(pane_id, config).unwrap();

        let handle = manager.get(pane_id).unwrap();
        let result = handle.resize(120, 40);
        assert!(result.is_ok());
    }
}
