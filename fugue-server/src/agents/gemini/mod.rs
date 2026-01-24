//! Gemini Agent Detector (FEAT-098)
//!
//! This module provides the `GeminiAgentDetector` which implements the generic
//! `AgentDetector` trait for detecting and tracking Gemini CLI sessions.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use ccmux_protocol::{AgentActivity, AgentState, JsonValue};

use super::AgentDetector;

/// Debounce duration for state change broadcasts
///
/// This prevents TUI flicker caused by rapid spinner animation frames.
const STATE_BROADCAST_DEBOUNCE_MS: u64 = 100;

/// Gemini CLI agent detector implementing the AgentDetector trait
pub struct GeminiAgentDetector {
    /// Whether Gemini has been detected
    is_active: bool,
    /// Detection confidence (0-100)
    confidence: u8,
    /// Current activity state (tracks actual state)
    current_activity: AgentActivity,
    /// Last activity that was broadcast (for debounce comparison)
    last_broadcast_activity: AgentActivity,
    /// Last time a state change was broadcast (for debouncing)
    last_state_broadcast: Option<Instant>,
    /// Debounce duration for state broadcasts
    broadcast_debounce: Duration,
    /// Detected model name (e.g., "Gemini 3")
    model: Option<String>,
}

impl GeminiAgentDetector {
    /// Create a new Gemini agent detector
    pub fn new() -> Self {
        Self {
            is_active: false,
            confidence: 0,
            current_activity: AgentActivity::Idle,
            last_broadcast_activity: AgentActivity::Idle,
            last_state_broadcast: None,
            broadcast_debounce: Duration::from_millis(STATE_BROADCAST_DEBOUNCE_MS),
            model: None,
        }
    }

    /// Create a detector with custom debounce duration
    #[allow(dead_code)]
    pub fn with_debounce(debounce_ms: u64) -> Self {
        Self {
            is_active: false,
            confidence: 0,
            current_activity: AgentActivity::Idle,
            last_broadcast_activity: AgentActivity::Idle,
            last_state_broadcast: None,
            broadcast_debounce: Duration::from_millis(debounce_ms),
            model: None,
        }
    }

    /// Check for Gemini presence indicators in text
    ///
    /// Returns true if Gemini is detected, and also sets `is_active` to true.
    fn check_presence(&mut self, text: &str) -> bool {
        // Strong indicators - high confidence
        let strong_patterns = [
            "GEMINI.md file",        // Skills display mentions GEMINI.md
            "Welcome to Gemini",     // Welcome message
            "Gemini CLI",            // CLI name
            "gemini>",               // Gemini prompt variant
        ];

        for pattern in strong_patterns {
            if text.contains(pattern) {
                self.confidence = 100;
                self.is_active = true;
                return true;
            }
        }

        // Model indicator pattern: "Auto (Gemini X)" or similar
        if text.contains("(Gemini") {
            self.confidence = 100;
            self.is_active = true;
            // Try to extract model name
            if let Some(start) = text.find("(Gemini") {
                if let Some(end) = text[start..].find(')') {
                    let model = &text[start + 1..start + end];
                    self.model = Some(model.to_string());
                }
            }
            return true;
        }

        // Medium confidence: "Gemini" alone in output
        if text.contains("Gemini") {
            // Only set if we're not already detected with higher confidence
            if self.confidence < 70 {
                self.confidence = 70;
            }
            self.is_active = true;
            return true;
        }

        // Low confidence: Gemini prompt pattern
        // "> " followed by "Type your message" text
        if text.contains("> ") && text.contains("Type your message") {
            if self.confidence < 50 {
                self.confidence = 50;
            }
            self.is_active = true;
            return true;
        }

        false
    }

    /// Detect activity state from terminal output
    ///
    /// Gemini CLI uses Unicode Braille spinners (⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏) followed by status text.
    fn detect_activity_from_text(&self, text: &str) -> AgentActivity {
        // Braille spinner characters
        let spinners = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];

