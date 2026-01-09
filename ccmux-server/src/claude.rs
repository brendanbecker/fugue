//! Claude Code detection module
//!
//! Detects Claude Code activity state by analyzing PTY output patterns.
//! The detector identifies when Claude is running, tracks its activity state
//! (Idle, Thinking, Coding, etc.), and extracts session IDs for crash recovery.

use ccmux_protocol::{ClaudeActivity, ClaudeState};
use std::time::{Duration, Instant};

/// Minimum time between state changes to prevent rapid flickering
const DEFAULT_DEBOUNCE_MS: u64 = 100;

/// Detector for Claude Code state in a terminal pane
#[derive(Debug)]
pub struct ClaudeDetector {
    /// Whether Claude Code is detected in this pane
    is_claude: bool,
    /// Current detected activity state
    activity: ClaudeActivity,
    /// Captured session ID (UUID format)
    session_id: Option<String>,
    /// Detected model name
    model: Option<String>,
    /// Last state change time (for debouncing)
    last_change: Instant,
    /// Minimum time between state changes
    debounce: Duration,
    /// Confidence level (0-100) in current detection
    confidence: u8,
}

impl Default for ClaudeDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl ClaudeDetector {
    /// Create a new Claude detector
    pub fn new() -> Self {
        Self {
            is_claude: false,
            activity: ClaudeActivity::Idle,
            session_id: None,
            model: None,
            last_change: Instant::now(),
            debounce: Duration::from_millis(DEFAULT_DEBOUNCE_MS),
            confidence: 0,
        }
    }

    /// Create a detector with custom debounce duration
    pub fn with_debounce(debounce_ms: u64) -> Self {
        Self {
            debounce: Duration::from_millis(debounce_ms),
            ..Self::new()
        }
    }

    /// Check if Claude is detected in this pane
    pub fn is_claude(&self) -> bool {
        self.is_claude
    }

    /// Get current activity state
    pub fn activity(&self) -> &ClaudeActivity {
        &self.activity
    }

    /// Get detected session ID
    pub fn session_id(&self) -> Option<&str> {
        self.session_id.as_deref()
    }

    /// Get detected model
    pub fn model(&self) -> Option<&str> {
        self.model.as_deref()
    }

    /// Get detection confidence (0-100)
    pub fn confidence(&self) -> u8 {
        self.confidence
    }

    /// Get current state as ClaudeState if Claude is detected
    pub fn state(&self) -> Option<ClaudeState> {
        if self.is_claude {
            Some(ClaudeState {
                session_id: self.session_id.clone(),
                activity: self.activity.clone(),
                model: self.model.clone(),
                tokens_used: None, // Not detectable from PTY output
            })
        } else {
            None
        }
    }

    /// Manually mark this pane as running Claude
    ///
    /// Useful when Claude is started via a known command.
    /// Does not trigger debouncing - subsequent analyze() calls can immediately
    /// detect state changes.
    pub fn mark_as_claude(&mut self) {
        self.is_claude = true;
        self.confidence = 100;
        // Set last_change to past so subsequent analyze() isn't debounced
        self.last_change = Instant::now() - self.debounce - Duration::from_millis(1);
    }

    /// Reset detection state
    pub fn reset(&mut self) {
        self.is_claude = false;
        self.activity = ClaudeActivity::Idle;
        self.session_id = None;
        self.model = None;
        self.confidence = 0;
        self.last_change = Instant::now();
    }

    /// Analyze terminal output and detect Claude state changes
    ///
    /// Returns `Some(ClaudeActivity)` if a state change occurred (respecting debounce),
    /// or `None` if no significant change was detected.
    pub fn analyze(&mut self, text: &str) -> Option<ClaudeActivity> {
        // Try to detect Claude presence first
        if !self.is_claude {
            if self.detect_claude_presence(text) {
                self.is_claude = true;
                self.confidence = 80;
            } else {
                return None;
            }
        }

        // Extract session ID if present
        self.extract_session_id(text);

        // Extract model if present
        self.extract_model(text);

        // Detect activity state
        let new_activity = self.detect_activity(text);

        // Apply debouncing for state changes
        if new_activity != self.activity && self.last_change.elapsed() > self.debounce {
            self.activity = new_activity.clone();
            self.last_change = Instant::now();
            Some(new_activity)
        } else {
            None
        }
    }

