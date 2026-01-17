//! Claude Agent Detector (FEAT-084)
//!
//! This module provides the `ClaudeAgentDetector` which wraps the existing
//! `ClaudeDetector` to implement the generic `AgentDetector` trait.

use std::collections::HashMap;

use ccmux_protocol::{AgentActivity, AgentState, JsonValue};

use crate::claude::ClaudeDetector;

use super::AgentDetector;

/// Claude Code agent detector implementing the AgentDetector trait
///
/// This is a wrapper around the existing `ClaudeDetector` that adapts it
/// to the generic `AgentDetector` interface.
pub struct ClaudeAgentDetector {
    /// The underlying Claude detector
    inner: ClaudeDetector,
}

impl ClaudeAgentDetector {
    /// Create a new Claude agent detector
    pub fn new() -> Self {
        Self {
            inner: ClaudeDetector::new(),
        }
    }

    /// Create a detector with custom debounce duration
    pub fn with_debounce(debounce_ms: u64) -> Self {
        Self {
            inner: ClaudeDetector::with_debounce(debounce_ms),
        }
    }

    /// Get a reference to the underlying ClaudeDetector
    pub fn inner(&self) -> &ClaudeDetector {
        &self.inner
    }

    /// Get a mutable reference to the underlying ClaudeDetector
    pub fn inner_mut(&mut self) -> &mut ClaudeDetector {
        &mut self.inner
    }
}

impl Default for ClaudeAgentDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl AgentDetector for ClaudeAgentDetector {
    fn agent_type(&self) -> &'static str {
        "claude"
    }

    fn detect_presence(&mut self, text: &str) -> bool {
        // Use analyze() to trigger presence detection
        // The inner detector updates is_claude flag internally
        self.inner.analyze(text);
        self.inner.is_claude()
    }

    fn detect_activity(&self, text: &str) -> Option<AgentActivity> {
        if !self.inner.is_claude() {
            return None;
        }

        // The inner detector's activity is updated during analyze()
        // Return the current activity converted to AgentActivity
        Some(self.inner.activity().clone().into())
    }

    fn extract_session_id(&mut self, text: &str) -> Option<String> {
        // Session ID extraction happens during analyze()
        self.inner.analyze(text);
        self.inner.session_id().map(|s| s.to_string())
    }

    fn extract_metadata(&mut self, text: &str) -> HashMap<String, JsonValue> {
        // Metadata extraction happens during analyze()
        self.inner.analyze(text);

        let mut metadata = HashMap::new();

        if let Some(model) = self.inner.model() {
            metadata.insert(
                "model".to_string(),
                JsonValue::new(serde_json::Value::String(model.to_string())),
            );
        }

        // Note: tokens_used is rarely available from PTY output
        // but the detector has infrastructure for it

        metadata
    }

    fn confidence(&self) -> u8 {
        self.inner.confidence()
    }

    fn is_active(&self) -> bool {
        self.inner.is_claude()
    }

    fn state(&self) -> Option<AgentState> {
        if !self.inner.is_claude() {
            return None;
        }

        // Convert ClaudeState to AgentState
        self.inner.state().map(|claude_state| claude_state.into())
    }

    fn reset(&mut self) {
        self.inner.reset();
    }

    fn mark_as_active(&mut self) {
        self.inner.mark_as_claude();
    }

    fn analyze(&mut self, text: &str) -> Option<AgentState> {
        // Track whether Claude was already detected before this analyze call
        let was_active = self.inner.is_claude();

        // Analyze with inner detector (returns Some only on activity state changes)
        let state_changed = self.inner.analyze(text).is_some();

        // Return state if:
        // 1. Claude was just detected (transition from not-active to active), OR
        // 2. Activity state changed (inner detector returned Some)
        // This prevents excessive PaneStateChanged broadcasts (BUG-048)
        if (!was_active && self.inner.is_claude()) || state_changed {
            self.state()
        } else {
            None
        }
    }
}

