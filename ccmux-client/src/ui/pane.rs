//! Pane widget for terminal rendering
//!
//! Provides terminal emulation using tui-term and vt100 for rendering
//! PTY output in ratatui panes.

// Allow unused code that's part of the public API for future features
#![allow(dead_code)]

use std::io::Write;

use base64::{engine::general_purpose, Engine as _};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Borders, Widget};
use tui_term::vt100::{Parser, Screen};
use tui_term::widget::PseudoTerminal;
use uuid::Uuid;

use ccmux_protocol::{AgentActivity, AgentState, ClaudeActivity, PaneState};

/// Visual mode type for text selection
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VisualMode {
    /// Character-wise selection (vim 'v')
    Character,
    /// Line-wise selection (vim 'V')
    Line,
}

/// Position in the terminal buffer
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SelectionPos {
    /// Row index relative to current viewport (0 = top visible line)
    pub row: usize,
    /// Column (0-indexed)
    pub col: usize,
}

impl SelectionPos {
    pub fn new(row: usize, col: usize) -> Self {
        Self { row, col }
    }
}

/// Tracks text selection state within a pane
#[derive(Debug, Clone)]
pub struct Selection {
    /// Anchor position (where selection started)
    pub anchor: SelectionPos,
    /// Cursor position (current end of selection, moves with hjkl)
    pub cursor: SelectionPos,
    /// Visual mode type
    pub mode: VisualMode,
}

impl Selection {
    /// Create a new selection at the given position
    pub fn new(pos: SelectionPos, mode: VisualMode) -> Self {
        Self {
            anchor: pos,
            cursor: pos,
            mode,
        }
    }

    /// Returns (start, end) with start <= end (normalized for iteration)
    pub fn normalized(&self) -> (SelectionPos, SelectionPos) {
        if self.anchor.row < self.cursor.row
            || (self.anchor.row == self.cursor.row && self.anchor.col <= self.cursor.col)
        {
            (self.anchor, self.cursor)
        } else {
            (self.cursor, self.anchor)
        }
    }

    /// Check if a position is within the selection
    pub fn contains(&self, row: usize, col: usize) -> bool {
        let (start, end) = self.normalized();
        match self.mode {
            VisualMode::Line => row >= start.row && row <= end.row,
            VisualMode::Character => {
                if row < start.row || row > end.row {
                    false
                } else if row == start.row && row == end.row {
                    col >= start.col && col <= end.col
                } else if row == start.row {
                    col >= start.col
                } else if row == end.row {
                    col <= end.col
                } else {
                    true // Middle rows are fully selected
                }
            }
        }
    }

    /// Move cursor up by given number of lines
    pub fn move_up(&mut self, lines: usize) {
        self.cursor.row = self.cursor.row.saturating_sub(lines);
    }

    /// Move cursor down by given number of lines
    pub fn move_down(&mut self, lines: usize, max_row: usize) {
        self.cursor.row = (self.cursor.row + lines).min(max_row);
    }

    /// Move cursor left by given number of columns
    pub fn move_left(&mut self, cols: usize) {
        self.cursor.col = self.cursor.col.saturating_sub(cols);
    }

    /// Move cursor right by given number of columns
    pub fn move_right(&mut self, cols: usize, max_col: usize) {
        self.cursor.col = (self.cursor.col + cols).min(max_col);
    }
}

/// Pane focus state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FocusState {
    /// Pane is not focused
    #[default]
    Unfocused,
    /// Pane is focused and active
    Focused,
    /// Pane is in selection mode (for copy/paste)
    Selecting,
}

/// Terminal pane with VT100 emulation
pub struct Pane {
    /// Unique pane ID
    id: Uuid,
    /// VT100 parser for terminal emulation
    parser: Parser,
    /// Current pane title
    title: Option<String>,
    /// Current working directory
    cwd: Option<String>,
    /// Focus state
    focus_state: FocusState,
    /// Pane state (Normal, Claude, Exited)
    pane_state: PaneState,
    /// Scrollback offset (0 = bottom/live, positive = scrolled up)
    scroll_offset: usize,
    /// Whether to show scrollbar
    show_scrollbar: bool,
    /// Copy mode cursor position (row, col) in viewport coordinates
    copy_mode_cursor: Option<SelectionPos>,
    /// Current text selection (when in visual mode)
    selection: Option<Selection>,
    /// Internal paste buffer (fallback when OSC 52 not available)
    paste_buffer: Option<String>,
    /// Whether bracketed paste mode is enabled by the application
    bracketed_paste_enabled: bool,
    /// Whether this pane is a mirror of another pane (FEAT-062)
    is_mirror: bool,
}

impl Pane {
    /// Create a new pane with given dimensions
    pub fn new(id: Uuid, rows: u16, cols: u16) -> Self {
        Self {
            id,
            parser: Parser::new(rows, cols, 1000), // 1000 lines of scrollback
            title: None,
            cwd: None,
            focus_state: FocusState::Unfocused,
            pane_state: PaneState::Normal,
            scroll_offset: 0,
            show_scrollbar: true,
            copy_mode_cursor: None,
            selection: None,
            paste_buffer: None,
            bracketed_paste_enabled: false,
            is_mirror: false,
        }
    }

    /// Get pane ID
    pub fn id(&self) -> Uuid {
        self.id
    }

    /// Get pane title
    pub fn title(&self) -> Option<&str> {
        self.title.as_deref()
    }

    /// Set pane title
    pub fn set_title(&mut self, title: Option<String>) {
        self.title = title;
    }

    /// Get current working directory
    pub fn cwd(&self) -> Option<&str> {
        self.cwd.as_deref()
    }

    /// Set current working directory
    pub fn set_cwd(&mut self, cwd: Option<String>) {
        self.cwd = cwd;
    }

    /// Get focus state
    pub fn focus_state(&self) -> FocusState {
        self.focus_state
    }

    /// Set focus state
    pub fn set_focus_state(&mut self, state: FocusState) {
        self.focus_state = state;
    }