    /// Analyze a vt100 screen buffer for Claude state
    ///
    /// This provides more reliable detection by examining the full screen
    /// rather than incremental output.
    pub fn analyze_screen(&mut self, screen: &vt100::Screen) -> Option<ClaudeActivity> {
        let content = screen.contents();
        self.analyze(&content)
    }

    /// Check if the cursor appears to be at Claude's prompt
    pub fn is_at_prompt(&self, screen: &vt100::Screen) -> bool {
        let content = screen.contents();
        if let Some(line) = content.lines().last() {
            Self::is_prompt_line(line)
        } else {
            false
        }
    }

    /// Detect if Claude Code is present in the output
    fn detect_claude_presence(&mut self, text: &str) -> bool {
        // Strong indicators
        if text.contains("Claude Code") || text.contains("claude-code") {
            self.confidence = 95;
            return true;
        }

        // Claude startup patterns
        if text.contains("Anthropic") && text.contains("Claude") {
            self.confidence = 90;
            return true;
        }

        // Claude prompt pattern
        if Self::has_claude_prompt(text) {
            self.confidence = 75;
            return true;
        }

        // Spinner patterns typical of Claude
        if text.contains("⠋ Thinking") || text.contains("⠙ Thinking") {
            self.confidence = 85;
            return true;
        }

        false
    }

    /// Detect current activity state from text
    fn detect_activity(&self, text: &str) -> ClaudeActivity {
        // Check patterns in priority order (most specific first)

        // Permission/confirmation prompts - highest priority
        if Self::is_awaiting_confirmation(text) {
            return ClaudeActivity::AwaitingConfirmation;
        }

        // Tool execution markers
        if Self::is_tool_use(text) {
            return ClaudeActivity::ToolUse;
        }

        // Spinner patterns indicate active processing
        // Carriage return is used for spinner animation
        if text.contains('\r') || self.has_spinner_in_last_lines(text) {
            if Self::is_thinking(text) {
                return ClaudeActivity::Thinking;
            }
            if Self::is_coding(text) {
                return ClaudeActivity::Coding;
            }
        }

        // Thinking without spinner (status line or other indicators)
        if Self::is_thinking(text) {
            return ClaudeActivity::Thinking;
        }

        // Coding indicators
        if Self::is_coding(text) {
            return ClaudeActivity::Coding;
        }

        // Prompt detection (idle state)
        if Self::has_claude_prompt(text) {
            return ClaudeActivity::Idle;
        }

        // No clear indicator - maintain current state
        self.activity.clone()
    }

    /// Check for thinking state indicators
    fn is_thinking(text: &str) -> bool {
        text.contains("Thinking")
            || text.contains("thinking")
            || text.contains("Processing")
            || text.contains("Analyzing")
    }

    /// Check for coding state indicators
    fn is_coding(text: &str) -> bool {
        text.contains("Writing")
            || text.contains("Coding")
            || text.contains("Channelling")
            || text.contains("Generating")
            || text.contains("Creating file")
            || text.contains("Editing")
    }

    /// Check for tool use indicators
    fn is_tool_use(text: &str) -> bool {
        text.contains("Running:")
            || text.contains("Executing:")
            || text.contains("⚡")
            || text.contains("Read(")
            || text.contains("Write(")
            || text.contains("Edit(")
            || text.contains("Bash(")
            || text.contains("Glob(")
            || text.contains("Grep(")
    }

    /// Check for confirmation prompt indicators
    fn is_awaiting_confirmation(text: &str) -> bool {
        text.contains("[Y/n]")
            || text.contains("[y/N]")
            || text.contains("[Yes/no]")
            || text.contains("Allow?")
            || text.contains("Proceed?")
            || text.contains("Continue?")
            || text.contains("(y/n)")
    }

    /// Check if text contains a Claude prompt
    fn has_claude_prompt(text: &str) -> bool {
        // Check for prompt at end of text
        text.ends_with("> ")
            || text.ends_with("❯ ")
            // Check for prompt in content
            || text.contains("\n> ")
            || text.contains("\n❯ ")
            // Check for prompt with ANSI codes stripped (common patterns)
            || text.lines().last().map(Self::is_prompt_line).unwrap_or(false)
    }

