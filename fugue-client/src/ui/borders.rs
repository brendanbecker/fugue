//! Border rendering for panes
//!
//! Provides customizable border styles with support for active pane highlighting
//! and title rendering.

// Allow unused code that's part of the public API for future features
#![allow(dead_code)]

use ratatui::style::{Color, Modifier, Style};
use ratatui::symbols::border;
use ratatui::widgets::{Block, Borders};

use fugue_protocol::ClaudeActivity;

/// Border style options
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BorderStyle {
    /// No border
    None,
    /// Single line border (default)
    #[default]
    Single,
    /// Double line border
    Double,
    /// Rounded corners
    Rounded,
    /// Thick/heavy border
    Thick,
    /// ASCII-only border for compatibility
    Ascii,
}

impl BorderStyle {
    /// Get the ratatui border set for this style
    pub fn border_set(&self) -> border::Set {
        match self {
            BorderStyle::None => border::PLAIN,
            BorderStyle::Single => border::PLAIN,
            BorderStyle::Double => border::DOUBLE,
            BorderStyle::Rounded => border::ROUNDED,
            BorderStyle::Thick => border::THICK,
            BorderStyle::Ascii => border::Set {
                top_left: "+",
                top_right: "+",
                bottom_left: "+",
                bottom_right: "+",
                vertical_left: "|",
                vertical_right: "|",
                horizontal_top: "-",
                horizontal_bottom: "-",
            },
        }
    }
}

/// Border configuration for a pane
#[derive(Debug, Clone)]
pub struct BorderConfig {
    /// Border style
    pub style: BorderStyle,
    /// Whether this pane is focused
    pub focused: bool,
    /// Color for unfocused border
    pub unfocused_color: Color,
    /// Color for focused border
    pub focused_color: Color,
    /// Title text
    pub title: Option<String>,
    /// Title alignment
    pub title_alignment: TitleAlignment,
    /// Claude activity indicator (if any)
    pub claude_activity: Option<ClaudeActivity>,
    /// Animation tick count (for animated indicators)
    pub tick_count: u64,
}

/// Title alignment options
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TitleAlignment {
    #[default]
    Left,
    Center,
    Right,
}

impl Default for BorderConfig {
    fn default() -> Self {
        Self {
            style: BorderStyle::default(),
            focused: false,
            unfocused_color: Color::DarkGray,
            focused_color: Color::Cyan,
            title: None,
            title_alignment: TitleAlignment::Left,
            claude_activity: None,
            tick_count: 0,
        }
    }
}

impl BorderConfig {
    /// Create a new border config
    pub fn new() -> Self {
        Self::default()
    }

    /// Set border style
    pub fn style(mut self, style: BorderStyle) -> Self {
        self.style = style;
        self
    }

    /// Set focused state
    pub fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }

    /// Set unfocused color
    pub fn unfocused_color(mut self, color: Color) -> Self {
        self.unfocused_color = color;
        self
    }

    /// Set focused color
    pub fn focused_color(mut self, color: Color) -> Self {
        self.focused_color = color;
        self
    }

    /// Set title
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set title alignment
    pub fn title_alignment(mut self, alignment: TitleAlignment) -> Self {
        self.title_alignment = alignment;
        self
    }

    /// Set Claude activity
    pub fn claude_activity(mut self, activity: Option<ClaudeActivity>) -> Self {
        self.claude_activity = activity;
        self
    }

    /// Set tick count for animations
    pub fn tick_count(mut self, tick: u64) -> Self {
        self.tick_count = tick;
        self
    }

    /// Get the border color based on focus state
    pub fn border_color(&self) -> Color {
        if self.focused {
            self.focused_color
        } else {
            self.unfocused_color
        }
    }

    /// Get the border style with modifiers
    pub fn border_style(&self) -> Style {
        let mut style = Style::default().fg(self.border_color());
        if self.focused {
            style = style.add_modifier(Modifier::BOLD);
        }
        style
    }

    /// Build the full title string including Claude indicator
    pub fn full_title(&self) -> String {
        let base_title = self.title.as_deref().unwrap_or("");

        if let Some(activity) = &self.claude_activity {
            let indicator = claude_indicator(activity, self.tick_count);
            if base_title.is_empty() {
                indicator
            } else {
                format!("{} {}", base_title, indicator)
            }
        } else {
            base_title.to_string()
        }
    }

    /// Create a ratatui Block from this config
    pub fn to_block(&self) -> Block<'static> {
        if self.style == BorderStyle::None {
            return Block::default();
        }

        let title = self.full_title();
        let border_style = self.border_style();

        let mut block = Block::default()
            .borders(Borders::ALL)
            .border_set(self.style.border_set())
            .border_style(border_style);

        if !title.is_empty() {
            block = block.title(title);
        }

        block
    }
}

