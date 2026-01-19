//! Generic Agent Detection System (FEAT-084)
//!
//! This module provides a pluggable agent detection system that generalizes
//! Claude Code detection to work with any AI coding assistant.
//!
//! # Architecture
//!
//! - `AgentDetector` trait: Interface for agent-specific detection logic
//! - `DetectorRegistry`: Manages multiple detectors and routes analysis
//! - Individual detector implementations (e.g., `ClaudeAgentDetector`)
//!
//! # Example
//!
//! ```rust,ignore
//! use ccmux_server::agents::DetectorRegistry;
//!
//! let mut registry = DetectorRegistry::with_defaults();
//!
//! // Analyze terminal output
//! if let Some(state) = registry.analyze("Welcome to Claude Code") {
//!     println!("Detected agent: {}", state.agent_type);
//! }
//! ```

pub mod claude;
pub mod gemini;

use std::collections::HashMap;
use ccmux_protocol::{AgentActivity, AgentState, JsonValue};

/// Trait for agent-specific detection logic (FEAT-084)
///
/// Implementors of this trait provide the logic to detect a specific AI agent
/// (e.g., Claude, Copilot, Aider) in terminal output and track its activity state.
pub trait AgentDetector: Send + Sync {
    /// Return the agent type identifier (e.g., "claude", "copilot", "aider")
    fn agent_type(&self) -> &'static str;

    /// Detect if this agent is present in the terminal output
    ///
    /// Returns `true` if the agent is detected, updating internal state.
    fn detect_presence(&mut self, text: &str) -> bool;

    /// Detect the current activity state from terminal output
    ///
    /// Returns `Some(activity)` if a specific activity can be determined,
    /// or `None` if the activity cannot be determined from this text.
    fn detect_activity(&self, text: &str) -> Option<AgentActivity>;

    /// Extract session ID from terminal output if available
    fn extract_session_id(&mut self, text: &str) -> Option<String>;

    /// Extract metadata from terminal output (model, tokens, etc.)
    fn extract_metadata(&mut self, text: &str) -> HashMap<String, JsonValue>;

    /// Get detection confidence (0-100)
    fn confidence(&self) -> u8;

    /// Check if this detector has detected the agent
    fn is_active(&self) -> bool;

    /// Get the current agent state if the agent is active
    fn state(&self) -> Option<AgentState>;

    /// Reset the detector state
    fn reset(&mut self);

    /// Manually mark this detector as active (for known command launches)
    fn mark_as_active(&mut self);

    /// Analyze text and return state change if any occurred
    ///
    /// This is a convenience method that combines presence detection,
    /// activity detection, and metadata extraction.
    fn analyze(&mut self, text: &str) -> Option<AgentState>;
}

/// Registry for managing multiple agent detectors (FEAT-084)
///
/// The registry maintains a list of detectors and provides a unified interface
/// for analyzing terminal output. When multiple agents could potentially be
/// detected, the registry uses confidence levels to select the best match.
#[derive(Default)]
pub struct DetectorRegistry {
    /// List of registered detectors
    detectors: Vec<Box<dyn AgentDetector>>,
    /// Currently active detector index (if any)
    active_detector: Option<usize>,
}

