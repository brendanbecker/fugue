//! Session restoration from persisted state
//!
//! This module handles restoring sessions after crash recovery:
//! - Rebuilding the session hierarchy from snapshots
//! - Spawning new PTYs for each pane
//! - Handling failures gracefully with detailed status reporting

// Scaffolding for crash recovery feature - not all methods are wired up yet
#![allow(dead_code)]

use std::path::Path;

use tracing::{debug, error, info, warn};
use uuid::Uuid;

use fugue_protocol::PaneState;

use crate::claude::create_resume_command;
use crate::isolation;
use crate::pty::{PtyConfig, PtyManager};
use crate::session::{Pane, Session, SessionManager, Window};

use super::types::{PaneSnapshot, RecoveryState, SessionSnapshot, WindowSnapshot};

/// Result of restoring a single pane
#[derive(Debug, Clone)]
pub struct PaneRestorationResult {
    /// Pane ID
    pub pane_id: Uuid,
    /// Whether PTY spawn was attempted (false for exited panes)
    pub pty_attempted: bool,
    /// Whether PTY was successfully spawned
    pub pty_spawned: bool,
    /// Error message if PTY spawn failed
    pub error: Option<String>,
    /// Whether CWD was available
    pub cwd_restored: bool,
    /// Whether this was a Claude session resume
    pub claude_resumed: bool,
    /// The session ID used for Claude resume (if any)
    pub claude_session_id: Option<String>,
}

/// Result of restoring a session
#[derive(Debug, Clone)]
pub struct SessionRestorationResult {
    /// Session ID
    pub session_id: Uuid,
    /// Session name
    pub session_name: String,
    /// Number of windows restored
    pub windows_restored: usize,
    /// Pane restoration results
    pub pane_results: Vec<PaneRestorationResult>,
}

/// Overall restoration result
#[derive(Debug, Clone, Default)]
pub struct RestorationResult {
    /// Session restoration results
    pub sessions: Vec<SessionRestorationResult>,
    /// Total panes restored
    pub total_panes: usize,
    /// Panes with successful PTY spawn
    pub successful_ptys: usize,
    /// Panes with failed PTY spawn
    pub failed_ptys: usize,
    /// Whether recovery was from unclean shutdown
    pub was_crash_recovery: bool,
}

impl RestorationResult {
    /// Check if all PTYs were spawned successfully
    pub fn all_successful(&self) -> bool {
        self.failed_ptys == 0
    }

    /// Get summary message
    pub fn summary(&self) -> String {
        if self.sessions.is_empty() {
            return "No sessions to restore".to_string();
        }

        let crash_prefix = if self.was_crash_recovery {
            "Crash recovery: "
        } else {
            ""
        };

        if self.all_successful() {
            format!(
                "{}Restored {} session(s) with {} pane(s)",
                crash_prefix,
                self.sessions.len(),
                self.total_panes
            )
        } else {
            format!(
                "{}Restored {} session(s) with {} pane(s) ({} PTY failures)",
                crash_prefix,
                self.sessions.len(),
                self.total_panes,
                self.failed_ptys
            )
        }
    }
}

/// Restores sessions from persisted state
pub struct SessionRestorer {
    /// Whether to spawn PTYs (can be disabled for testing)
    spawn_ptys: bool,
}

impl Default for SessionRestorer {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionRestorer {
    /// Create a new session restorer
    pub fn new() -> Self {
        Self { spawn_ptys: true }
    }

    /// Create a restorer that doesn't spawn PTYs (for testing)
    pub fn without_pty_spawn() -> Self {
        Self { spawn_ptys: false }
    }