/// Get Claude activity indicator string
fn claude_indicator(activity: &ClaudeActivity, tick: u64) -> String {
    match activity {
        ClaudeActivity::Idle => "[ ]".to_string(),
        ClaudeActivity::Thinking => {
            let frames = ["[.  ]", "[.. ]", "[...]", "[ ..]", "[  .]", "[   ]"];
            frames[(tick / 3) as usize % frames.len()].to_string()
        }
        ClaudeActivity::Coding => "[>]".to_string(),
        ClaudeActivity::ToolUse => "[*]".to_string(),
        ClaudeActivity::AwaitingConfirmation => "[?]".to_string(),
    }
}

/// Theme configuration for borders
#[derive(Debug, Clone)]
pub struct BorderTheme {
    /// Default border style
    pub default_style: BorderStyle,
    /// Unfocused pane color
    pub unfocused_color: Color,
    /// Focused pane color
    pub focused_color: Color,
    /// Claude Idle indicator color
    pub claude_idle_color: Color,
    /// Claude Thinking indicator color
    pub claude_thinking_color: Color,
    /// Claude Coding indicator color
    pub claude_coding_color: Color,
    /// Claude Tool Use indicator color
    pub claude_tool_color: Color,
    /// Claude Awaiting Confirmation color
    pub claude_confirm_color: Color,
}

impl Default for BorderTheme {
    fn default() -> Self {
        Self {
            default_style: BorderStyle::Single,
            unfocused_color: Color::DarkGray,
            focused_color: Color::Cyan,
            claude_idle_color: Color::DarkGray,
            claude_thinking_color: Color::Yellow,
            claude_coding_color: Color::Green,
            claude_tool_color: Color::Blue,
            claude_confirm_color: Color::Magenta,
        }
    }
}

impl BorderTheme {
    /// Create a new theme
    pub fn new() -> Self {
        Self::default()
    }

    /// Dark theme variant
    pub fn dark() -> Self {
        Self::default()
    }

    /// Light theme variant
    pub fn light() -> Self {
        Self {
            default_style: BorderStyle::Single,
            unfocused_color: Color::Gray,
            focused_color: Color::Blue,
            claude_idle_color: Color::Gray,
            claude_thinking_color: Color::LightYellow,
            claude_coding_color: Color::LightGreen,
            claude_tool_color: Color::LightBlue,
            claude_confirm_color: Color::LightMagenta,
        }
    }

    /// Get style for Claude activity
    pub fn claude_style(&self, activity: &ClaudeActivity) -> Style {
        let color = match activity {
            ClaudeActivity::Idle => self.claude_idle_color,
            ClaudeActivity::Thinking => self.claude_thinking_color,
            ClaudeActivity::Coding => self.claude_coding_color,
            ClaudeActivity::ToolUse => self.claude_tool_color,
            ClaudeActivity::AwaitingConfirmation => self.claude_confirm_color,
        };

        let mut style = Style::default().fg(color);
        if matches!(activity, ClaudeActivity::Thinking | ClaudeActivity::AwaitingConfirmation) {
            style = style.add_modifier(Modifier::BOLD);
        }
        style
    }