impl DetectorRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            detectors: Vec::new(),
            active_detector: None,
        }
    }

    /// Create a registry with default detectors (Claude, Gemini)
    pub fn with_defaults() -> Self {
        let mut registry = Self::new();
        registry.register(Box::new(claude::ClaudeAgentDetector::new()));
        registry.register(Box::new(gemini::GeminiAgentDetector::new()));
        registry
    }

    /// Register a new detector
    pub fn register(&mut self, detector: Box<dyn AgentDetector>) {
        self.detectors.push(detector);
    }

    /// Analyze terminal output and return agent state if detected
    ///
    /// This method iterates through all registered detectors and returns
    /// the state from the first one that detects an active agent.
    ///
    /// BUG-057: Once an agent is detected in a pane, we exclusively use that
    /// detector until it becomes inactive. This prevents cross-contamination
    /// where text patterns (e.g., "Gemini" appearing in a Claude conversation)
    /// could cause a different detector to "steal" detection from the active one.
    pub fn analyze(&mut self, text: &str) -> Option<AgentState> {
        // If we already have an active detector, use it exclusively
        if let Some(idx) = self.active_detector {
            if let Some(detector) = self.detectors.get_mut(idx) {
                if let Some(state) = detector.analyze(text) {
                    return Some(state);
                }
                // Check if still active
                if detector.is_active() {
                    // BUG-057: Detector is still active but no state change.
                    // Return None without trying other detectors to prevent
                    // cross-contamination from patterns that might match other agents.
                    return None;
                }
                // Detector became inactive, clear it and try others
                self.active_detector = None;
            }
        }

        // No active detector (or it became inactive), try to find one
        for (idx, detector) in self.detectors.iter_mut().enumerate() {
            if let Some(state) = detector.analyze(text) {
                self.active_detector = Some(idx);
                return Some(state);
            }
        }

        None
    }

    /// Check if any agent is currently active
    pub fn is_agent_active(&self) -> bool {
        self.active_detector.is_some()
    }

    /// Get the active agent's state if any
    pub fn active_state(&self) -> Option<AgentState> {
        self.active_detector
            .and_then(|idx| self.detectors.get(idx))
            .and_then(|d| d.state())
    }

    /// Get the active agent type if any
    pub fn active_agent_type(&self) -> Option<&'static str> {
        self.active_detector
            .and_then(|idx| self.detectors.get(idx))
            .map(|d| d.agent_type())
    }

    /// Reset all detectors
    pub fn reset(&mut self) {
        for detector in &mut self.detectors {
            detector.reset();
        }
        self.active_detector = None;
    }

    /// Mark a specific agent type as active (for known command launches)
    ///
    /// Returns `true` if the agent type was found and marked active.
    pub fn mark_as_active(&mut self, agent_type: &str) -> bool {
        for (idx, detector) in self.detectors.iter_mut().enumerate() {
            if detector.agent_type() == agent_type {
                detector.mark_as_active();
                self.active_detector = Some(idx);
                return true;
            }
        }
        false
    }

    /// Get mutable access to the active detector (if any)
    pub fn active_detector_mut(&mut self) -> Option<&mut Box<dyn AgentDetector>> {
        let idx = self.active_detector?;
        self.detectors.get_mut(idx)
    }

    /// Get a reference to all registered detectors
    pub fn detectors(&self) -> &[Box<dyn AgentDetector>] {
        &self.detectors
    }

    /// Check if this is specifically a Claude pane (backward compat)
    pub fn is_claude(&self) -> bool {
        self.active_agent_type() == Some("claude")
    }
}