impl std::fmt::Debug for ClaudeAgentDetector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClaudeAgentDetector")
            .field("is_active", &self.inner.is_claude())
            .field("confidence", &self.inner.confidence())
            .field("session_id", &self.inner.session_id())
            .field("model", &self.inner.model())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_claude_agent_detector_new() {
        let detector = ClaudeAgentDetector::new();
        assert_eq!(detector.agent_type(), "claude");
        assert!(!detector.is_active());
        assert_eq!(detector.confidence(), 0);
    }

    #[test]
    fn test_claude_agent_detector_detect_presence() {
        let mut detector = ClaudeAgentDetector::new();

        let detected = detector.detect_presence("Welcome to Claude Code v1.0");
        assert!(detected);
        assert!(detector.is_active());
    }

    #[test]
    fn test_claude_agent_detector_state() {
        let mut detector = ClaudeAgentDetector::new();

        // Not active yet
        assert!(detector.state().is_none());

        // Detect Claude
        detector.detect_presence("Welcome to Claude Code v1.0");

        let state = detector.state();
        assert!(state.is_some());

        let state = state.unwrap();
        assert_eq!(state.agent_type, "claude");
        assert_eq!(state.activity, AgentActivity::Idle);
    }

    #[test]
    fn test_claude_agent_detector_mark_as_active() {
        let mut detector = ClaudeAgentDetector::new();

        detector.mark_as_active();
        assert!(detector.is_active());
        assert_eq!(detector.confidence(), 100);
    }

    #[test]
    fn test_claude_agent_detector_reset() {
        let mut detector = ClaudeAgentDetector::new();

        detector.mark_as_active();
        assert!(detector.is_active());

        detector.reset();
        assert!(!detector.is_active());
        assert_eq!(detector.confidence(), 0);
    }

    #[test]
    fn test_claude_agent_detector_analyze() {
        let mut detector = ClaudeAgentDetector::new();

        // No detection yet
        let state = detector.analyze("Hello world");
        assert!(state.is_none());

        // Detect Claude
        let state = detector.analyze("Claude Code session started");
        assert!(state.is_some());
        assert_eq!(state.unwrap().agent_type, "claude");
    }

    #[test]
    fn test_claude_agent_detector_activity_conversion() {
        let mut detector = ClaudeAgentDetector::new();
        detector.mark_as_active();

        // Analyze thinking state
        detector.analyze("\r\u{280b} Thinking...");

        let activity = detector.detect_activity("");
        assert!(activity.is_some());
        // ClaudeActivity::Thinking maps to AgentActivity::Processing
        assert_eq!(activity.unwrap(), AgentActivity::Processing);
    }

    #[test]
    fn test_claude_agent_detector_metadata() {
        let mut detector = ClaudeAgentDetector::new();
        detector.mark_as_active();

        // Analyze with model info
        let metadata = detector.extract_metadata("Model: claude-3-opus");

        assert!(metadata.contains_key("model"));
    }

    #[test]
    fn test_analyze_returns_none_on_repeated_calls_without_change() {
        // BUG-048: analyze() should return None when no state change occurs
        // Use zero debounce for accurate testing
        let mut detector = ClaudeAgentDetector::with_debounce(0);

        // First call detects Claude - should return Some
        let state = detector.analyze("Welcome to Claude Code v1.0");
        assert!(state.is_some(), "First detection should return Some");

        // Subsequent calls with no state change should return None
        let state = detector.analyze("some random text");
        assert!(state.is_none(), "Repeated call without change should return None");

        let state = detector.analyze("more text");
        assert!(state.is_none(), "Another call without change should return None");
    }

    #[test]
    fn test_analyze_returns_some_only_on_state_transitions() {
        // BUG-048: analyze() should only return Some when state actually changes
        // Use zero debounce for accurate testing
        let mut detector = ClaudeAgentDetector::with_debounce(0);

        // Initial detection
        let state = detector.analyze("Welcome to Claude Code v1.0");
        assert!(state.is_some(), "Initial detection should return Some");

        // No change
        let state = detector.analyze("idle text");
        assert!(state.is_none(), "No change should return None");

        // Transition to thinking/processing
        let state = detector.analyze("\r\u{280b} Thinking...");
        assert!(state.is_some(), "State transition should return Some");

        // Still thinking - no change
        let state = detector.analyze("\r\u{280b} Thinking...");
        assert!(state.is_none(), "Same state should return None");
    }
}