    /// Check if pane is focused
    pub fn is_focused(&self) -> bool {
        self.focus_state == FocusState::Focused
    }

    /// Get pane state
    pub fn pane_state(&self) -> &PaneState {
        &self.pane_state
    }

    /// Set pane state
    pub fn set_pane_state(&mut self, state: PaneState) {
        self.pane_state = state;
    }

    /// Get agent state if in agent state (FEAT-084)
    pub fn agent_state(&self) -> Option<&AgentState> {
        match &self.pane_state {
            PaneState::Agent(state) => Some(state),
            _ => None,
        }
    }

    /// Get agent activity if in agent state (FEAT-084)
    pub fn agent_activity(&self) -> Option<&AgentActivity> {
        match &self.pane_state {
            PaneState::Agent(state) => Some(&state.activity),
            _ => None,
        }
    }

    /// Get Claude activity if this is a Claude pane
    pub fn claude_activity(&self) -> Option<ClaudeActivity> {
        self.pane_state.claude_activity()
    }

    /// Process output data through the terminal parser
    ///
    /// Implements viewport pinning:
    /// - If user was at bottom (scroll_offset == 0), stay at bottom
    /// - If user has scrolled up, don't yank them back down
    pub fn process_output(&mut self, data: &[u8]) {
        // Scan for bracketed paste mode sequences
        // \x1b[?2004h = Enable
        // \x1b[?2004l = Disable
        // Simple scan handles most cases; split packets are edge cases accepted for now
        let s = String::from_utf8_lossy(data);
        if s.contains("\x1b[?2004h") {
            self.bracketed_paste_enabled = true;
        }
        if s.contains("\x1b[?2004l") {
            self.bracketed_paste_enabled = false;
        }

        let was_at_bottom = self.scroll_offset == 0;
        self.parser.process(data);
        if was_at_bottom {
            self.parser.set_scrollback(0);
            self.scroll_offset = 0;
        }
        // If user was scrolled up, leave scroll_offset unchanged
    }

    /// Check if bracketed paste mode is enabled
    pub fn is_bracketed_paste_enabled(&self) -> bool {
        self.bracketed_paste_enabled
    }

    // ==================== Mirror Pane Support (FEAT-062) ====================

    /// Check if this pane is a mirror pane
    pub fn is_mirror(&self) -> bool {
        self.is_mirror
    }

    /// Set whether this pane is a mirror pane
    pub fn set_is_mirror(&mut self, is_mirror: bool) {
        self.is_mirror = is_mirror;
    }

    /// Resize the terminal
    ///
    /// Respects viewport pinning: only reset scroll if already at bottom
    pub fn resize(&mut self, rows: u16, cols: u16) {
        let (current_rows, current_cols) = self.size();
        if current_rows != rows || current_cols != cols {
            let was_at_bottom = self.scroll_offset == 0;
            self.parser.set_size(rows, cols);
            if was_at_bottom {
                self.parser.set_scrollback(0);
                self.scroll_offset = 0;
            }
            // If user was scrolled up, leave scroll_offset unchanged
        }
    }

    /// Get current terminal size (rows, cols)
    pub fn size(&self) -> (u16, u16) {
        let screen = self.parser.screen();
        (screen.size().0, screen.size().1)
    }

    /// Scroll up by given number of lines
    pub fn scroll_up(&mut self, lines: usize) {
        // Calculate desired scroll position
        let desired_offset = self.scroll_offset.saturating_add(lines);
        // Set scrollback - vt100 will clamp to actual scrollback buffer size
        self.parser.set_scrollback(desired_offset);
        // Read back the clamped value to stay in sync with vt100's state
        self.scroll_offset = self.parser.screen().scrollback();
    }

    /// Scroll down by given number of lines
    pub fn scroll_down(&mut self, lines: usize) {
        // Calculate desired scroll position
        let desired_offset = self.scroll_offset.saturating_sub(lines);
        // Set scrollback - vt100 will clamp to valid range
        self.parser.set_scrollback(desired_offset);
        // Read back the clamped value to stay in sync with vt100's state
        self.scroll_offset = self.parser.screen().scrollback();
    }

    /// Scroll to top of scrollback
    pub fn scroll_to_top(&mut self) {
        // Set to maximum possible value - vt100 will clamp to actual scrollback size
        self.parser.set_scrollback(usize::MAX);
        // Read back the clamped value
        self.scroll_offset = self.parser.screen().scrollback();
    }

    /// Scroll to bottom (live view)
    pub fn scroll_to_bottom(&mut self) {
        self.scroll_offset = 0;
        self.parser.set_scrollback(0);
    }

    /// Check if scrolled (not at bottom)
    pub fn is_scrolled(&self) -> bool {
        self.scroll_offset > 0
    }

    /// Get current scroll offset
    pub fn scroll_offset(&self) -> usize {
        self.scroll_offset
    }

    /// Get scroll position as percentage (0.0 = bottom, 1.0 = top)
    pub fn scroll_percentage(&self) -> f32 {
        let max = self.parser.screen().scrollback();
        if max == 0 {
            0.0
        } else {
            self.scroll_offset as f32 / max as f32
        }
    }

    /// Toggle scrollbar visibility
    pub fn toggle_scrollbar(&mut self) {
        self.show_scrollbar = !self.show_scrollbar;
    }

    /// Get the underlying screen for rendering
    pub fn screen(&self) -> &Screen {
        self.parser.screen()
    }

    /// Build the title string for display
    pub fn display_title(&self) -> String {
        let base_title = self.title.as_deref().unwrap_or("pane");

        match &self.pane_state {
            PaneState::Normal => base_title.to_string(),
            PaneState::Agent(state) => {
                let activity = match &state.activity {
                    AgentActivity::Idle => "Idle",
                    AgentActivity::Processing => "Processing...",
                    AgentActivity::Generating => "Generating",
                    AgentActivity::ToolUse => "Tool Use",
                    AgentActivity::AwaitingConfirmation => "Confirm?",
                    AgentActivity::Custom(name) => name.as_str(),
                };
                // Include agent type prefix for non-Claude agents
                if state.agent_type == "claude" {
                    format!("{} [{}]", base_title, activity)
                } else {
                    format!("{} [{}:{}]", base_title, state.agent_type, activity)
                }
            }
            PaneState::Exited { code } => {
                let code_str = code.map(|c| c.to_string()).unwrap_or_else(|| "?".to_string());
                format!("{} [Exit:{}]", base_title, code_str)
            }
        }
    }

