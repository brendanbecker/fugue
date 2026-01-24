//! Claude Agent Detector (FEAT-084)
//!
//! This module provides the `ClaudeAgentDetector` which wraps the existing
//! `ClaudeDetector` to implement the generic `AgentDetector` trait.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use fugue_protocol::{AgentActivity, AgentState, JsonValue};

use crate::claude::ClaudeDetector;

use super::AgentDetector;

/// Debounce duration for state change broadcasts (BUG-048)
///
/// This prevents TUI flicker caused by rapid spinner animation frames
/// that may trigger state changes faster than the inner detector's debounce.
const STATE_BROADCAST_DEBOUNCE_MS: u64 = 100;

/// Claude Code agent detector implementing the AgentDetector trait
///
/// This is a wrapper around the existing `ClaudeDetector` that adapts it
/// to the generic `AgentDetector` interface.
pub struct ClaudeAgentDetector {
    /// The underlying Claude detector
    inner: ClaudeDetector,
    /// Last time a state change was broadcast (for debouncing spinner flicker)
    last_state_broadcast: Option<Instant>,
    /// Debounce duration for state broadcasts
    broadcast_debounce: Duration,
}

impl ClaudeAgentDetector {
    /// Create a new Claude agent detector
    pub fn new() -> Self {
        Self {
            inner: ClaudeDetector::new(),
            last_state_broadcast: None,
            broadcast_debounce: Duration::from_millis(STATE_BROADCAST_DEBOUNCE_MS),
        }
    }

    /// Create a detector with custom debounce duration
    ///
    /// This sets the debounce for both the inner detector and the wrapper-level
    /// broadcast debounce.
    #[allow(dead_code)] // Used in tests and available for future use
    pub fn with_debounce(debounce_ms: u64) -> Self {
        Self {
            inner: ClaudeDetector::with_debounce(debounce_ms),
            last_state_broadcast: None,
            broadcast_debounce: Duration::from_millis(debounce_ms),
        }
    }

    /// Get a reference to the underlying ClaudeDetector
    #[allow(dead_code)] // API surface for accessing underlying detector
    pub fn inner(&self) -> &ClaudeDetector {
        &self.inner
    }

    /// Get a mutable reference to the underlying ClaudeDetector
    #[allow(dead_code)] // API surface for accessing underlying detector
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

    fn detect_activity(&self, _text: &str) -> Option<AgentActivity> {
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
        self.last_state_broadcast = None;
    }

    fn mark_as_active(&mut self) {
        self.inner.mark_as_claude();
    }

    fn analyze(&mut self, text: &str) -> Option<AgentState> {
        // Track whether Claude was already detected before this analyze call
        let was_active = self.inner.is_claude();

        // Analyze with inner detector (returns Some only on activity state changes)
        let state_changed = self.inner.analyze(text).is_some();

        // Determine if we should broadcast a state update
        let just_detected = !was_active && self.inner.is_claude();

        // BUG-048: Apply wrapper-level debounce to prevent spinner-induced flicker
        // Initial detection is never debounced to ensure responsive first-time detection.
        // Subsequent state changes are debounced to prevent rapid spinner animation
        // frames from causing excessive TUI redraws.
        if just_detected {
            // First detection - broadcast immediately without debounce
            self.last_state_broadcast = Some(Instant::now());
            self.state()
        } else if state_changed {
            // State change detected - check wrapper-level debounce
            let should_broadcast = match self.last_state_broadcast {
                None => true, // No previous broadcast, allow it
                Some(last) => last.elapsed() > self.broadcast_debounce,
            };

            if should_broadcast {
                self.last_state_broadcast = Some(Instant::now());
                self.state()
            } else {
                // Debounced - suppress this state change broadcast
                None
            }
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

    #[test]
    fn test_spinner_debounce_rapid_state_changes() {
        // BUG-048: Rapid spinner animation frames should be debounced at wrapper level
        // Use a 50ms debounce for testing
        let mut detector = ClaudeAgentDetector::with_debounce(50);

        // Initial detection - should return Some
        let state = detector.analyze("Welcome to Claude Code v1.0");
        assert!(state.is_some(), "Initial detection should not be debounced");

        // First state change to thinking - should return Some (initial detection sets timer)
        std::thread::sleep(std::time::Duration::from_millis(60));
        let state = detector.analyze("\r\u{280b} Thinking...");
        assert!(state.is_some(), "First state change after debounce should return Some");

        // Rapid subsequent state change (within debounce window) - should be debounced
        let state = detector.analyze("\r\u{280b} Writing code...");
        assert!(state.is_none(), "Rapid state change should be debounced");

        // Wait for debounce window to pass
        std::thread::sleep(std::time::Duration::from_millis(60));

        // Now state change should go through
        let state = detector.analyze("\r\u{280b} Writing code...");
        assert!(state.is_some(), "State change after debounce window should return Some");
    }

    #[test]
    fn test_initial_detection_not_debounced() {
        // BUG-048: Initial detection should never be debounced
        let mut detector = ClaudeAgentDetector::with_debounce(1000); // Long debounce

        // First detection should always return Some, regardless of debounce setting
        let state = detector.analyze("Welcome to Claude Code v1.0");
        assert!(state.is_some(), "Initial detection should never be debounced");
        assert_eq!(state.unwrap().agent_type, "claude");
    }

    #[test]
    fn test_reset_clears_debounce_timer() {
        // Reset should clear the debounce timer
        let mut detector = ClaudeAgentDetector::with_debounce(50);

        // Detect Claude
        let state = detector.analyze("Welcome to Claude Code v1.0");
        assert!(state.is_some());

        // Reset
        detector.reset();

        // After reset, next detection should work immediately (no debounce)
        let state = detector.analyze("Welcome to Claude Code v1.0");
        assert!(state.is_some(), "After reset, detection should not be debounced");
    }

    #[test]
    fn test_debounce_does_not_affect_same_state_suppression() {
        // Even with zero debounce, same state should still return None
        let mut detector = ClaudeAgentDetector::with_debounce(0);

        // Detect Claude
        detector.analyze("Welcome to Claude Code v1.0");

        // Transition to thinking
        let state = detector.analyze("\r\u{280b} Thinking...");
        assert!(state.is_some());

        // Same thinking state - should return None (inner detector suppresses, not debounce)
        let state = detector.analyze("\r\u{280b} Thinking...");
        assert!(state.is_none(), "Same state should return None regardless of debounce");
    }
}
