//! Claude Code state types
//!
//! Defines state types and events for Claude Code detection.
//! The main types (ClaudeState, ClaudeActivity) are defined in ccmux-protocol
//! for sharing between server and client. This module provides additional
//! types for state change events.

use ccmux_protocol::{ClaudeActivity, ClaudeState};
use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime};

/// Event emitted when Claude state changes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeStateChange {
    /// Previous activity state
    pub previous: ClaudeActivity,
    /// New activity state
    pub current: ClaudeActivity,
    /// Full Claude state after the change
    pub state: ClaudeState,
    /// Unix timestamp when the change occurred
    pub timestamp: u64,
    /// Human-readable description of the change
    pub description: String,
}

impl ClaudeStateChange {
    /// Create a new state change event
    pub fn new(previous: ClaudeActivity, current: ClaudeActivity, state: ClaudeState) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let description = Self::describe_transition(&previous, &current);

        Self {
            previous,
            current,
            state,
            timestamp,
            description,
        }
    }

    /// Generate a human-readable description of the state transition
    fn describe_transition(from: &ClaudeActivity, to: &ClaudeActivity) -> String {
        match (from, to) {
            (ClaudeActivity::Idle, ClaudeActivity::Thinking) => {
                "Started processing request".to_string()
            }
            (ClaudeActivity::Thinking, ClaudeActivity::Coding) => {
                "Started writing code".to_string()
            }
            (ClaudeActivity::Thinking, ClaudeActivity::ToolUse) => {
                "Started executing tool".to_string()
            }
            (ClaudeActivity::Coding, ClaudeActivity::ToolUse) => {
                "Switched to tool execution".to_string()
            }
            (ClaudeActivity::ToolUse, ClaudeActivity::Thinking) => {
                "Tool complete, continuing to think".to_string()
            }
            (ClaudeActivity::ToolUse, ClaudeActivity::AwaitingConfirmation) => {
                "Tool requires confirmation".to_string()
            }
            (_, ClaudeActivity::AwaitingConfirmation) => "Awaiting user confirmation".to_string(),
            (_, ClaudeActivity::Idle) => "Returned to idle".to_string(),
            _ => format!("{:?} -> {:?}", from, to),
        }
    }

    /// Check if this is a significant state change (not just noise)
    pub fn is_significant(&self) -> bool {
        // Any change to/from AwaitingConfirmation is significant
        if self.previous == ClaudeActivity::AwaitingConfirmation
            || self.current == ClaudeActivity::AwaitingConfirmation
        {
            return true;
        }

        // Transitions to Idle are significant (task completion)
        if self.current == ClaudeActivity::Idle && self.previous != ClaudeActivity::Idle {
            return true;
        }

        // Starting work (Idle -> anything) is significant
        if self.previous == ClaudeActivity::Idle && self.current != ClaudeActivity::Idle {
            return true;
        }

        // Other transitions are less significant but still reported
        true
    }
}

/// Session information extracted from Claude Code output
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ClaudeSessionInfo {
    /// Session ID (UUID format)
    pub session_id: Option<String>,
    /// Model being used
    pub model: Option<String>,
    /// Token usage if available (extracted from output)
    pub tokens_used: Option<u64>,
    /// Time when session was first detected
    pub detected_at: Option<u64>,
    /// Whether this session info has been confirmed by multiple signals
    pub confidence: u8,
}

impl ClaudeSessionInfo {
    /// Create a new session info
    pub fn new() -> Self {
        Self::default()
    }

    /// Update session ID
    pub fn set_session_id(&mut self, id: String) {
        self.session_id = Some(id);
        if self.detected_at.is_none() {
            self.detected_at = Some(
                SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0),
            );
        }
    }

    /// Update model
    pub fn set_model(&mut self, model: String) {
        self.model = Some(model);
    }

    /// Update token usage
    /// Reserved for future use when token info becomes available in PTY output
    #[allow(dead_code)]
    pub fn set_tokens(&mut self, tokens: u64) {
        self.tokens_used = Some(tokens);
    }

    /// Convert to ClaudeState with the given activity
    pub fn to_claude_state(&self, activity: ClaudeActivity) -> ClaudeState {
        ClaudeState {
            session_id: self.session_id.clone(),
            activity,
            model: self.model.clone(),
            tokens_used: self.tokens_used,
        }
    }
}

/// Configuration for state detection timing
#[derive(Debug, Clone)]
pub struct DetectorConfig {
    /// Minimum time between state changes (debounce)
    pub debounce_duration: Duration,
    /// Timeout after which active states decay to idle
    pub idle_timeout: Duration,
    /// Whether to log state transitions
    pub log_transitions: bool,
}