    /// Create a BorderConfig from this theme
    pub fn config(&self, focused: bool) -> BorderConfig {
        BorderConfig {
            style: self.default_style,
            focused,
            unfocused_color: self.unfocused_color,
            focused_color: self.focused_color,
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_border_style_default() {
        let style = BorderStyle::default();
        assert_eq!(style, BorderStyle::Single);
    }

    #[test]
    fn test_border_style_border_set() {
        let single = BorderStyle::Single.border_set();
        let double = BorderStyle::Double.border_set();
        let rounded = BorderStyle::Rounded.border_set();

        assert_ne!(single.top_left, double.top_left);
        assert_ne!(double.top_left, rounded.top_left);
    }

    #[test]
    fn test_border_config_default() {
        let config = BorderConfig::default();
        assert!(!config.focused);
        assert_eq!(config.style, BorderStyle::Single);
        assert!(config.title.is_none());
    }

    #[test]
    fn test_border_config_builder() {
        let config = BorderConfig::new()
            .style(BorderStyle::Double)
            .focused(true)
            .title("Test")
            .focused_color(Color::Green);

        assert!(config.focused);
        assert_eq!(config.style, BorderStyle::Double);
        assert_eq!(config.title, Some("Test".to_string()));
        assert_eq!(config.focused_color, Color::Green);
    }

    #[test]
    fn test_border_config_color() {
        let unfocused = BorderConfig::new().focused(false);
        let focused = BorderConfig::new().focused(true);

        assert_eq!(unfocused.border_color(), Color::DarkGray);
        assert_eq!(focused.border_color(), Color::Cyan);
    }

    #[test]
    fn test_border_config_full_title() {
        let config = BorderConfig::new().title("Pane");
        assert_eq!(config.full_title(), "Pane");
    }

    #[test]
    fn test_border_config_full_title_with_claude() {
        let config = BorderConfig::new()
            .title("Claude")
            .claude_activity(Some(ClaudeActivity::Thinking));

        let title = config.full_title();
        assert!(title.contains("Claude"));
        assert!(title.contains("["));
    }

    #[test]
    fn test_border_config_to_block() {
        let config = BorderConfig::new()
            .style(BorderStyle::Rounded)
            .focused(true)
            .title("Test");

        let block = config.to_block();
        // Block was created successfully
        let _ = block;
    }

    #[test]
    fn test_border_config_to_block_no_border() {
        let config = BorderConfig::new().style(BorderStyle::None);
        let block = config.to_block();
        let _ = block;
    }

    #[test]
    fn test_claude_indicator() {
        assert_eq!(claude_indicator(&ClaudeActivity::Idle, 0), "[ ]");
        assert_eq!(claude_indicator(&ClaudeActivity::Coding, 0), "[>]");
        assert_eq!(claude_indicator(&ClaudeActivity::ToolUse, 0), "[*]");
        assert_eq!(claude_indicator(&ClaudeActivity::AwaitingConfirmation, 0), "[?]");
    }

    #[test]
    fn test_claude_indicator_thinking_animation() {
        let frame0 = claude_indicator(&ClaudeActivity::Thinking, 0);
        let frame3 = claude_indicator(&ClaudeActivity::Thinking, 3);
        let frame6 = claude_indicator(&ClaudeActivity::Thinking, 6);

        assert!(frame0.contains("."));
        assert_ne!(frame0, frame3);
        assert_ne!(frame3, frame6);
    }

    #[test]
    fn test_border_theme_default() {
        let theme = BorderTheme::default();
        assert_eq!(theme.default_style, BorderStyle::Single);
        assert_eq!(theme.focused_color, Color::Cyan);
    }

    #[test]
    fn test_border_theme_dark() {
        let theme = BorderTheme::dark();
        assert_eq!(theme.default_style, BorderStyle::Single);
    }

    #[test]
    fn test_border_theme_light() {
        let theme = BorderTheme::light();
        assert_eq!(theme.focused_color, Color::Blue);
    }

    #[test]
    fn test_border_theme_claude_style() {
        let theme = BorderTheme::default();

        let _idle_style = theme.claude_style(&ClaudeActivity::Idle);
        let thinking_style = theme.claude_style(&ClaudeActivity::Thinking);

        // Thinking should have bold modifier
        assert!(thinking_style.add_modifier == Modifier::BOLD);
    }

    #[test]
    fn test_border_theme_config() {
        let theme = BorderTheme::default();

        let focused_config = theme.config(true);
        let unfocused_config = theme.config(false);

        assert!(focused_config.focused);
        assert!(!unfocused_config.focused);
    }

    #[test]
    fn test_title_alignment_default() {
        let alignment = TitleAlignment::default();
        assert_eq!(alignment, TitleAlignment::Left);
    }

    #[test]
    fn test_border_style_ascii() {
        let set = BorderStyle::Ascii.border_set();
        assert_eq!(set.top_left, "+");
        assert_eq!(set.vertical_left, "|");
        assert_eq!(set.horizontal_top, "-");
    }
}