    /// Restore sessions from recovery state
    ///
    /// This rebuilds the session hierarchy and optionally spawns PTYs for each pane.
    pub fn restore(
        &self,
        state: &RecoveryState,
        session_manager: &mut SessionManager,
        pty_manager: &mut PtyManager,
    ) -> RestorationResult {
        let mut result = RestorationResult {
            was_crash_recovery: !state.clean_shutdown,
            ..Default::default()
        };

        if state.sessions.is_empty() {
            info!("No sessions to restore");
            return result;
        }

        info!(
            "Restoring {} session(s) from {}",
            state.sessions.len(),
            if state.clean_shutdown {
                "clean shutdown"
            } else {
                "crash recovery"
            }
        );

        for session_snapshot in &state.sessions {
            let session_result = self.restore_session(session_snapshot, session_manager, pty_manager);

            result.total_panes += session_result.pane_results.len();
            result.successful_ptys += session_result
                .pane_results
                .iter()
                .filter(|p| p.pty_attempted && p.pty_spawned)
                .count();
            result.failed_ptys += session_result
                .pane_results
                .iter()
                .filter(|p| p.pty_attempted && !p.pty_spawned)
                .count();

            result.sessions.push(session_result);
        }

        info!("{}", result.summary());

        result
    }

    /// Restore a single session
    fn restore_session(
        &self,
        snapshot: &SessionSnapshot,
        session_manager: &mut SessionManager,
        pty_manager: &mut PtyManager,
    ) -> SessionRestorationResult {
        debug!("Restoring session '{}' ({})", snapshot.name, snapshot.id);

        // Create restored session with metadata
        let mut session = Session::restore_with_metadata(
            snapshot.id,
            &snapshot.name,
            snapshot.created_at,
            snapshot.metadata.clone(),
        );

        let mut pane_results = Vec::new();

        // Restore windows
        for window_snapshot in &snapshot.windows {
            let (window, window_pane_results) =
                self.restore_window(window_snapshot, pty_manager, &snapshot.name);

            pane_results.extend(window_pane_results);
            session.add_restored_window(window);
        }

        // Set active window
        session.set_active_window_id(snapshot.active_window_id);

        let windows_restored = session.window_count();

        // Add to session manager
        session_manager.add_restored_session(session);

        SessionRestorationResult {
            session_id: snapshot.id,
            session_name: snapshot.name.clone(),
            windows_restored,
            pane_results,
        }
    }

    /// Restore a single window
    fn restore_window(
        &self,
        snapshot: &WindowSnapshot,
        pty_manager: &mut PtyManager,
        session_name: &str,
    ) -> (Window, Vec<PaneRestorationResult>) {
        debug!(
            "Restoring window '{}' ({}) with {} panes",
            snapshot.name,
            snapshot.id,
            snapshot.panes.len()
        );

        let mut window = Window::restore(
            snapshot.id,
            snapshot.session_id,
            snapshot.index,
            &snapshot.name,
            snapshot.created_at,
        );

        let mut pane_results = Vec::new();

        // Restore panes
        for pane_snapshot in &snapshot.panes {
            let (pane, pane_result) = self.restore_pane(
                pane_snapshot,
                pty_manager,
                snapshot.session_id,
                session_name,
            );
            pane_results.push(pane_result);
            window.add_restored_pane(pane);
        }

        // Set active pane
        window.set_active_pane_id(snapshot.active_pane_id);

        (window, pane_results)
    }

