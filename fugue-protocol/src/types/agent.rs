use super::common::JsonValue;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ==================== Generic Agent System (FEAT-084) ====================

/// Generic agent state for any AI coding assistant (FEAT-084)
///
/// This type generalizes `ClaudeState` to work with any AI agent by introducing
/// a common interface for agent detection and state tracking.
///
/// # Examples
/// ```rust
/// use fugue_protocol::types::agent::{AgentState, AgentActivity};
/// use std::collections::HashMap;
///
/// let state = AgentState::new("claude")
///     .with_activity(AgentActivity::Processing)
///     .with_session_id("abc123".to_string());
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AgentState {
    /// Type identifier for the agent (e.g., "claude", "copilot", "aider")
    pub agent_type: String,
    /// Agent session ID if available
    pub session_id: Option<String>,
    /// Current activity state
    pub activity: AgentActivity,
    /// Arbitrary metadata (model, tokens, etc.)
    #[serde(default)]
    pub metadata: HashMap<String, JsonValue>,
}

impl AgentState {
    /// Create a new agent state with the given type
    pub fn new(agent_type: impl Into<String>) -> Self {
        Self {
            agent_type: agent_type.into(),
            session_id: None,
            activity: AgentActivity::Idle,
            metadata: HashMap::new(),
        }
    }

    /// Set the activity (builder pattern)
    pub fn with_activity(mut self, activity: AgentActivity) -> Self {
        self.activity = activity;
        self
    }

    /// Set the session ID (builder pattern)
    pub fn with_session_id(mut self, session_id: String) -> Self {
        self.session_id = Some(session_id);
        self
    }

    /// Set a metadata value (builder pattern)
    pub fn with_metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), JsonValue::new(value));
        self
    }

    /// Get a metadata value
    pub fn get_metadata(&self, key: &str) -> Option<&serde_json::Value> {
        self.metadata.get(key).map(|v| v.inner())
    }

    /// Set a metadata value
    pub fn set_metadata(&mut self, key: impl Into<String>, value: serde_json::Value) {
        self.metadata.insert(key.into(), JsonValue::new(value));
    }

    /// Check if this is a specific agent type
    pub fn is_agent_type(&self, agent_type: &str) -> bool {
        self.agent_type == agent_type
    }

    /// Check if this is Claude
    pub fn is_claude(&self) -> bool {
        self.agent_type == "claude"
    }
}

impl Default for AgentState {
    fn default() -> Self {
        Self::new("unknown")
    }
}

/// Generic agent activity states (FEAT-084)
///
/// This generalizes `ClaudeActivity` to work with any AI agent.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum AgentActivity {
    /// Waiting for input
    #[default]
    Idle,
    /// Processing/thinking (replaces ClaudeActivity::Thinking)
    Processing,
    /// Generating content (replaces ClaudeActivity::Coding)
    Generating,
    /// Executing tools (same as ClaudeActivity::ToolUse)
    ToolUse,
    /// Waiting for user confirmation (same as ClaudeActivity::AwaitingConfirmation)
    AwaitingConfirmation,
    /// Agent-specific custom state
    Custom(String),
}

impl AgentActivity {
    /// Check if the activity is an active (non-idle) state
    pub fn is_active(&self) -> bool {
        !matches!(self, AgentActivity::Idle)
    }
}

/// Claude Code specific state
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ClaudeState {
    /// Claude session ID if detected
    pub session_id: Option<String>,
    /// Current activity state
    pub activity: ClaudeActivity,
    /// Model being used
    pub model: Option<String>,
    /// Token usage if available
    pub tokens_used: Option<u64>,
}

impl Default for ClaudeState {
    fn default() -> Self {
        Self {
            session_id: None,
            activity: ClaudeActivity::Idle,
            model: None,
            tokens_used: None,
        }
    }
}

/// Claude activity states
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ClaudeActivity {
    /// Waiting for input
    Idle,
    /// Processing/thinking
    Thinking,
    /// Writing code
    Coding,
    /// Executing tools
    ToolUse,
    /// Waiting for user confirmation
    AwaitingConfirmation,
}

// ==================== From Trait Conversions (FEAT-084) ====================

impl From<ClaudeState> for AgentState {
    fn from(claude: ClaudeState) -> Self {
        let mut state = AgentState::new("claude")
            .with_activity(claude.activity.into());

        if let Some(session_id) = claude.session_id {
            state.session_id = Some(session_id);
        }

        if let Some(model) = claude.model {
            state.set_metadata("model", serde_json::Value::String(model));
        }

        if let Some(tokens) = claude.tokens_used {
            state.set_metadata("tokens_used", serde_json::Value::Number(tokens.into()));
        }

        state
    }
}

impl From<ClaudeActivity> for AgentActivity {
    fn from(activity: ClaudeActivity) -> Self {
        match activity {
            ClaudeActivity::Idle => AgentActivity::Idle,
            ClaudeActivity::Thinking => AgentActivity::Processing,
            ClaudeActivity::Coding => AgentActivity::Generating,
            ClaudeActivity::ToolUse => AgentActivity::ToolUse,
            ClaudeActivity::AwaitingConfirmation => AgentActivity::AwaitingConfirmation,
        }
    }
}