    // ==================== Copy Mode & Selection Methods ====================

    /// Enter copy mode and initialize cursor at the screen center
    pub fn enter_copy_mode(&mut self) {
        let (rows, _cols) = self.size();
        // Start cursor in the middle of the screen
        let cursor_row = (rows / 2) as usize;
        self.copy_mode_cursor = Some(SelectionPos::new(cursor_row, 0));
        self.selection = None;
    }

    /// Exit copy mode and clear selection
    pub fn exit_copy_mode(&mut self) {
        self.copy_mode_cursor = None;
        self.selection = None;
        self.focus_state = FocusState::Focused;
    }

    /// Get copy mode cursor position
    pub fn copy_mode_cursor(&self) -> Option<SelectionPos> {
        self.copy_mode_cursor
    }

    /// Move copy mode cursor (before selection is started)
    pub fn move_copy_cursor(&mut self, row_delta: i32, col_delta: i32) {
        let (rows, cols) = self.size();
        let max_row = rows.saturating_sub(1) as usize;
        let max_col = cols.saturating_sub(1) as usize;

        if let Some(ref mut cursor) = self.copy_mode_cursor {
            if row_delta < 0 {
                cursor.row = cursor.row.saturating_sub((-row_delta) as usize);
            } else {
                cursor.row = (cursor.row + row_delta as usize).min(max_row);
            }

            if col_delta < 0 {
                cursor.col = cursor.col.saturating_sub((-col_delta) as usize);
            } else {
                cursor.col = (cursor.col + col_delta as usize).min(max_col);
            }
        }

        // If selection is active, also move the selection cursor
        if let Some(ref mut selection) = self.selection {
            if row_delta < 0 {
                selection.move_up((-row_delta) as usize);
            } else {
                selection.move_down(row_delta as usize, max_row);
            }

            if col_delta < 0 {
                selection.move_left((-col_delta) as usize);
            } else {
                selection.move_right(col_delta as usize, max_col);
            }
        }
    }

    /// Start character-wise visual selection at current cursor
    pub fn start_visual_selection(&mut self) {
        if let Some(cursor) = self.copy_mode_cursor {
            self.selection = Some(Selection::new(cursor, VisualMode::Character));
            self.focus_state = FocusState::Selecting;
        }
    }

    /// Start line-wise visual selection at current cursor
    pub fn start_visual_line_selection(&mut self) {
        if let Some(cursor) = self.copy_mode_cursor {
            self.selection = Some(Selection::new(cursor, VisualMode::Line));
            self.focus_state = FocusState::Selecting;
        }
    }

    /// Get current selection
    pub fn selection(&self) -> Option<&Selection> {
        self.selection.as_ref()
    }

    /// Check if selection is active
    pub fn has_selection(&self) -> bool {
        self.selection.is_some()
    }

    /// Cancel current selection but stay in copy mode
    pub fn cancel_selection(&mut self) {
        self.selection = None;
        self.focus_state = FocusState::Focused;
    }

    /// Extract selected text from the screen buffer
    pub fn extract_selection(&self) -> Option<String> {
        let selection = self.selection.as_ref()?;
        let (start, end) = selection.normalized();
        let screen = self.parser.screen();
        let (_rows, cols) = self.size();

        let mut result = String::new();

        for row in start.row..=end.row {
            let mut line = String::new();

            // Get the row's content
            for col in 0..cols {
                let cell = screen.cell(row as u16, col);
                if let Some(cell) = cell {
                    line.push(cell.contents().chars().next().unwrap_or(' '));
                } else {
                    line.push(' ');
                }
            }

            // Trim trailing whitespace from line
            let trimmed = line.trim_end();

            match selection.mode {
                VisualMode::Line => {
                    result.push_str(trimmed);
                    result.push('\n');
                }
                VisualMode::Character => {
                    if row == start.row && row == end.row {
                        // Single line selection
                        let start_col = start.col.min(trimmed.len());
                        let end_col = (end.col + 1).min(trimmed.len());
                        if start_col < end_col {
                            result.push_str(&trimmed[start_col..end_col]);
                        }
                    } else if row == start.row {
                        // First line of multi-line selection
                        let start_col = start.col.min(trimmed.len());
                        result.push_str(&trimmed[start_col..]);
                        result.push('\n');
                    } else if row == end.row {
                        // Last line of multi-line selection
                        let end_col = (end.col + 1).min(trimmed.len());
                        result.push_str(&trimmed[..end_col]);
                    } else {
                        // Middle lines
                        result.push_str(trimmed);
                        result.push('\n');
                    }
                }
            }
        }

        if result.is_empty() {
            None
        } else {
            Some(result)
        }
    }

    /// Copy selected text to system clipboard via OSC 52 and internal buffer
    pub fn yank_selection(&mut self) -> Option<String> {
        let text = self.extract_selection()?;

        // Store in internal paste buffer (fallback)
        self.paste_buffer = Some(text.clone());

        // Send OSC 52 to system clipboard
        // Format: ESC ] 52 ; c ; BASE64_TEXT BEL
        let encoded = general_purpose::STANDARD.encode(&text);
        let osc52 = format!("\x1b]52;c;{}\x07", encoded);

        // Write directly to stdout (terminal)
        let mut stdout = std::io::stdout();
        let _ = stdout.write_all(osc52.as_bytes());
        let _ = stdout.flush();

        Some(text)
    }

    /// Get text from internal paste buffer
    pub fn paste_buffer(&self) -> Option<&str> {
        self.paste_buffer.as_deref()
    }