    /// Restore a single pane
    fn restore_pane(
        &self,
        snapshot: &PaneSnapshot,
        pty_manager: &mut PtyManager,
        session_id: Uuid,
        session_name: &str,
    ) -> (Pane, PaneRestorationResult) {
        debug!(
            "Restoring pane {} ({}x{})",
            snapshot.id, snapshot.cols, snapshot.rows
        );

        // Create pane with restored state
        let pane = Pane::restore(
            snapshot.id,
            snapshot.window_id,
            snapshot.index,
            snapshot.cols,
            snapshot.rows,
            snapshot.state.clone(),
            snapshot.name.clone(),
            snapshot.title.clone(),
            snapshot.cwd.clone(),
            snapshot.created_at,
        );

        // Determine if we should spawn a PTY
        let should_spawn_pty = self.spawn_ptys && Self::should_spawn_pty(&snapshot.state);

        let mut result = PaneRestorationResult {
            pane_id: snapshot.id,
            pty_attempted: should_spawn_pty,
            pty_spawned: false,
            error: None,
            cwd_restored: false,
            claude_resumed: false,
            claude_session_id: None,
        };

        // Check if this is a Claude pane with a session ID to resume
        let claude_session_id = if let PaneState::Agent(ref agent_state) = snapshot.state {
            if agent_state.is_claude() {
                agent_state.session_id.clone()
            } else {
                None
            }
        } else {
            None
        };

        // Track Claude resume intent (even if PTY spawning is disabled)
        if let Some(ref session_id) = claude_session_id {
            result.claude_resumed = true;
            result.claude_session_id = Some(session_id.clone());
        }

        if should_spawn_pty {
            // Build PTY config based on whether we're resuming a Claude session
            let mut pty_config = if let Some(ref session_id) = claude_session_id {
                // Resume Claude session
                let (cmd, args) = create_resume_command(session_id);
                info!(
                    "Resuming Claude session {} for pane {}",
                    session_id, snapshot.id
                );

                let mut config = PtyConfig::command(&cmd).with_size(snapshot.cols, snapshot.rows);
                for arg in args {
                    config = config.with_arg(arg);
                }
                config
            } else {
                // Normal shell or Claude without session ID
                PtyConfig::shell().with_size(snapshot.cols, snapshot.rows)
            };

            // Try to restore CWD
            if let Some(ref cwd) = snapshot.cwd {
                if Path::new(cwd).exists() {
                    pty_config = pty_config.with_cwd(cwd);
                    result.cwd_restored = true;
                } else {
                    warn!(
                        "CWD '{}' no longer exists for pane {}, using default",
                        cwd, snapshot.id
                    );
                }
            }

            // Apply isolation for Claude panes
            let is_claude_pane = matches!(&snapshot.state, PaneState::Agent(s) if s.is_claude());
            if is_claude_pane {
                match isolation::ensure_config_dir(snapshot.id) {
                    Ok(config_dir) => {
                        debug!(
                            "Setting up isolation for Claude pane {}: {}",
                            snapshot.id,
                            config_dir.display()
                        );
                        pty_config = pty_config
                            .with_env(isolation::CLAUDE_CONFIG_DIR_ENV, config_dir.to_string_lossy().as_ref())
                            .with_env(isolation::FUGUE_PANE_ID_ENV, snapshot.id.to_string());
                    }
                    Err(e) => {
                        warn!(
                            "Failed to create isolation dir for pane {}: {}",
                            snapshot.id, e
                        );
                        // Continue without isolation - better than failing
                    }
                }
            }

            // Add CCMUX context environment variables
            pty_config = pty_config.with_fugue_context(
                session_id,
                session_name,
                snapshot.window_id,
                snapshot.id,
            );

            // Spawn PTY
            match pty_manager.spawn(snapshot.id, pty_config) {
                Ok(_) => {
                    debug!("Spawned PTY for pane {}", snapshot.id);
                    result.pty_spawned = true;
                }
                Err(e) => {
                    error!("Failed to spawn PTY for pane {}: {}", snapshot.id, e);
                    result.error = Some(e.to_string());
                }
            }
        } else if !self.spawn_ptys {
            // PTY spawning disabled (testing mode)
            result.pty_attempted = true;
            result.pty_spawned = true; // Mark as successful for testing
        }

        (pane, result)
    }