    /// Check if a line appears to be a Claude prompt line
    fn is_prompt_line(line: &str) -> bool {
        let trimmed = line.trim();
        trimmed == ">" || trimmed == "❯" || trimmed.ends_with("> ") || trimmed.ends_with("❯ ")
    }

    /// Check for spinner characters in recent lines
    fn has_spinner_in_last_lines(&self, text: &str) -> bool {
        const SPINNER_CHARS: &[char] = &['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];

        text.lines()
            .rev()
            .take(3)
            .any(|line| SPINNER_CHARS.iter().any(|&c| line.contains(c)))
    }

    /// Extract session ID from text (UUID format)
    fn extract_session_id(&mut self, text: &str) {
        // Look for session ID in context (case-insensitive)
        let text_lower = text.to_lowercase();
        if !text_lower.contains("session") {
            return;
        }

        // Simple UUID pattern matching (avoid regex dependency)
        // Format: xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx
        for word in text.split_whitespace() {
            if Self::is_uuid_like(word) {
                self.session_id = Some(word.to_string());
                return;
            }
        }

        // Also check for UUIDs after colons (e.g., "Session: abc-123...")
        for line in text.lines() {
            if let Some(idx) = line.find(':') {
                let after_colon = line[idx + 1..].trim();
                let first_word = after_colon.split_whitespace().next().unwrap_or("");
                if Self::is_uuid_like(first_word) {
                    self.session_id = Some(first_word.to_string());
                    return;
                }
            }
        }
    }

    /// Check if a string looks like a UUID
    fn is_uuid_like(s: &str) -> bool {
        // UUID format: 8-4-4-4-12 hex characters
        let parts: Vec<&str> = s.split('-').collect();
        if parts.len() != 5 {
            return false;
        }

        let expected_lengths = [8, 4, 4, 4, 12];
        parts
            .iter()
            .zip(expected_lengths.iter())
            .all(|(part, &len)| part.len() == len && part.chars().all(|c| c.is_ascii_hexdigit()))
    }

    /// Extract model name from text
    fn extract_model(&mut self, text: &str) {
        // Look for common model patterns
        const MODEL_PATTERNS: &[&str] = &[
            "claude-3-opus",
            "claude-3-sonnet",
            "claude-3-haiku",
            "claude-3.5-sonnet",
            "claude-opus-4",
            "claude-sonnet-4",
        ];

        for pattern in MODEL_PATTERNS {
            if text.contains(pattern) {
                self.model = Some(pattern.to_string());
                return;
            }
        }

        // Also look for "model:" prefix
        for line in text.lines() {
            let line_lower = line.to_lowercase();
            if line_lower.contains("model:") || line_lower.contains("model =") {
                if let Some(model) = line.split_whitespace().find(|w| w.starts_with("claude")) {
                    self.model = Some(model.trim_matches(|c: char| !c.is_alphanumeric() && c != '-' && c != '.').to_string());
                    return;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== Basic Functionality Tests ====================

    #[test]
    fn test_detector_new() {
        let detector = ClaudeDetector::new();
        assert!(!detector.is_claude());
        assert_eq!(*detector.activity(), ClaudeActivity::Idle);
        assert!(detector.session_id().is_none());
        assert!(detector.model().is_none());
        assert_eq!(detector.confidence(), 0);
    }

    #[test]
    fn test_detector_default() {
        let detector = ClaudeDetector::default();
        assert!(!detector.is_claude());
    }

    #[test]
    fn test_detector_with_debounce() {
        let detector = ClaudeDetector::with_debounce(500);
        assert_eq!(detector.debounce, Duration::from_millis(500));
    }

    #[test]
    fn test_mark_as_claude() {
        let mut detector = ClaudeDetector::new();
        assert!(!detector.is_claude());

        detector.mark_as_claude();
        assert!(detector.is_claude());
        assert_eq!(detector.confidence(), 100);
    }

    #[test]
    fn test_reset() {
        let mut detector = ClaudeDetector::new();
        detector.mark_as_claude();
        detector.activity = ClaudeActivity::Thinking;
        detector.session_id = Some("test-id".to_string());
        detector.model = Some("claude-3-opus".to_string());

        detector.reset();

        assert!(!detector.is_claude());
        assert_eq!(*detector.activity(), ClaudeActivity::Idle);
        assert!(detector.session_id().is_none());
        assert!(detector.model().is_none());
        assert_eq!(detector.confidence(), 0);
    }

    #[test]
    fn test_state_none_when_not_claude() {
        let detector = ClaudeDetector::new();
        assert!(detector.state().is_none());
    }

    #[test]
    fn test_state_some_when_claude() {
        let mut detector = ClaudeDetector::new();
        detector.mark_as_claude();
        detector.activity = ClaudeActivity::Thinking;

        let state = detector.state();
        assert!(state.is_some());
        let state = state.unwrap();
        assert_eq!(state.activity, ClaudeActivity::Thinking);
    }

    // ==================== Claude Presence Detection Tests ====================

    #[test]
    fn test_detect_claude_code_string() {
        let mut detector = ClaudeDetector::new();
        detector.analyze("Welcome to Claude Code v1.0");
        assert!(detector.is_claude());
    }

    #[test]
    fn test_detect_claude_code_cli() {
        let mut detector = ClaudeDetector::new();
        detector.analyze("claude-code --version");
        assert!(detector.is_claude());
    }

    #[test]
    fn test_detect_anthropic_claude() {
        let mut detector = ClaudeDetector::new();
        detector.analyze("Powered by Anthropic Claude");
        assert!(detector.is_claude());
    }

    #[test]
    fn test_detect_from_prompt() {
        let mut detector = ClaudeDetector::new();
        detector.analyze("Task complete.\n\n> ");
        assert!(detector.is_claude());
    }

    #[test]
    fn test_detect_from_spinner() {
        let mut detector = ClaudeDetector::new();
        detector.analyze("\r⠋ Thinking...");
        assert!(detector.is_claude());
    }

    #[test]
    fn test_no_detect_random_text() {
        let mut detector = ClaudeDetector::new();
        detector.analyze("Hello world, this is a normal shell");
        assert!(!detector.is_claude());
    }

    // ==================== Activity Detection Tests ====================

    #[test]
    fn test_detect_thinking_state() {
        let mut detector = ClaudeDetector::new();
        detector.mark_as_claude();

        let activity = detector.analyze("\r⠋ Thinking...");
        assert_eq!(activity, Some(ClaudeActivity::Thinking));
    }

    #[test]
    fn test_detect_coding_state() {
        let mut detector = ClaudeDetector::new();
        detector.mark_as_claude();

        let activity = detector.analyze("\r⠋ Writing code...");
        assert_eq!(activity, Some(ClaudeActivity::Coding));
    }

    #[test]
    fn test_detect_coding_channelling() {
        let mut detector = ClaudeDetector::new();
        detector.mark_as_claude();

        let activity = detector.analyze("\r⠙ Channelling...");
        assert_eq!(activity, Some(ClaudeActivity::Coding));
    }

    #[test]
    fn test_detect_tool_use_running() {
        let mut detector = ClaudeDetector::new();
        detector.mark_as_claude();

        let activity = detector.analyze("Running: git status");
        assert_eq!(activity, Some(ClaudeActivity::ToolUse));
    }

    #[test]
    fn test_detect_tool_use_executing() {
        let mut detector = ClaudeDetector::new();
        detector.mark_as_claude();

        let activity = detector.analyze("Executing: npm install");
        assert_eq!(activity, Some(ClaudeActivity::ToolUse));
    }

    #[test]
    fn test_detect_tool_use_specific() {
        let mut detector = ClaudeDetector::new();
        detector.mark_as_claude();

        let activity = detector.analyze("Read(/path/to/file.rs)");
        assert_eq!(activity, Some(ClaudeActivity::ToolUse));
    }

    #[test]
    fn test_detect_confirmation_yn_upper() {
        let mut detector = ClaudeDetector::new();
        detector.mark_as_claude();

        let activity = detector.analyze("Create file /tmp/test.rs? [Y/n]");
        assert_eq!(activity, Some(ClaudeActivity::AwaitingConfirmation));
    }

    #[test]
    fn test_detect_confirmation_yn_lower() {
        let mut detector = ClaudeDetector::new();
        detector.mark_as_claude();

        let activity = detector.analyze("Delete all files? [y/N]");
        assert_eq!(activity, Some(ClaudeActivity::AwaitingConfirmation));
    }

    #[test]
    fn test_detect_confirmation_allow() {
        let mut detector = ClaudeDetector::new();
        detector.mark_as_claude();

        let activity = detector.analyze("Run bash command? Allow?");
        assert_eq!(activity, Some(ClaudeActivity::AwaitingConfirmation));
    }

    #[test]
    fn test_detect_idle_prompt() {
        let mut detector = ClaudeDetector::new();
        detector.mark_as_claude();
        // First set to a different state
        detector.activity = ClaudeActivity::Thinking;

        // Wait for debounce
        std::thread::sleep(Duration::from_millis(150));

        let activity = detector.analyze("Done!\n\n> ");
        assert_eq!(activity, Some(ClaudeActivity::Idle));
    }

    #[test]
    fn test_detect_idle_unicode_prompt() {
        let mut detector = ClaudeDetector::new();
        detector.mark_as_claude();
        detector.activity = ClaudeActivity::Thinking;

        std::thread::sleep(Duration::from_millis(150));

        let activity = detector.analyze("Complete!\n\n❯ ");
        assert_eq!(activity, Some(ClaudeActivity::Idle));
    }

    // ==================== Session ID Extraction Tests ====================

    #[test]
    fn test_extract_session_id() {
        let mut detector = ClaudeDetector::new();
        detector.mark_as_claude();

        detector.analyze("Session: a1b2c3d4-e5f6-7890-abcd-ef1234567890");
        assert_eq!(
            detector.session_id(),
            Some("a1b2c3d4-e5f6-7890-abcd-ef1234567890")
        );
    }

    #[test]
    fn test_extract_session_id_case_insensitive() {
        let mut detector = ClaudeDetector::new();
        detector.mark_as_claude();

        detector.analyze("SESSION: a1b2c3d4-e5f6-7890-abcd-ef1234567890");
        assert_eq!(
            detector.session_id(),
            Some("a1b2c3d4-e5f6-7890-abcd-ef1234567890")
        );
    }

    #[test]
    fn test_no_session_id_without_context() {
        let mut detector = ClaudeDetector::new();
        detector.mark_as_claude();

        // UUID present but no "session" context
        detector.analyze("a1b2c3d4-e5f6-7890-abcd-ef1234567890");
        assert!(detector.session_id().is_none());
    }

    #[test]
    fn test_is_uuid_like() {
        assert!(ClaudeDetector::is_uuid_like(
            "a1b2c3d4-e5f6-7890-abcd-ef1234567890"
        ));
        assert!(ClaudeDetector::is_uuid_like(
            "00000000-0000-0000-0000-000000000000"
        ));
        assert!(!ClaudeDetector::is_uuid_like("not-a-uuid"));
        assert!(!ClaudeDetector::is_uuid_like(
            "a1b2c3d4-e5f6-7890-abcd"
        ));
        assert!(!ClaudeDetector::is_uuid_like(
            "g1b2c3d4-e5f6-7890-abcd-ef1234567890"
        )); // 'g' not hex
    }

    // ==================== Model Extraction Tests ====================

    #[test]
    fn test_extract_model_opus() {
        let mut detector = ClaudeDetector::new();
        detector.mark_as_claude();

        detector.analyze("Using claude-3-opus for this task");
        assert_eq!(detector.model(), Some("claude-3-opus"));
    }

    #[test]
    fn test_extract_model_sonnet() {
        let mut detector = ClaudeDetector::new();
        detector.mark_as_claude();

        detector.analyze("Model: claude-3.5-sonnet");
        assert_eq!(detector.model(), Some("claude-3.5-sonnet"));
    }

    // ==================== Debouncing Tests ====================

    #[test]
    fn test_debounce_prevents_rapid_changes() {
        let mut detector = ClaudeDetector::with_debounce(200);
        detector.mark_as_claude();

        // First change should work
        let result1 = detector.analyze("\r⠋ Thinking...");
        assert_eq!(result1, Some(ClaudeActivity::Thinking));

        // Immediate change should be debounced
        let result2 = detector.analyze("> ");
        assert!(result2.is_none());

        // Activity should still be Thinking due to debounce
        assert_eq!(*detector.activity(), ClaudeActivity::Thinking);
    }

    #[test]
    fn test_debounce_allows_change_after_delay() {
        let mut detector = ClaudeDetector::with_debounce(50);
        detector.mark_as_claude();

        detector.analyze("\r⠋ Thinking...");
        assert_eq!(*detector.activity(), ClaudeActivity::Thinking);

        // Wait for debounce period
        std::thread::sleep(Duration::from_millis(100));

        let result = detector.analyze("> ");
        assert_eq!(result, Some(ClaudeActivity::Idle));
    }

    // ==================== Priority Tests ====================

    #[test]
    fn test_confirmation_priority_over_idle() {
        let mut detector = ClaudeDetector::new();
        detector.mark_as_claude();

        // Text contains both prompt and confirmation
        let activity = detector.analyze("Execute command? [Y/n]\n> ");

        // Confirmation should take priority
        assert_eq!(activity, Some(ClaudeActivity::AwaitingConfirmation));
    }

    #[test]
    fn test_tool_use_priority_over_thinking() {
        let mut detector = ClaudeDetector::new();
        detector.mark_as_claude();

        // Text contains both thinking indicator and tool use
        let activity = detector.analyze("Thinking... Running: test command");

        // Tool use should take priority
        assert_eq!(activity, Some(ClaudeActivity::ToolUse));
    }

    // ==================== Prompt Detection Tests ====================

    #[test]
    fn test_is_prompt_line() {
        assert!(ClaudeDetector::is_prompt_line(">"));
        assert!(ClaudeDetector::is_prompt_line("> "));
        assert!(ClaudeDetector::is_prompt_line("❯"));
        assert!(ClaudeDetector::is_prompt_line("❯ "));
        assert!(ClaudeDetector::is_prompt_line("  > ")); // with leading whitespace

        assert!(!ClaudeDetector::is_prompt_line("Hello"));
        assert!(!ClaudeDetector::is_prompt_line(">> nested"));
    }

    #[test]
    fn test_has_claude_prompt() {
        assert!(ClaudeDetector::has_claude_prompt("Some output\n\n> "));
        assert!(ClaudeDetector::has_claude_prompt("Result\n❯ "));
        assert!(ClaudeDetector::has_claude_prompt("> ")); // Just prompt

        assert!(!ClaudeDetector::has_claude_prompt("Normal output"));
        assert!(!ClaudeDetector::has_claude_prompt("$ shell")); // Shell prompt
    }

    // ==================== State Integration Tests ====================

    #[test]
    fn test_state_includes_session_and_model() {
        let mut detector = ClaudeDetector::new();
        detector.mark_as_claude();
        detector.session_id = Some("test-session-id".to_string());
        detector.model = Some("claude-3-opus".to_string());
        detector.activity = ClaudeActivity::Coding;

        let state = detector.state().unwrap();
        assert_eq!(state.session_id, Some("test-session-id".to_string()));
        assert_eq!(state.model, Some("claude-3-opus".to_string()));
        assert_eq!(state.activity, ClaudeActivity::Coding);
        assert!(state.tokens_used.is_none()); // Not detectable from PTY
    }

    // ==================== Spinner Detection Tests ====================

    #[test]
    fn test_has_spinner_in_last_lines() {
        let detector = ClaudeDetector::new();

        assert!(detector.has_spinner_in_last_lines("⠋ Loading...\nDone"));
        assert!(detector.has_spinner_in_last_lines("Start\n⠙ Processing"));
        assert!(!detector.has_spinner_in_last_lines("No spinners here"));
    }

    // ==================== Debug Format Tests ====================

    #[test]
    fn test_debug_format() {
        let detector = ClaudeDetector::new();
        let debug = format!("{:?}", detector);
        assert!(debug.contains("ClaudeDetector"));
        assert!(debug.contains("is_claude"));
    }
}