impl std::fmt::Debug for DetectorRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DetectorRegistry")
            .field("detector_count", &self.detectors.len())
            .field("active_detector", &self.active_detector)
            .field("active_agent_type", &self.active_agent_type())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_new() {
        let registry = DetectorRegistry::new();
        assert!(registry.detectors.is_empty());
        assert!(!registry.is_agent_active());
    }

    #[test]
    fn test_registry_with_defaults() {
        let registry = DetectorRegistry::with_defaults();
        assert!(!registry.detectors.is_empty());
        // Should have Claude and Gemini detectors
        assert!(registry.detectors.iter().any(|d| d.agent_type() == "claude"));
        assert!(registry.detectors.iter().any(|d| d.agent_type() == "gemini"));
    }

    #[test]
    fn test_registry_analyze_detects_claude() {
        let mut registry = DetectorRegistry::with_defaults();

        let state = registry.analyze("Welcome to Claude Code v1.0");
        assert!(state.is_some());

        let state = state.unwrap();
        assert_eq!(state.agent_type, "claude");
        assert!(registry.is_agent_active());
        assert!(registry.is_claude());
    }

    #[test]
    fn test_registry_analyze_no_agent() {
        let mut registry = DetectorRegistry::with_defaults();

        let state = registry.analyze("Hello world, this is a normal shell");
        assert!(state.is_none());
        assert!(!registry.is_agent_active());
    }

    #[test]
    fn test_registry_mark_as_active() {
        let mut registry = DetectorRegistry::with_defaults();

        assert!(registry.mark_as_active("claude"));
        assert!(registry.is_agent_active());
        assert!(registry.is_claude());
    }

    #[test]
    fn test_registry_mark_as_active_unknown() {
        let mut registry = DetectorRegistry::with_defaults();

        assert!(!registry.mark_as_active("unknown_agent"));
        assert!(!registry.is_agent_active());
    }

    #[test]
    fn test_registry_reset() {
        let mut registry = DetectorRegistry::with_defaults();

        registry.mark_as_active("claude");
        assert!(registry.is_agent_active());

        registry.reset();
        assert!(!registry.is_agent_active());
    }

    #[test]
    fn test_registry_analyze_detects_gemini() {
        let mut registry = DetectorRegistry::with_defaults();

        let state = registry.analyze("Welcome to Gemini CLI");
        assert!(state.is_some());

        let state = state.unwrap();
        assert_eq!(state.agent_type, "gemini");
        assert!(registry.is_agent_active());
        assert!(!registry.is_claude());
    }

    #[test]
    fn test_registry_mark_gemini_as_active() {
        let mut registry = DetectorRegistry::with_defaults();

        assert!(registry.mark_as_active("gemini"));
        assert!(registry.is_agent_active());
        assert_eq!(registry.active_agent_type(), Some("gemini"));
        assert!(!registry.is_claude());
    }

    // BUG-057: Test that active agent detection cannot be stolen by another detector
    #[test]
    fn test_bug_057_active_detector_not_hijacked() {
        let mut registry = DetectorRegistry::with_defaults();

        // First detect Claude
        let state = registry.analyze("Welcome to Claude Code v1.0");
        assert!(state.is_some());
        assert_eq!(state.unwrap().agent_type, "claude");
        assert!(registry.is_claude());

        // Now send text that contains "Gemini" - this should NOT switch to Gemini
        // because Claude is already the active detector
        let state = registry.analyze("Let me help you understand Gemini CLI.");
        assert!(state.is_none()); // No state change expected
        assert!(registry.is_claude()); // Should STILL be Claude
        assert_eq!(registry.active_agent_type(), Some("claude"));

        // More text with Gemini references should still not switch
        let state = registry.analyze("The Gemini model is different from Claude.");
        assert!(state.is_none());
        assert!(registry.is_claude());

        // Text with Gemini spinners (braille) should also not switch
        let state = registry.analyze("⠋ Processing with Gemini");
        assert!(state.is_none());
        assert!(registry.is_claude());
    }

    // BUG-057: Test the reverse case - Gemini should not be hijacked by Claude patterns
    #[test]
    fn test_bug_057_gemini_not_hijacked_by_claude() {
        let mut registry = DetectorRegistry::with_defaults();

        // First detect Gemini
        let state = registry.analyze("Welcome to Gemini CLI");
        assert!(state.is_some());
        assert_eq!(state.unwrap().agent_type, "gemini");
        assert!(!registry.is_claude());

        // Text mentioning Claude should NOT switch detection
        let state = registry.analyze("Let me explain how Claude Code works.");
        assert!(state.is_none());
        assert!(!registry.is_claude());
        assert_eq!(registry.active_agent_type(), Some("gemini"));

        // Claude session patterns should also not switch
        let state = registry.analyze("Claude Code session started with session ID abc-123");
        assert!(state.is_none());
        assert_eq!(registry.active_agent_type(), Some("gemini"));
    }

    // BUG-057: Test that detection can switch after reset
    #[test]
    fn test_bug_057_detection_switches_after_reset() {
        let mut registry = DetectorRegistry::with_defaults();

        // Detect Claude
        registry.analyze("Welcome to Claude Code v1.0");
        assert!(registry.is_claude());

        // Reset the registry (simulating process exit/restart)
        registry.reset();
        assert!(!registry.is_agent_active());

        // Now Gemini can be detected
        let state = registry.analyze("Welcome to Gemini CLI");
        assert!(state.is_some());
        assert_eq!(state.unwrap().agent_type, "gemini");
        assert!(!registry.is_claude());
    }
}