    /// Determine if a PTY should be spawned for a pane based on its state
    fn should_spawn_pty(state: &PaneState) -> bool {
        match state {
            // Normal panes always get a PTY
            PaneState::Normal => true,
            // Agent panes get a PTY (agents run in shell)
            PaneState::Agent(_) => true,
            // Exited panes don't need a PTY
            PaneState::Exited { .. } => false,
            // Status panes don't need a PTY
            PaneState::Status => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use fugue_protocol::{AgentState, AgentActivity};

    fn create_test_session_snapshot() -> SessionSnapshot {
        let session_id = Uuid::new_v4();
        let window_id = Uuid::new_v4();
        let pane_id = Uuid::new_v4();

        SessionSnapshot {
            id: session_id,
            name: "test-session".to_string(),
            windows: vec![WindowSnapshot {
                id: window_id,
                session_id,
                name: "main".to_string(),
                index: 0,
                panes: vec![PaneSnapshot {
                    id: pane_id,
                    window_id,
                    index: 0,
                    cols: 80,
                    rows: 24,
                    state: PaneState::Normal,
                    name: None,
                    title: Some("bash".to_string()),
                    cwd: Some("/tmp".to_string()),
                    created_at: 12345,
                    scrollback: None,
                }],
                active_pane_id: Some(pane_id),
                created_at: 12345,
            }],
            active_window_id: Some(window_id),
            created_at: 12345,
            metadata: HashMap::new(),
            environment: HashMap::new(),
        }
    }

    #[test]
    fn test_restorer_new() {
        let restorer = SessionRestorer::new();
        assert!(restorer.spawn_ptys);
    }

    #[test]
    fn test_restorer_without_pty_spawn() {
        let restorer = SessionRestorer::without_pty_spawn();
        assert!(!restorer.spawn_ptys);
    }

    #[test]
    fn test_restore_empty_state() {
        let restorer = SessionRestorer::without_pty_spawn();
        let state = RecoveryState::default();
        let mut session_manager = SessionManager::new();
        let mut pty_manager = PtyManager::new();

        let result = restorer.restore(&state, &mut session_manager, &mut pty_manager);

        assert!(result.sessions.is_empty());
        assert_eq!(result.total_panes, 0);
        assert!(result.all_successful());
    }

    #[test]
    fn test_restore_session() {
        let restorer = SessionRestorer::without_pty_spawn();

        let snapshot = create_test_session_snapshot();
        let state = RecoveryState {
            sessions: vec![snapshot.clone()],
            clean_shutdown: true,
            ..Default::default()
        };

        let mut session_manager = SessionManager::new();
        let mut pty_manager = PtyManager::new();

        let result = restorer.restore(&state, &mut session_manager, &mut pty_manager);

        assert_eq!(result.sessions.len(), 1);
        assert_eq!(result.total_panes, 1);
        assert!(!result.was_crash_recovery);

        // Check session was added
        let session = session_manager.get_session(snapshot.id);
        assert!(session.is_some());
        assert_eq!(session.unwrap().name(), "test-session");
    }

    #[test]
    fn test_restore_preserves_ids() {
        let restorer = SessionRestorer::without_pty_spawn();

        let snapshot = create_test_session_snapshot();
        let session_id = snapshot.id;
        let window_id = snapshot.windows[0].id;
        let pane_id = snapshot.windows[0].panes[0].id;

        let state = RecoveryState {
            sessions: vec![snapshot],
            clean_shutdown: true,
            ..Default::default()
        };

        let mut session_manager = SessionManager::new();
        let mut pty_manager = PtyManager::new();

        restorer.restore(&state, &mut session_manager, &mut pty_manager);

        // Check IDs are preserved
        let session = session_manager.get_session(session_id).unwrap();
        assert_eq!(session.id(), session_id);

        let window = session.get_window(window_id).unwrap();
        assert_eq!(window.id(), window_id);

        let pane = window.get_pane(pane_id).unwrap();
        assert_eq!(pane.id(), pane_id);
    }

    #[test]
    fn test_restore_preserves_active() {
        let restorer = SessionRestorer::without_pty_spawn();

        let snapshot = create_test_session_snapshot();
        let window_id = snapshot.windows[0].id;
        let pane_id = snapshot.windows[0].panes[0].id;

        let state = RecoveryState {
            sessions: vec![snapshot.clone()],
            clean_shutdown: true,
            ..Default::default()
        };

        let mut session_manager = SessionManager::new();
        let mut pty_manager = PtyManager::new();

        restorer.restore(&state, &mut session_manager, &mut pty_manager);

        let session = session_manager.get_session(snapshot.id).unwrap();
        assert_eq!(session.active_window_id(), Some(window_id));

        let window = session.get_window(window_id).unwrap();
        assert_eq!(window.active_pane_id(), Some(pane_id));
    }

    #[test]
    fn test_restore_preserves_pane_attributes() {
        let restorer = SessionRestorer::without_pty_spawn();

        let snapshot = create_test_session_snapshot();
        let pane_snapshot = &snapshot.windows[0].panes[0];

        let state = RecoveryState {
            sessions: vec![snapshot.clone()],
            clean_shutdown: true,
            ..Default::default()
        };

        let mut session_manager = SessionManager::new();
        let mut pty_manager = PtyManager::new();

        restorer.restore(&state, &mut session_manager, &mut pty_manager);

        let (_, _, pane) = session_manager.find_pane(pane_snapshot.id).unwrap();

        assert_eq!(pane.dimensions(), (80, 24));
        assert_eq!(pane.title(), Some("bash"));
        assert_eq!(pane.cwd(), Some("/tmp"));
    }

    #[test]
    fn test_should_spawn_pty_normal() {
        assert!(SessionRestorer::should_spawn_pty(&PaneState::Normal));
    }

    #[test]
    fn test_should_spawn_pty_agent() {
        assert!(SessionRestorer::should_spawn_pty(&PaneState::Agent(
            AgentState::new("claude")
        )));
    }

    #[test]
    fn test_should_spawn_pty_exited() {
        assert!(!SessionRestorer::should_spawn_pty(&PaneState::Exited {
            code: Some(0)
        }));
    }

    #[test]
    fn test_restore_multiple_sessions() {
        let restorer = SessionRestorer::without_pty_spawn();

        let snapshot1 = create_test_session_snapshot();
        let mut snapshot2 = create_test_session_snapshot();
        snapshot2.id = Uuid::new_v4();
        snapshot2.name = "test-session-2".to_string();

        let state = RecoveryState {
            sessions: vec![snapshot1.clone(), snapshot2.clone()],
            clean_shutdown: true,
            ..Default::default()
        };

        let mut session_manager = SessionManager::new();
        let mut pty_manager = PtyManager::new();

        let result = restorer.restore(&state, &mut session_manager, &mut pty_manager);

        assert_eq!(result.sessions.len(), 2);
        assert_eq!(result.total_panes, 2);
        assert_eq!(session_manager.session_count(), 2);
    }

    #[test]
    fn test_result_summary_empty() {
        let result = RestorationResult::default();
        assert_eq!(result.summary(), "No sessions to restore");
    }

    #[test]
    fn test_result_summary_success() {
        let result = RestorationResult {
            sessions: vec![SessionRestorationResult {
                session_id: Uuid::new_v4(),
                session_name: "test".to_string(),
                windows_restored: 1,
                pane_results: vec![PaneRestorationResult {
                    pane_id: Uuid::new_v4(),
                    pty_attempted: true,
                    pty_spawned: true,
                    error: None,
                    cwd_restored: true,
                    claude_resumed: false,
                    claude_session_id: None,
                }],
            }],
            total_panes: 1,
            successful_ptys: 1,
            failed_ptys: 0,
            was_crash_recovery: false,
        };

        assert!(result.summary().contains("Restored 1 session"));
        assert!(result.summary().contains("1 pane"));
    }

    #[test]
    fn test_result_summary_crash_recovery() {
        let result = RestorationResult {
            sessions: vec![SessionRestorationResult {
                session_id: Uuid::new_v4(),
                session_name: "test".to_string(),
                windows_restored: 1,
                pane_results: vec![],
            }],
            total_panes: 0,
            successful_ptys: 0,
            failed_ptys: 0,
            was_crash_recovery: true,
        };

        assert!(result.summary().contains("Crash recovery"));
    }

    #[test]
    fn test_result_summary_failures() {
        let result = RestorationResult {
            sessions: vec![SessionRestorationResult {
                session_id: Uuid::new_v4(),
                session_name: "test".to_string(),
                windows_restored: 1,
                pane_results: vec![PaneRestorationResult {
                    pane_id: Uuid::new_v4(),
                    pty_attempted: true,
                    pty_spawned: false,
                    error: Some("spawn failed".to_string()),
                    cwd_restored: false,
                    claude_resumed: false,
                    claude_session_id: None,
                }],
            }],
            total_panes: 1,
            successful_ptys: 0,
            failed_ptys: 1,
            was_crash_recovery: false,
        };

        assert!(!result.all_successful());
        assert!(result.summary().contains("1 PTY failures"));
    }

    #[test]
    fn test_restore_exited_pane_no_pty() {
        let restorer = SessionRestorer::new(); // PTY spawning enabled

        let session_id = Uuid::new_v4();
        let window_id = Uuid::new_v4();
        let pane_id = Uuid::new_v4();

        let snapshot = SessionSnapshot {
            id: session_id,
            name: "test".to_string(),
            windows: vec![WindowSnapshot {
                id: window_id,
                session_id,
                name: "main".to_string(),
                index: 0,
                panes: vec![PaneSnapshot {
                    id: pane_id,
                    window_id,
                    index: 0,
                    cols: 80,
                    rows: 24,
                    state: PaneState::Exited { code: Some(0) },
                    name: None,
                    title: None,
                    cwd: None,
                    created_at: 0,
                    scrollback: None,
                }],
                active_pane_id: Some(pane_id),
                created_at: 0,
            }],
            active_window_id: Some(window_id),
            created_at: 0,
            metadata: HashMap::new(),
            environment: HashMap::new(),
        };

        let state = RecoveryState {
            sessions: vec![snapshot],
            clean_shutdown: true,
            ..Default::default()
        };

        let mut session_manager = SessionManager::new();
        let mut pty_manager = PtyManager::new();

        let result = restorer.restore(&state, &mut session_manager, &mut pty_manager);

        // Exited pane should not spawn PTY
        assert_eq!(result.total_panes, 1);
        assert_eq!(result.successful_ptys, 0);
        assert_eq!(result.failed_ptys, 0); // Not a failure, just not attempted
        assert!(!result.sessions[0].pane_results[0].pty_spawned);
    }

    #[test]
    fn test_restore_claude_pane_with_session_id() {
        let restorer = SessionRestorer::without_pty_spawn();

        let session_id_uuid = Uuid::new_v4();
        let window_id = Uuid::new_v4();
        let pane_id = Uuid::new_v4();
        let claude_session_id = "test-claude-session-123".to_string();

        let snapshot = SessionSnapshot {
            id: session_id_uuid,
            name: "claude-test".to_string(),
            windows: vec![WindowSnapshot {
                id: window_id,
                session_id: session_id_uuid,
                name: "main".to_string(),
                index: 0,
                panes: vec![PaneSnapshot {
                    id: pane_id,
                    window_id,
                    index: 0,
                    cols: 80,
                    rows: 24,
                    state: PaneState::Agent({
                        let mut state = AgentState::new("claude")
                            .with_activity(AgentActivity::Idle);
                        state.session_id = Some(claude_session_id.clone());
                        state.set_metadata("model", serde_json::Value::String("claude-3-opus".to_string()));
                        state.set_metadata("tokens_used", serde_json::Value::Number(5000.into()));
                        state
                    }),
                    name: None,
                    title: Some("Claude".to_string()),
                    cwd: Some("/tmp".to_string()),
                    created_at: 12345,
                    scrollback: None,
                }],
                active_pane_id: Some(pane_id),
                created_at: 12345,
            }],
            active_window_id: Some(window_id),
            created_at: 12345,
            metadata: HashMap::new(),
            environment: HashMap::new(),
        };

        let state = RecoveryState {
            sessions: vec![snapshot],
            clean_shutdown: false, // Crash recovery
            ..Default::default()
        };

        let mut session_manager = SessionManager::new();
        let mut pty_manager = PtyManager::new();

        let result = restorer.restore(&state, &mut session_manager, &mut pty_manager);

        // Check restoration result
        assert_eq!(result.total_panes, 1);
        assert!(result.was_crash_recovery);

        // Check Claude session resume was marked
        let pane_result = &result.sessions[0].pane_results[0];
        assert!(pane_result.claude_resumed);
        assert_eq!(pane_result.claude_session_id, Some(claude_session_id));

        // Check pane was restored with Claude state
        let (_, _, pane) = session_manager.find_pane(pane_id).unwrap();
        assert!(pane.is_claude());
        let claude_state = pane.claude_state().unwrap();
        assert!(claude_state.session_id.is_some());
    }

    #[test]
    fn test_restore_claude_pane_without_session_id() {
        let restorer = SessionRestorer::without_pty_spawn();

        let session_id_uuid = Uuid::new_v4();
        let window_id = Uuid::new_v4();
        let pane_id = Uuid::new_v4();

        let snapshot = SessionSnapshot {
            id: session_id_uuid,
            name: "claude-test".to_string(),
            windows: vec![WindowSnapshot {
                id: window_id,
                session_id: session_id_uuid,
                name: "main".to_string(),
                index: 0,
                panes: vec![PaneSnapshot {
                    id: pane_id,
                    window_id,
                    index: 0,
                    cols: 80,
                    rows: 24,
                    state: PaneState::Agent(
                        AgentState::new("claude").with_activity(AgentActivity::Idle)
                    ),
                    name: None,
                    title: None,
                    cwd: None,
                    created_at: 0,
                    scrollback: None,
                }],
                active_pane_id: Some(pane_id),
                created_at: 0,
            }],
            active_window_id: Some(window_id),
            created_at: 0,
            metadata: HashMap::new(),
            environment: HashMap::new(),
        };

        let state = RecoveryState {
            sessions: vec![snapshot],
            clean_shutdown: true,
            ..Default::default()
        };

        let mut session_manager = SessionManager::new();
        let mut pty_manager = PtyManager::new();

        let result = restorer.restore(&state, &mut session_manager, &mut pty_manager);

        // Claude without session ID should NOT be resumed
        let pane_result = &result.sessions[0].pane_results[0];
        assert!(!pane_result.claude_resumed);
        assert!(pane_result.claude_session_id.is_none());
    }

    #[test]
    fn test_restore_preserves_metadata() {
        let restorer = SessionRestorer::without_pty_spawn();

        let session_id = Uuid::new_v4();
        let window_id = Uuid::new_v4();
        let pane_id = Uuid::new_v4();

        // Create metadata to persist
        let mut metadata = HashMap::new();
        metadata.insert("qa.tester".to_string(), "claude".to_string());
        metadata.insert("beads.root".to_string(), "/path/to/beads".to_string());

        let snapshot = SessionSnapshot {
            id: session_id,
            name: "test-session".to_string(),
            windows: vec![WindowSnapshot {
                id: window_id,
                session_id,
                name: "main".to_string(),
                index: 0,
                panes: vec![PaneSnapshot {
                    id: pane_id,
                    window_id,
                    index: 0,
                    cols: 80,
                    rows: 24,
                    state: PaneState::Normal,
                    name: None,
                    title: None,
                    cwd: None,
                    created_at: 12345,
                    scrollback: None,
                }],
                active_pane_id: Some(pane_id),
                created_at: 12345,
            }],
            active_window_id: Some(window_id),
            created_at: 12345,
            metadata,
            environment: HashMap::new(),
        };

        let state = RecoveryState {
            sessions: vec![snapshot],
            clean_shutdown: true,
            ..Default::default()
        };

        let mut session_manager = SessionManager::new();
        let mut pty_manager = PtyManager::new();

        let result = restorer.restore(&state, &mut session_manager, &mut pty_manager);

        assert_eq!(result.sessions.len(), 1);
        assert_eq!(result.total_panes, 1);

        // Check session was added with metadata
        let session = session_manager.get_session(session_id);
        assert!(session.is_some());
        let session = session.unwrap();
        assert_eq!(session.get_metadata("qa.tester"), Some(&"claude".to_string()));
        assert_eq!(session.get_metadata("beads.root"), Some(&"/path/to/beads".to_string()));
    }
}