    /// Get visual mode indicator for status bar
    pub fn visual_mode_indicator(&self) -> Option<&'static str> {
        self.selection.as_ref().map(|s| match s.mode {
            VisualMode::Character => "-- VISUAL --",
            VisualMode::Line => "-- VISUAL LINE --",
        })
    }

    // ==================== Mouse Selection Methods ====================

    /// Start mouse selection at the given pane-relative position
    pub fn mouse_selection_start(&mut self, row: usize, col: usize) {
        let pos = SelectionPos::new(row, col);
        self.copy_mode_cursor = Some(pos);
        self.selection = Some(Selection::new(pos, VisualMode::Character));
        self.focus_state = FocusState::Selecting;
    }

    /// Update mouse selection end position
    pub fn mouse_selection_update(&mut self, row: usize, col: usize) {
        let (rows, cols) = self.size();
        let row = row.min(rows.saturating_sub(1) as usize);
        let col = col.min(cols.saturating_sub(1) as usize);

        if let Some(ref mut selection) = self.selection {
            selection.cursor = SelectionPos::new(row, col);
        }
        if let Some(ref mut cursor) = self.copy_mode_cursor {
            cursor.row = row;
            cursor.col = col;
        }
    }

    /// Finalize mouse selection
    pub fn mouse_selection_end(&mut self, row: usize, col: usize) {
        // Update to final position
        self.mouse_selection_update(row, col);
        // Selection remains active - user can yank with 'y' or cancel with 'q'
    }

    /// Select word at position (for double-click)
    pub fn select_word_at(&mut self, row: usize, col: usize) {
        let screen = self.parser.screen();
        let (_, cols) = self.size();

        // Find word boundaries
        let mut start_col = col;
        let mut end_col = col;

        // Move start_col left to word boundary
        while start_col > 0 {
            if let Some(cell) = screen.cell(row as u16, (start_col - 1) as u16) {
                let ch = cell.contents().chars().next().unwrap_or(' ');
                if ch.is_whitespace() {
                    break;
                }
                start_col -= 1;
            } else {
                break;
            }
        }

        // Move end_col right to word boundary
        while end_col < cols as usize - 1 {
            if let Some(cell) = screen.cell(row as u16, (end_col + 1) as u16) {
                let ch = cell.contents().chars().next().unwrap_or(' ');
                if ch.is_whitespace() {
                    break;
                }
                end_col += 1;
            } else {
                break;
            }
        }

        // Create selection
        let anchor = SelectionPos::new(row, start_col);
        let cursor = SelectionPos::new(row, end_col);
        self.selection = Some(Selection {
            anchor,
            cursor,
            mode: VisualMode::Character,
        });
        self.copy_mode_cursor = Some(cursor);
        self.focus_state = FocusState::Selecting;
    }

    /// Select entire line at position (for triple-click)
    pub fn select_line_at(&mut self, row: usize) {
        let pos = SelectionPos::new(row, 0);
        self.selection = Some(Selection::new(pos, VisualMode::Line));
        self.copy_mode_cursor = Some(pos);
        self.focus_state = FocusState::Selecting;
    }
}

/// Widget state for pane rendering
pub struct PaneWidgetState<'a> {
    pane: &'a Pane,
    tick_count: u64,
}

impl<'a> PaneWidgetState<'a> {
    pub fn new(pane: &'a Pane, tick_count: u64) -> Self {
        Self { pane, tick_count }
    }
}

/// Pane widget for rendering
pub struct PaneWidget {
    /// Block for border styling
    block: Option<Block<'static>>,
    /// Whether to show Claude state indicator
    show_indicator: bool,
}

impl PaneWidget {
    pub fn new() -> Self {
        Self {
            block: None,
            show_indicator: true,
        }
    }

    /// Set the block (border) for this pane
    pub fn block(mut self, block: Block<'static>) -> Self {
        self.block = Some(block);
        self
    }

    /// Set whether to show Claude indicator
    pub fn show_indicator(mut self, show: bool) -> Self {
        self.show_indicator = show;
        self
    }

    /// Build the border style based on focus state
    fn border_style(focus_state: FocusState) -> Style {
        match focus_state {
            FocusState::Unfocused => Style::default().fg(Color::DarkGray),
            FocusState::Focused => Style::default().fg(Color::Cyan),
            FocusState::Selecting => Style::default().fg(Color::Yellow),
        }
    }

    /// Get the border block for a pane
    pub fn create_block(pane: &Pane) -> Block<'static> {
        let title = pane.display_title();

        // FEAT-062: Use dim cyan border for mirror panes
        let border_style = if pane.is_mirror() {
            Style::default().fg(Color::Cyan).add_modifier(Modifier::DIM)
        } else {
            Self::border_style(pane.focus_state())
        };

        let mut block = Block::default()
            .borders(Borders::ALL)
            .title(title)
            .border_style(border_style);

        // Add scroll indicator if scrolled
        if pane.is_scrolled() {
            let scroll_indicator = format!(" [{}↑] ", pane.scroll_offset);
            block = block.title_bottom(scroll_indicator);
        }

        // FEAT-062: Add mirror indicator for mirror panes
        if pane.is_mirror() {
            block = block.title_bottom(" [READ-ONLY] ");
        }

        block
    }
}

impl Default for PaneWidget {
    fn default() -> Self {
        Self::new()
    }
}

impl Widget for PaneWidget {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Without state, we can only render an empty block
        let block = self.block.unwrap_or_else(|| {
            Block::default()
                .borders(Borders::ALL)
                .title("Pane")
        });
        block.render(area, buf);
    }
}

