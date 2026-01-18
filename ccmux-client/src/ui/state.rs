use std::collections::HashMap;
use std::time::Instant;

use ratatui::layout::Rect;
use ratatui::widgets::ListState;
use uuid::Uuid;

use ccmux_protocol::{
    ClaudeActivity, MailPriority, PaneInfo, PaneState, SessionInfo, SplitDirection, WindowInfo,
};

use crate::input::InputMode;
use super::pane::{FocusState, PaneManager};
use super::layout::LayoutManager;

/// Application state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppState {
    /// Initial state, not connected
    Disconnected,
    /// Connecting to server
    Connecting,
    /// Connected, selecting session
    SessionSelect,
    /// Attached to a session
    Attached,
    /// Shutting down
    Quitting,
}

/// View mode within Attached state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    /// Normal pane view
    Panes,
    /// System-wide dashboard
    Dashboard,
}

/// Message in the mailbox widget
#[derive(Debug, Clone)]
pub struct MailboxMessage {
    pub pane_id: Uuid,
    pub timestamp: std::time::SystemTime,
    pub priority: MailPriority,
    pub summary: String,
}

/// Main state for the client
pub struct ClientState {
    /// Current application state
    pub state: AppState,
    /// Current view mode
    pub view_mode: ViewMode,
    /// Client ID
    pub client_id: Uuid,
    /// Current session info
    pub session: Option<SessionInfo>,
    /// Windows in current session
    pub windows: HashMap<Uuid, WindowInfo>,
    /// Panes in current session
    pub panes: HashMap<Uuid, PaneInfo>,
    /// Mailbox messages (FEAT-073)
    pub mailbox: Vec<MailboxMessage>,
    /// List state for mailbox UI
    pub mailbox_state: ListState,
    /// Active pane ID
    pub active_pane_id: Option<Uuid>,
    /// Last (previously active) pane ID for Ctrl-b ; (tmux last-pane)
    pub last_pane_id: Option<Uuid>,
    /// Last (previously active) window ID for Ctrl-b l (tmux last-window)
    pub last_window_id: Option<Uuid>,
    /// Available sessions (when in SessionSelect state)
    pub available_sessions: Vec<SessionInfo>,
    /// Selected session index in session list
    pub session_list_index: usize,
    /// Terminal size (cols, rows)
    pub terminal_size: (u16, u16),
    /// Animation tick counter
    pub tick_count: u64,
    /// Status message to display
    pub status_message: Option<String>,
    /// UI pane manager for terminal rendering
    pub pane_manager: PaneManager,
    /// Layout manager for pane arrangement
    pub layout: Option<LayoutManager>,
    /// Pending split direction for next pane creation
    pub pending_split_direction: Option<SplitDirection>,
    /// Custom command to run in new sessions (from CLI args)
    pub session_command: Option<String>,
    /// Previous input mode for tracking mode transitions (FEAT-056)
    /// Used to detect when user exits command mode
    pub previous_input_mode: InputMode,
    /// Last tick when beads status was requested (FEAT-058)
    pub last_beads_request_tick: u64,
    /// Whether current pane is in a beads-tracked repo (FEAT-057)
    pub is_beads_tracked: bool,
    /// Beads ready task count (FEAT-058): None = unavailable, Some(n) = n tasks ready
    pub beads_ready_count: Option<usize>,
    /// Expiry time for human control lock (FEAT-077)
    pub human_control_lock_expiry: Option<Instant>,
    /// Last seen commit sequence number (FEAT-075)
    pub last_seen_commit_seq: u64,
    /// Whether a full screen redraw is requested
    pub needs_redraw: bool,
}

impl ClientState {
    pub fn new(client_id: Uuid) -> Self {
        Self {
            state: AppState::Disconnected,
            view_mode: ViewMode::Panes,
            client_id,
            session: None,
            windows: HashMap::new(),
            panes: HashMap::new(),
            mailbox: Vec::new(),
            mailbox_state: ListState::default(),
            active_pane_id: None,
            last_pane_id: None,
            last_window_id: None,
            available_sessions: Vec::new(),
            session_list_index: 0,
            terminal_size: (80, 24),
            tick_count: 0,
            status_message: None,
            pane_manager: PaneManager::new(),
            layout: None,
            pending_split_direction: None,
            session_command: None,
            previous_input_mode: InputMode::Normal,
            last_beads_request_tick: 0,
            is_beads_tracked: false,
            beads_ready_count: None,
            human_control_lock_expiry: None,
            last_seen_commit_seq: 0,
            needs_redraw: false,
        }
    }

    /// Check if application should quit
    pub fn should_quit(&self) -> bool {
        self.state == AppState::Quitting
    }

    /// Calculate weights for all panes based on activity and focus
    pub fn calculate_pane_weights(&self) -> HashMap<Uuid, f32> {
        let mut weights = HashMap::new();
        for (id, pane) in self.pane_manager.iter() {
            let mut weight = 1.0;

            // Focus bonus
            if pane.is_focused() {
                weight *= 1.2;
            }

            // Claude activity bonus
            if let Some(activity) = pane.claude_activity() {
                match activity {
                    ClaudeActivity::Thinking | ClaudeActivity::Coding | ClaudeActivity::ToolUse => {
                        weight *= 1.5;
                    }
                    ClaudeActivity::AwaitingConfirmation => {
                        weight *= 1.3;
                    }
                    ClaudeActivity::Idle => {}
                }
            }

            // Exited penalty
            if let PaneState::Exited { .. } = pane.pane_state() {
                weight *= 0.7;
            }

            weights.insert(*id, weight);
        }
        weights
    }

    /// Update pane layout and sizes based on current terminal size
    pub fn update_pane_layout(&mut self, terminal_size: (u16, u16)) {
        let (term_cols, term_rows) = terminal_size;

        // Calculate pane area (minus status bar)
        let pane_area = Rect::new(0, 0, term_cols, term_rows.saturating_sub(1));

        if self.layout.is_some() {
            // We need to clone layout here or handle borrow checker because calculate_pane_weights borrows self immutable
            // and layout.calculate_rects needs layout (which is in self)
            // But layout.calculate_rects takes &self.
            
            // Workaround: extract layout momentarily or use two steps
            // self.layout is Option<LayoutManager>
            
            let weights = self.calculate_pane_weights();
            
            if let Some(ref layout) = self.layout {
                let pane_rects = layout.calculate_rects(pane_area, &weights);
                
                // We need to collect pane_rects because we need to mutate self.pane_manager in the loop
                // pane_rects returns Vec, so it's already collected/owned? 
                // layout.calculate_rects returns Vec<(Uuid, Rect)>
                
                for (pane_id, rect) in pane_rects {
                    // Account for border (1 cell on each side)
                    let inner_width = rect.width.saturating_sub(2);
                    let inner_height = rect.height.saturating_sub(2);

                    // Resize the UI pane to match the calculated layout
                    self.pane_manager.resize_pane(pane_id, inner_height, inner_width);

                    // Update focus state
                    let is_active = Some(pane_id) == self.active_pane_id;
                    if let Some(ui_pane) = self.pane_manager.get_mut(pane_id) {
                        ui_pane.set_focus_state(if is_active {
                            FocusState::Focused
                        } else {
                            FocusState::Unfocused
                        });
                    }
                }
            }
        }
    }
}