        // Check for spinner presence indicating activity
        let has_spinner = spinners.iter().any(|&c| text.contains(c));

        if has_spinner {
            // Look for specific activity hints
            let text_lower = text.to_lowercase();

            if text_lower.contains("tool") || text_lower.contains("executing") {
                return AgentActivity::ToolUse;
            }

            if text_lower.contains("writing") || text_lower.contains("generating") {
                return AgentActivity::Generating;
            }

            // Generic processing (thinking, reviewing, tracking, etc.)
            return AgentActivity::Processing;
        }

        // Check for confirmation prompts
        if text.contains("[Y/n]")
            || text.contains("[y/N]")
            || text.contains("confirm")
            || text.contains("Continue?")
        {
            return AgentActivity::AwaitingConfirmation;
        }

        // Check for idle prompt: "> " at end of line or before cursor
        // The Gemini prompt is typically "> " followed by optional text
        if text.contains("\n> ") || text.ends_with("> ") || text.contains("> \x1b") {
            return AgentActivity::Idle;
        }

        // Default to current activity if no clear indicator
        self.current_activity.clone()
    }
}

impl Default for GeminiAgentDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl AgentDetector for GeminiAgentDetector {
    fn agent_type(&self) -> &'static str {
        "gemini"
    }

    fn detect_presence(&mut self, text: &str) -> bool {
        self.check_presence(text) || self.is_active
    }

    fn detect_activity(&self, text: &str) -> Option<AgentActivity> {
        if !self.is_active {
            return None;
        }
        Some(self.detect_activity_from_text(text))
    }

    fn extract_session_id(&mut self, _text: &str) -> Option<String> {
        // Gemini CLI doesn't display session IDs in the same way Claude does
        None
    }

    fn extract_metadata(&mut self, text: &str) -> HashMap<String, JsonValue> {
        let mut metadata = HashMap::new();

        // Extract model if we haven't already
        if self.model.is_none() {
            if let Some(start) = text.find("(Gemini") {
                if let Some(end) = text[start..].find(')') {
                    let model = &text[start + 1..start + end];
                    self.model = Some(model.to_string());
                }
            }
        }

        if let Some(ref model) = self.model {
            metadata.insert(
                "model".to_string(),
                JsonValue::new(serde_json::Value::String(model.clone())),
            );
        }

        // Extract skills count if visible
        if let Some(start) = text.find(" skills") {
            // Look backwards for the number
            let prefix = &text[..start];
            if let Some(num_start) = prefix.rfind(|c: char| !c.is_ascii_digit()) {
                let num_str = &prefix[num_start + 1..];
                if let Ok(count) = num_str.parse::<u32>() {
                    metadata.insert(
                        "skills_count".to_string(),
                        JsonValue::new(serde_json::Value::Number(count.into())),
                    );
                }
            }
        }

        metadata
    }

    fn confidence(&self) -> u8 {
        self.confidence
    }

    fn is_active(&self) -> bool {
        self.is_active
    }

    fn state(&self) -> Option<AgentState> {
        if !self.is_active {
            return None;
        }

        let mut state = AgentState::new("gemini");
        state.activity = self.current_activity.clone();

        if let Some(ref model) = self.model {
            state.metadata.insert(
                "model".to_string(),
                JsonValue::new(serde_json::Value::String(model.clone())),
            );
        }

        Some(state)
    }

    fn reset(&mut self) {
        self.is_active = false;
        self.confidence = 0;
        self.current_activity = AgentActivity::Idle;
        self.last_broadcast_activity = AgentActivity::Idle;
        self.last_state_broadcast = None;
        self.model = None;
    }

    fn mark_as_active(&mut self) {
        self.is_active = true;
        self.confidence = 100;
    }

    fn analyze(&mut self, text: &str) -> Option<AgentState> {
        let was_active = self.is_active;

        // Detect presence (updates is_active and confidence)
        self.check_presence(text);

        if !self.is_active {
            return None;
        }

        // Detect activity
        let new_activity = self.detect_activity_from_text(text);
        self.current_activity = new_activity.clone();

        // Extract metadata
        self.extract_metadata(text);

        // Determine if we should broadcast a state update
        let just_detected = !was_active && self.is_active;

        // Compare against last BROADCAST activity, not just previous activity.
        // This ensures that if a state change was debounced, we'll still
        // broadcast it once the debounce window passes.
        let activity_differs_from_broadcast = new_activity != self.last_broadcast_activity;

        if just_detected {
            // First detection - broadcast immediately
            self.last_state_broadcast = Some(Instant::now());
            self.last_broadcast_activity = new_activity;
            self.state()
        } else if activity_differs_from_broadcast {
            // Activity differs from last broadcast - check debounce
            let should_broadcast = match self.last_state_broadcast {
                None => true,
                Some(last) => last.elapsed() > self.broadcast_debounce,
            };

            if should_broadcast {
                self.last_state_broadcast = Some(Instant::now());
                self.last_broadcast_activity = new_activity;
                self.state()
            } else {
                None
            }
        } else {
            None
        }
    }
}