/// Render a pane with its terminal content
pub fn render_pane(pane: &Pane, area: Rect, buf: &mut Buffer, tick_count: u64) {
    // Create the border block
    let block = PaneWidget::create_block(pane);
    let inner = block.inner(area);

    // Render the block
    block.render(area, buf);

    // Render the terminal content
    if inner.width > 0 && inner.height > 0 {
        // Debug: Check for size mismatch between VT100 screen and render area
        // Only log every ~1 second (tick_count is ~10/sec) to avoid spam
        let (screen_rows, screen_cols) = pane.size();
        if (screen_rows != inner.height || screen_cols != inner.width)
            && tick_count.is_multiple_of(10) {
                tracing::warn!(
                    screen_rows, screen_cols,
                    render_height = inner.height, render_width = inner.width,
                    scroll_offset = pane.scroll_offset(),
                    "VT100 screen size mismatch with render area"
                );
            }

        let pseudo_term = PseudoTerminal::new(pane.screen())
            .style(Style::default().fg(Color::White).bg(Color::Black));

        pseudo_term.render(inner, buf);

        // Render selection highlighting
        render_selection(pane, inner, buf);

        // Render copy mode cursor
        render_copy_mode_cursor(pane, inner, buf, tick_count);

        // Render agent/Claude state indicator if applicable (FEAT-084)
        if let Some(state) = pane.agent_state() {
            render_agent_indicator(state, inner, buf, tick_count);
        } else if let Some(ref activity) = pane.claude_activity() {
            render_claude_indicator(activity, inner, buf, tick_count);
        }
    }
}

/// Render selection highlighting
fn render_selection(pane: &Pane, area: Rect, buf: &mut Buffer) {
    let selection = match pane.selection() {
        Some(s) => s,
        None => return,
    };

    // Selection highlight style - reversed colors
    let selection_style = Style::default()
        .fg(Color::Black)
        .bg(Color::White)
        .add_modifier(Modifier::REVERSED);

    let (start, end) = selection.normalized();

    for row in start.row..=end.row {
        if row >= area.height as usize {
            continue;
        }

        let y = area.y + row as u16;

        match selection.mode {
            VisualMode::Line => {
                // Highlight entire line
                for col in 0..area.width {
                    let x = area.x + col;
                    if let Some(cell) = buf.cell_mut((x, y)) {
                        // Apply reversed style to existing cell
                        cell.set_style(selection_style);
                    }
                }
            }
            VisualMode::Character => {
                // Determine column range for this row
                let (start_col, end_col) = if row == start.row && row == end.row {
                    (start.col, end.col)
                } else if row == start.row {
                    (start.col, area.width as usize - 1)
                } else if row == end.row {
                    (0, end.col)
                } else {
                    (0, area.width as usize - 1)
                };

                for col in start_col..=end_col {
                    if col >= area.width as usize {
                        continue;
                    }
                    let x = area.x + col as u16;
                    if let Some(cell) = buf.cell_mut((x, y)) {
                        cell.set_style(selection_style);
                    }
                }
            }
        }
    }
}

/// Render copy mode cursor (blinking block cursor)
fn render_copy_mode_cursor(pane: &Pane, area: Rect, buf: &mut Buffer, tick_count: u64) {
    let cursor = match pane.copy_mode_cursor() {
        Some(c) => c,
        None => return,
    };

    // Skip if selection is active (cursor is shown via selection highlight)
    if pane.has_selection() {
        return;
    }

    // Blink cursor (on for 3 ticks, off for 3 ticks)
    if (tick_count / 3) % 2 == 1 {
        return;
    }

    if cursor.row >= area.height as usize || cursor.col >= area.width as usize {
        return;
    }

    let x = area.x + cursor.col as u16;
    let y = area.y + cursor.row as u16;

    // Block cursor style - inverse video
    let cursor_style = Style::default()
        .fg(Color::Black)
        .bg(Color::White);

    if let Some(cell) = buf.cell_mut((x, y)) {
        cell.set_style(cursor_style);
    }
}

/// Render agent state indicator in the pane (FEAT-084)
fn render_agent_indicator(state: &AgentState, area: Rect, buf: &mut Buffer, tick_count: u64) {
    // Small indicator in top-right corner
    if area.width < 5 || area.height < 1 {
        return;
    }

    let indicator = match &state.activity {
        AgentActivity::Idle => "[ ]",
        AgentActivity::Processing => {
            let frames = ["[.  ]", "[.. ]", "[...]", "[ ..]", "[  .]", "[   ]"];
            frames[(tick_count / 3) as usize % frames.len()]
        }
        AgentActivity::Generating => "[>]",
        AgentActivity::ToolUse => "[*]",
        AgentActivity::AwaitingConfirmation => "[?]",
        AgentActivity::Custom(_) => "[~]",
    };

    let style = match &state.activity {
        AgentActivity::Idle => Style::default().fg(Color::DarkGray),
        AgentActivity::Processing => Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        AgentActivity::Generating => Style::default().fg(Color::Green),
        AgentActivity::ToolUse => Style::default().fg(Color::Blue),
        AgentActivity::AwaitingConfirmation => Style::default().fg(Color::Magenta),
        AgentActivity::Custom(_) => Style::default().fg(Color::Cyan),
    };

    let x = area.x + area.width.saturating_sub(indicator.len() as u16 + 1);
    let y = area.y;

    for (i, ch) in indicator.chars().enumerate() {
        let cell_x = x + (i as u16);
        if cell_x < area.x + area.width {
            if let Some(cell) = buf.cell_mut((cell_x, y)) {
                cell.set_char(ch).set_style(style);
            }
        }
    }
}

/// Render Claude state indicator in the pane (deprecated, use render_agent_indicator)
fn render_claude_indicator(activity: &ClaudeActivity, area: Rect, buf: &mut Buffer, tick_count: u64) {
    // Small indicator in top-right corner
    if area.width < 5 || area.height < 1 {
        return;
    }

    let indicator = match activity {
        ClaudeActivity::Idle => "[ ]",
        ClaudeActivity::Thinking => {
            let frames = ["[.  ]", "[.. ]", "[...]", "[ ..]", "[  .]", "[   ]"];
            frames[(tick_count / 3) as usize % frames.len()]
        }
        ClaudeActivity::Coding => "[>]",
        ClaudeActivity::ToolUse => "[*]",
        ClaudeActivity::AwaitingConfirmation => "[?]",
    };

    let style = match activity {
        ClaudeActivity::Idle => Style::default().fg(Color::DarkGray),
        ClaudeActivity::Thinking => Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        ClaudeActivity::Coding => Style::default().fg(Color::Green),
        ClaudeActivity::ToolUse => Style::default().fg(Color::Blue),
        ClaudeActivity::AwaitingConfirmation => Style::default().fg(Color::Magenta),
    };

    let x = area.x + area.width.saturating_sub(indicator.len() as u16 + 1);
    let y = area.y;

    for (i, ch) in indicator.chars().enumerate() {
        let cell_x = x + (i as u16);
        if cell_x < area.x + area.width {
            if let Some(cell) = buf.cell_mut((cell_x, y)) {
                cell.set_char(ch).set_style(style);
            }
        }
    }
}