impl Default for DetectorConfig {
    fn default() -> Self {
        Self {
            debounce_duration: Duration::from_millis(100),
            idle_timeout: Duration::from_secs(60),
            log_transitions: true,
        }
    }
}

impl DetectorConfig {
    /// Create a new config with custom debounce
    pub fn with_debounce(debounce_ms: u64) -> Self {
        Self {
            debounce_duration: Duration::from_millis(debounce_ms),
            ..Self::default()
        }
    }

    /// Enable or disable transition logging
    pub fn with_logging(mut self, enabled: bool) -> Self {
        self.log_transitions = enabled;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_change_creation() {
        let state = ClaudeState::default();
        let change = ClaudeStateChange::new(
            ClaudeActivity::Idle,
            ClaudeActivity::Thinking,
            state.clone(),
        );

        assert_eq!(change.previous, ClaudeActivity::Idle);
        assert_eq!(change.current, ClaudeActivity::Thinking);
        assert!(change.timestamp > 0);
        assert!(!change.description.is_empty());
    }

    #[test]
    fn test_state_change_descriptions() {
        let state = ClaudeState::default();

        let idle_to_thinking =
            ClaudeStateChange::new(ClaudeActivity::Idle, ClaudeActivity::Thinking, state.clone());
        assert!(idle_to_thinking.description.contains("processing"));

        let thinking_to_coding = ClaudeStateChange::new(
            ClaudeActivity::Thinking,
            ClaudeActivity::Coding,
            state.clone(),
        );
        assert!(thinking_to_coding.description.contains("writing"));

        let to_confirmation = ClaudeStateChange::new(
            ClaudeActivity::Thinking,
            ClaudeActivity::AwaitingConfirmation,
            state.clone(),
        );
        assert!(to_confirmation.description.contains("confirmation"));
    }

    #[test]
    fn test_state_change_is_significant() {
        let state = ClaudeState::default();

        // Idle -> Thinking is significant
        let change =
            ClaudeStateChange::new(ClaudeActivity::Idle, ClaudeActivity::Thinking, state.clone());
        assert!(change.is_significant());

        // To AwaitingConfirmation is significant
        let change = ClaudeStateChange::new(
            ClaudeActivity::Thinking,
            ClaudeActivity::AwaitingConfirmation,
            state.clone(),
        );
        assert!(change.is_significant());

        // From AwaitingConfirmation is significant
        let change = ClaudeStateChange::new(
            ClaudeActivity::AwaitingConfirmation,
            ClaudeActivity::Thinking,
            state.clone(),
        );
        assert!(change.is_significant());

        // Back to Idle is significant
        let change =
            ClaudeStateChange::new(ClaudeActivity::Thinking, ClaudeActivity::Idle, state.clone());
        assert!(change.is_significant());
    }

    #[test]
    fn test_session_info_default() {
        let info = ClaudeSessionInfo::new();
        assert!(info.session_id.is_none());
        assert!(info.model.is_none());
        assert!(info.tokens_used.is_none());
    }

    #[test]
    fn test_session_info_setters() {
        let mut info = ClaudeSessionInfo::new();

        info.set_session_id("test-session".to_string());
        assert_eq!(info.session_id, Some("test-session".to_string()));
        assert!(info.detected_at.is_some());

        info.set_model("claude-3-opus".to_string());
        assert_eq!(info.model, Some("claude-3-opus".to_string()));

        info.set_tokens(5000);
        assert_eq!(info.tokens_used, Some(5000));
    }

    #[test]
    fn test_session_info_to_claude_state() {
        let mut info = ClaudeSessionInfo::new();
        info.set_session_id("test-id".to_string());
        info.set_model("claude-3-opus".to_string());
        info.set_tokens(1000);

        let state = info.to_claude_state(ClaudeActivity::Coding);

        assert_eq!(state.session_id, Some("test-id".to_string()));
        assert_eq!(state.model, Some("claude-3-opus".to_string()));
        assert_eq!(state.tokens_used, Some(1000));
        assert_eq!(state.activity, ClaudeActivity::Coding);
    }

    #[test]
    fn test_detector_config_default() {
        let config = DetectorConfig::default();

        assert_eq!(config.debounce_duration, Duration::from_millis(100));
        assert_eq!(config.idle_timeout, Duration::from_secs(60));
        assert!(config.log_transitions);
    }

    #[test]
    fn test_detector_config_with_debounce() {
        let config = DetectorConfig::with_debounce(500);
        assert_eq!(config.debounce_duration, Duration::from_millis(500));
    }

    #[test]
    fn test_detector_config_with_logging() {
        let config = DetectorConfig::default().with_logging(false);
        assert!(!config.log_transitions);
    }
}