impl std::fmt::Debug for GeminiAgentDetector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GeminiAgentDetector")
            .field("is_active", &self.is_active)
            .field("confidence", &self.confidence)
            .field("current_activity", &self.current_activity)
            .field("model", &self.model)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gemini_agent_detector_new() {
        let detector = GeminiAgentDetector::new();
        assert_eq!(detector.agent_type(), "gemini");
        assert!(!detector.is_active());
        assert_eq!(detector.confidence(), 0);
    }

    #[test]
    fn test_detect_presence_gemini_md() {
        let mut detector = GeminiAgentDetector::new();

        let detected = detector.detect_presence("Loading GEMINI.md file from project");
        assert!(detected);
        assert!(detector.is_active());
        assert_eq!(detector.confidence(), 100);
    }

    #[test]
    fn test_detect_presence_welcome() {
        let mut detector = GeminiAgentDetector::new();

        let detected = detector.detect_presence("Welcome to Gemini CLI");
        assert!(detected);
        assert!(detector.is_active());
    }

    #[test]
    fn test_detect_presence_model_indicator() {
        let mut detector = GeminiAgentDetector::new();

        let detected = detector.detect_presence("Model: Auto (Gemini 3)");
        assert!(detected);
        assert!(detector.is_active());
        assert_eq!(detector.model, Some("Gemini 3".to_string()));
    }

    #[test]
    fn test_detect_presence_gemini_word() {
        let mut detector = GeminiAgentDetector::new();

        let detected = detector.detect_presence("Using Gemini for code generation");
        assert!(detected);
        // Lower confidence for just the word
        assert_eq!(detector.confidence(), 70);
    }

    #[test]
    fn test_detect_activity_spinner_processing() {
        let mut detector = GeminiAgentDetector::new();
        detector.mark_as_active();

        let activity = detector.detect_activity_from_text("⠋ Tracking Down the File");
        assert_eq!(activity, AgentActivity::Processing);

        let activity = detector.detect_activity_from_text("⠙ Reviewing Implementation Details");
        assert_eq!(activity, AgentActivity::Processing);
    }

    #[test]
    fn test_detect_activity_spinner_tool_use() {
        let mut detector = GeminiAgentDetector::new();
        detector.mark_as_active();

        let activity = detector.detect_activity_from_text("⠹ Executing tool: file_read");
        assert_eq!(activity, AgentActivity::ToolUse);
    }

    #[test]
    fn test_detect_activity_spinner_generating() {
        let mut detector = GeminiAgentDetector::new();
        detector.mark_as_active();

        let activity = detector.detect_activity_from_text("⠼ Writing code to file");
        assert_eq!(activity, AgentActivity::Generating);
    }

    #[test]
    fn test_detect_activity_idle_prompt() {
        let mut detector = GeminiAgentDetector::new();
        detector.mark_as_active();

        let activity = detector.detect_activity_from_text("Done!\n> ");
        assert_eq!(activity, AgentActivity::Idle);
    }

    #[test]
    fn test_detect_activity_confirmation() {
        let mut detector = GeminiAgentDetector::new();
        detector.mark_as_active();

        let activity = detector.detect_activity_from_text("Create this file? [Y/n]");
        assert_eq!(activity, AgentActivity::AwaitingConfirmation);
    }

    #[test]
    fn test_state_returns_none_when_inactive() {
        let detector = GeminiAgentDetector::new();
        assert!(detector.state().is_none());
    }

    #[test]
    fn test_state_returns_gemini_type() {
        let mut detector = GeminiAgentDetector::new();
        detector.mark_as_active();

        let state = detector.state();
        assert!(state.is_some());
        assert_eq!(state.unwrap().agent_type, "gemini");
    }

    #[test]
    fn test_reset_clears_state() {
        let mut detector = GeminiAgentDetector::new();
        detector.mark_as_active();
        assert!(detector.is_active());

        detector.reset();
        assert!(!detector.is_active());
        assert_eq!(detector.confidence(), 0);
        assert!(detector.model.is_none());
    }

    #[test]
    fn test_analyze_detects_gemini() {
        let mut detector = GeminiAgentDetector::new();

        let state = detector.analyze("Welcome to Gemini CLI v1.0");
        assert!(state.is_some());
        assert_eq!(state.unwrap().agent_type, "gemini");
    }

    #[test]
    fn test_analyze_returns_none_on_no_detection() {
        let mut detector = GeminiAgentDetector::new();

        let state = detector.analyze("Hello world, just a normal shell");
        assert!(state.is_none());
    }

    #[test]
    fn test_analyze_returns_none_on_repeated_calls_without_change() {
        let mut detector = GeminiAgentDetector::with_debounce(0);

        // First call detects Gemini
        let state = detector.analyze("Welcome to Gemini CLI");
        assert!(state.is_some());

        // Subsequent calls without state change
        let state = detector.analyze("some random text");
        assert!(state.is_none());
    }

    #[test]
    fn test_analyze_returns_some_on_activity_change() {
        let mut detector = GeminiAgentDetector::with_debounce(0);

        // Initial detection - returns Some for first detection
        let state = detector.analyze("Welcome to Gemini CLI");
        assert!(state.is_some());

        // Same idle state - no change, returns None
        let state = detector.analyze("\n> ");
        assert!(state.is_none(), "No activity change should return None");

        // Change to processing - returns Some
        let state = detector.analyze("⠋ Thinking...");
        assert!(state.is_some());
        assert_eq!(state.unwrap().activity, AgentActivity::Processing);

        // Back to idle - returns Some
        let state = detector.analyze("\n> ");
        assert!(state.is_some());
        assert_eq!(state.unwrap().activity, AgentActivity::Idle);
    }

    #[test]
    fn test_extract_skills_count() {
        let mut detector = GeminiAgentDetector::new();

        let metadata = detector.extract_metadata("Available: 14 skills");
        assert!(metadata.contains_key("skills_count"));
    }

    #[test]
    fn test_initial_detection_not_debounced() {
        let mut detector = GeminiAgentDetector::with_debounce(1000);

        // First detection should always succeed regardless of debounce
        let state = detector.analyze("Welcome to Gemini CLI");
        assert!(state.is_some());
    }

    #[test]
    fn test_debounce_rapid_state_changes() {
        let mut detector = GeminiAgentDetector::with_debounce(50);

        // Initial detection
        let state = detector.analyze("Welcome to Gemini CLI");
        assert!(state.is_some());

        // Wait for debounce
        std::thread::sleep(std::time::Duration::from_millis(60));

        // First activity change
        let state = detector.analyze("⠋ Thinking...");
        assert!(state.is_some());

        // Rapid subsequent change (within debounce window)
        let state = detector.analyze("\n> ");
        assert!(state.is_none()); // Should be debounced

        // Wait for debounce
        std::thread::sleep(std::time::Duration::from_millis(60));

        // Now should go through
        let state = detector.analyze("\n> ");
        assert!(state.is_some());
    }
}