/// Collection of panes
#[derive(Default)]
pub struct PaneManager {
    panes: std::collections::HashMap<Uuid, Pane>,
    active_pane_id: Option<Uuid>,
}

impl PaneManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a new pane
    pub fn add_pane(&mut self, id: Uuid, rows: u16, cols: u16) {
        let pane = Pane::new(id, rows, cols);
        self.panes.insert(id, pane);

        // Set as active if it's the first pane
        if self.active_pane_id.is_none() {
            self.active_pane_id = Some(id);
        }
    }

    /// Remove a pane
    pub fn remove_pane(&mut self, id: Uuid) -> Option<Pane> {
        let pane = self.panes.remove(&id);

        // Update active pane if needed
        if self.active_pane_id == Some(id) {
            self.active_pane_id = self.panes.keys().next().copied();
        }

        pane
    }

    /// Get a pane by ID
    pub fn get(&self, id: Uuid) -> Option<&Pane> {
        self.panes.get(&id)
    }

    /// Get a mutable pane by ID
    pub fn get_mut(&mut self, id: Uuid) -> Option<&mut Pane> {
        self.panes.get_mut(&id)
    }

    /// Get the active pane
    pub fn active_pane(&self) -> Option<&Pane> {
        self.active_pane_id.and_then(|id| self.panes.get(&id))
    }

    /// Get the active pane mutably
    pub fn active_pane_mut(&mut self) -> Option<&mut Pane> {
        self.active_pane_id.and_then(|id| self.panes.get_mut(&id))
    }

    /// Get active pane ID
    pub fn active_pane_id(&self) -> Option<Uuid> {
        self.active_pane_id
    }

    /// Set the active pane
    pub fn set_active(&mut self, id: Uuid) {
        if self.panes.contains_key(&id) {
            // Update focus states
            if let Some(old_id) = self.active_pane_id {
                if let Some(pane) = self.panes.get_mut(&old_id) {
                    pane.set_focus_state(FocusState::Unfocused);
                }
            }
            if let Some(pane) = self.panes.get_mut(&id) {
                pane.set_focus_state(FocusState::Focused);
            }
            self.active_pane_id = Some(id);
        }
    }

    /// Process output for a pane
    pub fn process_output(&mut self, id: Uuid, data: &[u8]) {
        if let Some(pane) = self.panes.get_mut(&id) {
            pane.process_output(data);
        }
    }

    /// Resize a pane
    pub fn resize_pane(&mut self, id: Uuid, rows: u16, cols: u16) {
        if let Some(pane) = self.panes.get_mut(&id) {
            pane.resize(rows, cols);
        }
    }

    /// Update pane state
    pub fn update_pane_state(&mut self, id: Uuid, state: PaneState) {
        if let Some(pane) = self.panes.get_mut(&id) {
            pane.set_pane_state(state);
        }
    }

    /// Get all pane IDs
    pub fn pane_ids(&self) -> Vec<Uuid> {
        self.panes.keys().copied().collect()
    }

    /// Get number of panes
    pub fn count(&self) -> usize {
        self.panes.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.panes.is_empty()
    }

    /// Iterate over all panes
    pub fn iter(&self) -> impl Iterator<Item = (&Uuid, &Pane)> {
        self.panes.iter()
    }

    /// Iterate over all panes mutably
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&Uuid, &mut Pane)> {
        self.panes.iter_mut()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pane_creation() {
        let id = Uuid::new_v4();
        let pane = Pane::new(id, 24, 80);

        assert_eq!(pane.id(), id);
        assert_eq!(pane.focus_state(), FocusState::Unfocused);
        assert!(!pane.is_focused());
    }

    #[test]
    fn test_pane_title() {
        let id = Uuid::new_v4();
        let mut pane = Pane::new(id, 24, 80);

        assert!(pane.title().is_none());

        pane.set_title(Some("test".to_string()));
        assert_eq!(pane.title(), Some("test"));
    }

    #[test]
    fn test_pane_focus() {
        let id = Uuid::new_v4();
        let mut pane = Pane::new(id, 24, 80);

        pane.set_focus_state(FocusState::Focused);
        assert!(pane.is_focused());
        assert_eq!(pane.focus_state(), FocusState::Focused);

        pane.set_focus_state(FocusState::Unfocused);
        assert!(!pane.is_focused());
    }

    #[test]
    fn test_pane_output() {
        let id = Uuid::new_v4();
        let mut pane = Pane::new(id, 24, 80);

        pane.process_output(b"Hello, World!");
        // Output was processed (we can't easily verify content)
    }

    #[test]
    fn test_pane_resize() {
        let id = Uuid::new_v4();
        let mut pane = Pane::new(id, 24, 80);

        pane.resize(40, 120);
        let (rows, cols) = pane.size();
        assert_eq!(rows, 40);
        assert_eq!(cols, 120);
    }

    #[test]
    fn test_pane_scrolling() {
        let id = Uuid::new_v4();
        let mut pane = Pane::new(id, 24, 80);

        assert!(!pane.is_scrolled());
        assert_eq!(pane.scroll_offset, 0);

        pane.scroll_up(10);
        // Note: scrollback is empty, so scroll_up may not change offset
    }

    #[test]
    fn test_display_title_normal() {
        let id = Uuid::new_v4();
        let mut pane = Pane::new(id, 24, 80);
        pane.set_title(Some("bash".to_string()));

        assert_eq!(pane.display_title(), "bash");
    }

    #[test]
    fn test_display_title_agent() {
        let id = Uuid::new_v4();
        let mut pane = Pane::new(id, 24, 80);
        pane.set_title(Some("claude".to_string()));
        pane.set_pane_state(PaneState::Agent(
            ccmux_protocol::AgentState::new("claude")
                .with_activity(ccmux_protocol::AgentActivity::Processing),
        ));

        assert!(pane.display_title().contains("Processing"));
    }

    #[test]
    fn test_display_title_exited() {
        let id = Uuid::new_v4();
        let mut pane = Pane::new(id, 24, 80);
        pane.set_title(Some("process".to_string()));
        pane.set_pane_state(PaneState::Exited { code: Some(0) });

        assert!(pane.display_title().contains("Exit:0"));
    }

    #[test]
    fn test_pane_manager_new() {
        let manager = PaneManager::new();
        assert!(manager.is_empty());
        assert_eq!(manager.count(), 0);
    }

    #[test]
    fn test_pane_manager_add() {
        let mut manager = PaneManager::new();
        let id = Uuid::new_v4();

        manager.add_pane(id, 24, 80);

        assert_eq!(manager.count(), 1);
        assert!(manager.get(id).is_some());
        assert_eq!(manager.active_pane_id(), Some(id));
    }

    #[test]
    fn test_pane_manager_remove() {
        let mut manager = PaneManager::new();
        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();

        manager.add_pane(id1, 24, 80);
        manager.add_pane(id2, 24, 80);

        manager.remove_pane(id1);
        assert_eq!(manager.count(), 1);
        assert!(manager.get(id1).is_none());
    }

    #[test]
    fn test_pane_manager_set_active() {
        let mut manager = PaneManager::new();
        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();

        manager.add_pane(id1, 24, 80);
        manager.add_pane(id2, 24, 80);

        manager.set_active(id2);
        assert_eq!(manager.active_pane_id(), Some(id2));

        let pane1 = manager.get(id1).unwrap();
        let pane2 = manager.get(id2).unwrap();
        assert_eq!(pane1.focus_state(), FocusState::Unfocused);
        assert_eq!(pane2.focus_state(), FocusState::Focused);
    }

    #[test]
    fn test_pane_widget_border_style() {
        let unfocused = PaneWidget::border_style(FocusState::Unfocused);
        let focused = PaneWidget::border_style(FocusState::Focused);
        let selecting = PaneWidget::border_style(FocusState::Selecting);

        assert_ne!(unfocused, focused);
        assert_ne!(focused, selecting);
    }

    // ==================== Selection Tests ====================

    #[test]
    fn test_selection_pos_new() {
        let pos = SelectionPos::new(5, 10);
        assert_eq!(pos.row, 5);
        assert_eq!(pos.col, 10);
    }

    #[test]
    fn test_selection_new() {
        let pos = SelectionPos::new(3, 5);
        let selection = Selection::new(pos, VisualMode::Character);

        assert_eq!(selection.anchor.row, 3);
        assert_eq!(selection.anchor.col, 5);
        assert_eq!(selection.cursor.row, 3);
        assert_eq!(selection.cursor.col, 5);
        assert_eq!(selection.mode, VisualMode::Character);
    }

    #[test]
    fn test_selection_normalized_forward() {
        let mut selection = Selection::new(SelectionPos::new(2, 5), VisualMode::Character);
        selection.cursor = SelectionPos::new(5, 10);

        let (start, end) = selection.normalized();
        assert_eq!(start.row, 2);
        assert_eq!(start.col, 5);
        assert_eq!(end.row, 5);
        assert_eq!(end.col, 10);
    }

    #[test]
    fn test_selection_normalized_backward() {
        let mut selection = Selection::new(SelectionPos::new(5, 10), VisualMode::Character);
        selection.cursor = SelectionPos::new(2, 5);

        let (start, end) = selection.normalized();
        assert_eq!(start.row, 2);
        assert_eq!(start.col, 5);
        assert_eq!(end.row, 5);
        assert_eq!(end.col, 10);
    }

    #[test]
    fn test_selection_contains_character_mode() {
        let mut selection = Selection::new(SelectionPos::new(2, 5), VisualMode::Character);
        selection.cursor = SelectionPos::new(4, 10);

        // Middle row should be fully selected
        assert!(selection.contains(3, 0));
        assert!(selection.contains(3, 50));

        // Start row - only from start col
        assert!(!selection.contains(2, 3));
        assert!(selection.contains(2, 5));
        assert!(selection.contains(2, 10));

        // End row - only up to end col
        assert!(selection.contains(4, 0));
        assert!(selection.contains(4, 10));
        assert!(!selection.contains(4, 11));

        // Outside rows
        assert!(!selection.contains(1, 5));
        assert!(!selection.contains(5, 5));
    }

    #[test]
    fn test_selection_contains_line_mode() {
        let mut selection = Selection::new(SelectionPos::new(2, 5), VisualMode::Line);
        selection.cursor = SelectionPos::new(4, 10);

        // All columns in selected rows should be selected
        assert!(selection.contains(2, 0));
        assert!(selection.contains(2, 100));
        assert!(selection.contains(3, 50));
        assert!(selection.contains(4, 0));

        // Outside rows
        assert!(!selection.contains(1, 5));
        assert!(!selection.contains(5, 5));
    }

    #[test]
    fn test_selection_move_up() {
        let mut selection = Selection::new(SelectionPos::new(5, 0), VisualMode::Character);
        selection.move_up(2);
        assert_eq!(selection.cursor.row, 3);

        selection.move_up(10); // Should saturate at 0
        assert_eq!(selection.cursor.row, 0);
    }

    #[test]
    fn test_selection_move_down() {
        let mut selection = Selection::new(SelectionPos::new(5, 0), VisualMode::Character);
        selection.move_down(3, 20);
        assert_eq!(selection.cursor.row, 8);

        selection.move_down(100, 20); // Should clamp to max
        assert_eq!(selection.cursor.row, 20);
    }

    #[test]
    fn test_pane_enter_copy_mode() {
        let id = Uuid::new_v4();
        let mut pane = Pane::new(id, 24, 80);

        assert!(pane.copy_mode_cursor().is_none());

        pane.enter_copy_mode();

        assert!(pane.copy_mode_cursor().is_some());
        let cursor = pane.copy_mode_cursor().unwrap();
        assert_eq!(cursor.row, 12); // Middle of 24 rows
        assert_eq!(cursor.col, 0);
        assert!(pane.selection().is_none());
    }

    #[test]
    fn test_pane_exit_copy_mode() {
        let id = Uuid::new_v4();
        let mut pane = Pane::new(id, 24, 80);

        pane.enter_copy_mode();
        pane.start_visual_selection();

        assert!(pane.copy_mode_cursor().is_some());
        assert!(pane.selection().is_some());

        pane.exit_copy_mode();

        assert!(pane.copy_mode_cursor().is_none());
        assert!(pane.selection().is_none());
        assert_eq!(pane.focus_state(), FocusState::Focused);
    }

    #[test]
    fn test_pane_start_visual_selection() {
        let id = Uuid::new_v4();
        let mut pane = Pane::new(id, 24, 80);

        pane.enter_copy_mode();
        pane.start_visual_selection();

        assert!(pane.selection().is_some());
        let selection = pane.selection().unwrap();
        assert_eq!(selection.mode, VisualMode::Character);
        assert_eq!(pane.focus_state(), FocusState::Selecting);
    }

    #[test]
    fn test_pane_start_visual_line_selection() {
        let id = Uuid::new_v4();
        let mut pane = Pane::new(id, 24, 80);

        pane.enter_copy_mode();
        pane.start_visual_line_selection();

        assert!(pane.selection().is_some());
        let selection = pane.selection().unwrap();
        assert_eq!(selection.mode, VisualMode::Line);
    }

    #[test]
    fn test_pane_move_copy_cursor() {
        let id = Uuid::new_v4();
        let mut pane = Pane::new(id, 24, 80);

        pane.enter_copy_mode();
        let initial = pane.copy_mode_cursor().unwrap();

        pane.move_copy_cursor(2, 5);
        let moved = pane.copy_mode_cursor().unwrap();

        assert_eq!(moved.row, initial.row + 2);
        assert_eq!(moved.col, initial.col + 5);
    }

    #[test]
    fn test_pane_move_copy_cursor_with_selection() {
        let id = Uuid::new_v4();
        let mut pane = Pane::new(id, 24, 80);

        pane.enter_copy_mode();
        pane.start_visual_selection();
        pane.move_copy_cursor(3, 10);

        let selection = pane.selection().unwrap();
        // Anchor should stay at original position
        assert_eq!(selection.anchor.row, 12); // Middle of 24
        assert_eq!(selection.anchor.col, 0);
        // Cursor should have moved
        assert_eq!(selection.cursor.row, 15);
        assert_eq!(selection.cursor.col, 10);
    }

    #[test]
    fn test_pane_cancel_selection() {
        let id = Uuid::new_v4();
        let mut pane = Pane::new(id, 24, 80);

        pane.enter_copy_mode();
        pane.start_visual_selection();

        assert!(pane.selection().is_some());

        pane.cancel_selection();

        assert!(pane.selection().is_none());
        assert!(pane.copy_mode_cursor().is_some()); // Still in copy mode
        assert_eq!(pane.focus_state(), FocusState::Focused);
    }

    #[test]
    fn test_pane_visual_mode_indicator() {
        let id = Uuid::new_v4();
        let mut pane = Pane::new(id, 24, 80);

        assert!(pane.visual_mode_indicator().is_none());

        pane.enter_copy_mode();
        pane.start_visual_selection();
        assert_eq!(pane.visual_mode_indicator(), Some("-- VISUAL --"));

        pane.cancel_selection();
        pane.start_visual_line_selection();
        assert_eq!(pane.visual_mode_indicator(), Some("-- VISUAL LINE --"));
    }

    #[test]
    fn test_pane_mouse_selection_start() {
        let id = Uuid::new_v4();
        let mut pane = Pane::new(id, 24, 80);

        pane.mouse_selection_start(5, 10);

        assert!(pane.selection().is_some());
        let selection = pane.selection().unwrap();
        assert_eq!(selection.anchor.row, 5);
        assert_eq!(selection.anchor.col, 10);
        assert_eq!(pane.focus_state(), FocusState::Selecting);
    }

    #[test]
    fn test_pane_mouse_selection_update() {
        let id = Uuid::new_v4();
        let mut pane = Pane::new(id, 24, 80);

        pane.mouse_selection_start(5, 10);
        pane.mouse_selection_update(8, 15);

        let selection = pane.selection().unwrap();
        assert_eq!(selection.anchor.row, 5);
        assert_eq!(selection.anchor.col, 10);
        assert_eq!(selection.cursor.row, 8);
        assert_eq!(selection.cursor.col, 15);
    }

    #[test]
    fn test_pane_select_line_at() {
        let id = Uuid::new_v4();
        let mut pane = Pane::new(id, 24, 80);

        pane.select_line_at(7);

        let selection = pane.selection().unwrap();
        assert_eq!(selection.mode, VisualMode::Line);
        assert_eq!(selection.anchor.row, 7);
    }

    #[test]
    fn test_pane_bracketed_paste_detection() {
        let id = Uuid::new_v4();
        let mut pane = Pane::new(id, 24, 80);

        assert!(!pane.is_bracketed_paste_enabled());

        // Enable sequence
        pane.process_output(b"\x1b[?2004h");
        assert!(pane.is_bracketed_paste_enabled());

        // Normal output shouldn't change it
        pane.process_output(b"hello world");
        assert!(pane.is_bracketed_paste_enabled());

        // Disable sequence
        pane.process_output(b"\x1b[?2004l");
        assert!(!pane.is_bracketed_paste_enabled());
        
        // Mixed output
        pane.process_output(b"text\x1b[?2004hmore text");
        assert!(pane.is_bracketed_paste_enabled());
    }
}