impl From<AgentActivity> for ClaudeActivity {
    fn from(activity: AgentActivity) -> Self {
        match activity {
            AgentActivity::Idle => ClaudeActivity::Idle,
            AgentActivity::Processing => ClaudeActivity::Thinking,
            AgentActivity::Generating => ClaudeActivity::Coding,
            AgentActivity::ToolUse => ClaudeActivity::ToolUse,
            AgentActivity::AwaitingConfirmation => ClaudeActivity::AwaitingConfirmation,
            AgentActivity::Custom(_) => ClaudeActivity::Idle, // Fallback for unknown states
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== ClaudeActivity Tests ====================

    #[test]
    fn test_claude_activity_all_variants() {
        let activities = [
            ClaudeActivity::Idle,
            ClaudeActivity::Thinking,
            ClaudeActivity::Coding,
            ClaudeActivity::ToolUse,
            ClaudeActivity::AwaitingConfirmation,
        ];

        assert_eq!(activities.len(), 5);

        // All should be unique
        for (i, a) in activities.iter().enumerate() {
            for (j, b) in activities.iter().enumerate() {
                if i == j {
                    assert_eq!(a, b);
                } else {
                    assert_ne!(a, b);
                }
            }
        }
    }

    #[test]
    fn test_claude_activity_clone() {
        let activity = ClaudeActivity::Thinking;
        let cloned = activity.clone();
        assert_eq!(activity, cloned);
    }

    #[test]
    fn test_claude_activity_debug() {
        assert_eq!(format!("{:?}", ClaudeActivity::Idle), "Idle");
        assert_eq!(format!("{:?}", ClaudeActivity::Thinking), "Thinking");
        assert_eq!(format!("{:?}", ClaudeActivity::Coding), "Coding");
        assert_eq!(format!("{:?}", ClaudeActivity::ToolUse), "ToolUse");
        assert_eq!(
            format!("{:?}", ClaudeActivity::AwaitingConfirmation),
            "AwaitingConfirmation"
        );
    }

    #[test]
    fn test_claude_activity_serde() {
        for activity in [
            ClaudeActivity::Idle,
            ClaudeActivity::Thinking,
            ClaudeActivity::Coding,
            ClaudeActivity::ToolUse,
            ClaudeActivity::AwaitingConfirmation,
        ] {
            let serialized = bincode::serialize(&activity).unwrap();
            let deserialized: ClaudeActivity = bincode::deserialize(&serialized).unwrap();
            assert_eq!(activity, deserialized);
        }
    }

    // ==================== ClaudeState Tests ====================

    #[test]
    fn test_claude_state_default() {
        let state = ClaudeState::default();

        assert!(state.session_id.is_none());
        assert_eq!(state.activity, ClaudeActivity::Idle);
        assert!(state.model.is_none());
        assert!(state.tokens_used.is_none());
    }

    #[test]
    fn test_claude_state_with_all_fields() {
        let state = ClaudeState {
            session_id: Some("session-123".to_string()),
            activity: ClaudeActivity::Coding,
            model: Some("claude-3-opus".to_string()),
            tokens_used: Some(5000),
        };

        assert_eq!(state.session_id, Some("session-123".to_string()));
        assert_eq!(state.activity, ClaudeActivity::Coding);
        assert_eq!(state.model, Some("claude-3-opus".to_string()));
        assert_eq!(state.tokens_used, Some(5000));
    }

    #[test]
    fn test_claude_state_clone() {
        let state = ClaudeState {
            session_id: Some("test".to_string()),
            activity: ClaudeActivity::ToolUse,
            model: Some("claude-3-sonnet".to_string()),
            tokens_used: Some(1000),
        };

        let cloned = state.clone();
        assert_eq!(state, cloned);
    }

    #[test]
    fn test_claude_state_equality() {
        let state1 = ClaudeState::default();
        let state2 = ClaudeState::default();
        let state3 = ClaudeState {
            session_id: Some("x".to_string()),
            ..ClaudeState::default()
        };

        assert_eq!(state1, state2);
        assert_ne!(state1, state3);
    }

    #[test]
    fn test_claude_state_debug() {
        let state = ClaudeState::default();
        let debug = format!("{:?}", state);
        assert!(debug.contains("ClaudeState"));
        assert!(debug.contains("Idle"));
    }

    #[test]
    fn test_claude_state_serde() {
        let state = ClaudeState {
            session_id: Some("abc".to_string()),
            activity: ClaudeActivity::Coding,
            model: Some("claude-3".to_string()),
            tokens_used: Some(100),
        };

        let serialized = bincode::serialize(&state).unwrap();
        let deserialized: ClaudeState = bincode::deserialize(&serialized).unwrap();
        assert_eq!(state, deserialized);
    }
}
