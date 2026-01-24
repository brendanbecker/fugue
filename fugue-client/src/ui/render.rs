use std::time::Instant;

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::canvas::{Canvas, Rectangle};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};

use ccmux_protocol::{
    MailPriority, PaneState, PaneStuckStatus,
};

use super::pane::render_pane;
use super::status_pane::render_status_pane;
use super::state::{AppState, ClientState, ViewMode};

/// Draw the UI
pub fn draw(state: &mut ClientState, frame: &mut ratatui::Frame, input_status: &str) {
    let area = frame.area();

    match state.state {
        AppState::Disconnected => draw_disconnected(state, frame, area),
        AppState::Connecting => draw_connecting(state, frame, area),
        AppState::SessionSelect => draw_session_select(state, frame, area),
        AppState::Attached => draw_attached(state, frame, area, input_status),
        AppState::Quitting => {} 
    }
}

/// Draw disconnected state
fn draw_disconnected(state: &ClientState, frame: &mut ratatui::Frame, area: Rect) {
    let message = state.status_message.as_deref().unwrap_or("Disconnected");
    let paragraph = Paragraph::new(message)
        .style(Style::default().fg(Color::Red))
        .block(Block::default().borders(Borders::ALL).title("ccmux"));
    frame.render_widget(paragraph, area);
}

/// Draw connecting state
fn draw_connecting(state: &ClientState, frame: &mut ratatui::Frame, area: Rect) {
    let dots = ".".repeat(((state.tick_count / 5) % 4) as usize);
    let message = format!("Connecting{}", dots);
    let paragraph = Paragraph::new(message)
        .style(Style::default().fg(Color::Yellow))
        .block(Block::default().borders(Borders::ALL).title("ccmux"));
    frame.render_widget(paragraph, area);
}

/// Draw session select state
fn draw_session_select(state: &mut ClientState, frame: &mut ratatui::Frame, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(5), Constraint::Length(3)])
        .split(area);

    if state.available_sessions.is_empty() {
        // Show empty state message
        let empty_msg = Paragraph::new("No sessions available. Press 'n' to create one.")
            .style(Style::default().fg(Color::DarkGray))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Select Session")
                    .border_style(Style::default().fg(Color::Cyan)),
            );
        frame.render_widget(empty_msg, chunks[0]);
    } else {
        // Build list items with session metadata
        let items: Vec<ListItem> = state.available_sessions
            .iter()
            .map(|session| {
                let worktree_info = session
                    .worktree
                    .as_ref()
                    .map(|w| format!(" [{}]", w.path))
                    .unwrap_or_default();
                let orchestrator_badge = if session.has_tag("orchestrator") { " ★" } else { "" };
                ListItem::new(format!(
                    "{}{} ({} windows, {} clients){}",
                    session.name,
                    orchestrator_badge,
                    session.window_count,
                    session.attached_clients,
                    worktree_info
                ))
            })
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Select Session")
                    .border_style(Style::default().fg(Color::Cyan)),
            )
            .highlight_style(
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("> ");

        // Create ListState with current selection
        let mut list_state = ListState::default();
        list_state.select(Some(state.session_list_index));

        frame.render_stateful_widget(list, chunks[0], &mut list_state);
    }

    // Help line with j/k mentioned
    let help = Paragraph::new("↑/k ↓/j: navigate | Enter: attach | n: new | r: refresh | Ctrl+D: delete | q: quit")
        .style(Style::default().fg(Color::DarkGray))
        .block(Block::default().borders(Borders::ALL).title("Help"));
    frame.render_widget(help, chunks[1]);
}

/// Draw attached state (main pane view)
fn draw_attached(state: &mut ClientState, frame: &mut ratatui::Frame, area: Rect, input_status: &str) {
    match state.view_mode {
        ViewMode::Panes => draw_panes(state, frame, area, input_status),
        ViewMode::Dashboard => draw_dashboard(state, frame, area, input_status),
    }
}

/// Draw the normal pane view
fn draw_panes(state: &mut ClientState, frame: &mut ratatui::Frame, area: Rect, input_status: &str) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(3), Constraint::Length(1)])
        .split(area);

    // Main pane area
    let pane_area = chunks[0];

    // Render all panes using layout manager
    if let Some(ref layout) = state.layout {
        let weights = state.calculate_pane_weights();
        let pane_rects = layout.calculate_rects(pane_area, &weights);

        // Render each pane
        for (pane_id, rect) in &pane_rects {
            if let Some(ui_pane) = state.pane_manager.get(*pane_id) {
                if let PaneState::Status = ui_pane.pane_state() {
                    render_status_pane(ui_pane, *rect, frame.buffer_mut(), state);
                } else {
                    render_pane(ui_pane, *rect, frame.buffer_mut(), state.tick_count);
                }
            } else {
                // Fallback if UI pane not found
                let pane_block = Block::default()
                    .borders(Borders::ALL)
                    .title("Pane (no terminal)")
                    .border_style(Style::default().fg(Color::Red));
                let pane_widget = Paragraph::new("Terminal not initialized")
                    .block(pane_block);
                frame.render_widget(pane_widget, *rect);
            }
        }
    } else if let Some(pane_id) = state.active_pane_id {
        // Fallback: single pane, no layout, render single active pane
        if let Some(ui_pane) = state.pane_manager.get(pane_id) {
            if let PaneState::Status = ui_pane.pane_state() {
                render_status_pane(ui_pane, pane_area, frame.buffer_mut(), state);
            } else {
                render_pane(ui_pane, pane_area, frame.buffer_mut(), state.tick_count);
            }
        } else {
            let pane_block = Block::default()
                .borders(Borders::ALL)
                .title("Pane (no terminal)")
                .border_style(Style::default().fg(Color::Red));
            let pane_widget = Paragraph::new("Terminal not initialized")
                .block(pane_block);
            frame.render_widget(pane_widget, pane_area);
        }
    } else {
        // No active pane
        let pane_block = Block::default()
            .borders(Borders::ALL)
            .title("No Pane")
            .border_style(Style::default().fg(Color::DarkGray));
        let pane_widget = Paragraph::new("No active pane").block(pane_block);
        frame.render_widget(pane_widget, pane_area);
    }

    // Status bar
    let status = build_status_bar(state, input_status);
    let status_widget = Paragraph::new(status).style(Style::default().bg(Color::DarkGray));
    frame.render_widget(status_widget, chunks[1]);
}

/// Draw the visibility dashboard
fn draw_dashboard(state: &mut ClientState, frame: &mut ratatui::Frame, area: Rect, input_status: &str) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(3), Constraint::Length(1)])
        .split(area);

    let dashboard_area = chunks[0];
    
    let dashboard_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(dashboard_area);

    // Mailbox widget (Left)
    draw_mailbox(state, frame, dashboard_chunks[0]);

    // System Graph (Right)
    draw_system_graph(state, frame, dashboard_chunks[1]);

    // Status bar
    let status = build_status_bar(state, input_status);
    let status_widget = Paragraph::new(status).style(Style::default().bg(Color::DarkGray));
    frame.render_widget(status_widget, chunks[1]);
}

/// Draw the system graph widget
fn draw_system_graph(state: &ClientState, frame: &mut ratatui::Frame, area: Rect) {
    let canvas = Canvas::default()
        .block(Block::default().borders(Borders::ALL).title("System Graph"))
        .x_bounds([0.0, 100.0])
        .y_bounds([0.0, 100.0])
        .paint(|ctx| {
            // Draw nodes for each pane
            let panes: Vec<_> = state.panes.values().collect();
            let num_panes = panes.len();
            
            if num_panes == 0 {
                ctx.print(40.0, 50.0, "No active panes");
                return;
            }

            // Simple grid layout for nodes
            let cols = (num_panes as f64).sqrt().ceil() as usize;
            let rows = (num_panes as f64 / cols as f64).ceil() as usize;
            
            let cell_width = 100.0 / cols as f64;
            let cell_height = 100.0 / rows as f64;

            for (i, pane) in panes.iter().enumerate() {
                let r = i / cols;
                let c = i % cols;

                let x = c as f64 * cell_width + cell_width / 2.0;
                let y = 100.0 - (r as f64 * cell_height + cell_height / 2.0);

                let color = match &pane.stuck_status {
                    Some(PaneStuckStatus::Stuck { .. }) => Color::Red,
                    Some(PaneStuckStatus::Slow { .. }) => Color::Yellow,
                    _ => match &pane.state {
                        PaneState::Normal => Color::Green,
                        PaneState::Agent(agent_state) => {
                            if matches!(agent_state.activity, ccmux_protocol::AgentActivity::Idle) {
                                Color::Blue
                            } else {
                                Color::Cyan
                            }
                        }
                        PaneState::Exited { .. } => Color::Gray,
                        PaneState::Status => Color::Blue,
                    }
                };

                // Draw node as a rectangle
                ctx.draw(&Rectangle {
                    x: x - 5.0,
                    y: y - 5.0,
                    width: 10.0,
                    height: 10.0,
                    color,
                });

                // Print pane label
                let label = pane.name.as_deref().unwrap_or_else(|| {
                    pane.title.as_deref().unwrap_or("?")
                });
                let short_label = if label.len() > 8 { &label[0..8] } else { label };
                ctx.print(x - 4.0, y - 2.0, short_label.to_string());
            }
        });

    frame.render_widget(canvas, area);
}

/// Draw the mailbox widget
fn draw_mailbox(state: &mut ClientState, frame: &mut ratatui::Frame, area: Rect) {
    let items: Vec<ListItem> = state.mailbox.iter().rev().map(|msg| {
        let priority_style = match msg.priority {
            MailPriority::Info => Style::default().fg(Color::Cyan),
            MailPriority::Warning => Style::default().fg(Color::Yellow),
            MailPriority::Error => Style::default().fg(Color::Red),
        };

        let pane_name = state.panes.get(&msg.pane_id)
            .and_then(|p| p.name.as_ref())
            .cloned()
            .unwrap_or_else(|| format!("Pane {}", state.panes.get(&msg.pane_id).map(|p| p.index).unwrap_or(0)));

        ListItem::new(format!("[{}] {}", pane_name, msg.summary)).style(priority_style)
    }).collect();

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Mailbox"))
        .highlight_style(Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD))
        .highlight_symbol("> ");

    frame.render_stateful_widget(list, area, &mut state.mailbox_state);
}

/// Build status bar content
fn build_status_bar(state: &ClientState, mode_indicator: &str) -> String {
    let session_name = state.session
        .as_ref()
        .map(|s| s.name.as_str())
        .unwrap_or("No session");

    #[allow(deprecated)]
    let pane_info = if let Some(pane_id) = state.active_pane_id {
        if let Some(pane) = state.panes.get(&pane_id) {
            // Check stuck status first (overrides normal indicator)
            if let Some(stuck) = &pane.stuck_status {
                match stuck {
                    PaneStuckStatus::Stuck { duration, .. } => {
                        format!("[STUCK {}s]", duration)
                    }
                    PaneStuckStatus::Slow { duration } => {
                        format!("[SLOW {}s]", duration)
                    }
                    PaneStuckStatus::None => match &pane.state {
                        PaneState::Normal => "[ ]".to_string(),
                        PaneState::Agent(agent_state) => {
                            format_agent_indicator(&agent_state.agent_type, &agent_state.activity, state.tick_count)
                        }
                        PaneState::Exited { code } => format!("[Exit:{}]", code.unwrap_or(-1)),
                        PaneState::Status => "[Status]".to_string(),
                    },
                }
            } else {
                match &pane.state {
                    PaneState::Normal => "[ ]".to_string(),
                    PaneState::Agent(agent_state) => {
                        format_agent_indicator(&agent_state.agent_type, &agent_state.activity, state.tick_count)
                    }
                    PaneState::Exited { code } => format!("[Exit:{}]", code.unwrap_or(-1)),
                    PaneState::Status => "[Status]".to_string(),
                }
            }
        } else {
            "".to_string()
        }
    } else {
        "".to_string()
    };

    // FEAT-057/058: Beads indicator with ready count
    let beads_indicator = if state.is_beads_tracked {
        match state.beads_ready_count {
            Some(0) => " | bd:0".to_string(),
            Some(count) => format!(" | bd:{}", count),
            None => " | beads".to_string(),
        }
    } else {
        "".to_string()
    };

    // FEAT-077: Human control indicator
    let human_control_indicator = if let Some(expiry) = state.human_control_lock_expiry {
        let now = Instant::now();
        if expiry > now {
            let remaining = expiry.duration_since(now).as_secs_f32();
            format!(" [LOCKED: {:.1}s]", remaining)
        } else {
            "".to_string()
        }
    } else {
        "".to_string()
    };

    format!(
        " {} | {} panes {}{}{}{}",
        session_name,
        state.panes.len(),
        pane_info,
        beads_indicator,
        mode_indicator,
        human_control_indicator
    )
}

/// Format agent activity indicator with animation (FEAT-084)
fn format_agent_indicator(agent_type: &str, activity: &ccmux_protocol::AgentActivity, tick: u64) -> String {
    let prefix = agent_type.chars().next().unwrap_or('A').to_uppercase().to_string();
    match activity {
        ccmux_protocol::AgentActivity::Idle => format!("[{}]", prefix),
        ccmux_protocol::AgentActivity::Processing => {
            let frames = ["[.  ]", "[.. ]", "[... ]", "[ ..]", "[  .]", "[   ]"];
            format!("{}{}", prefix, frames[(tick / 3) as usize % frames.len()])
        }
        ccmux_protocol::AgentActivity::Generating => format!(">[{}]", prefix),
        ccmux_protocol::AgentActivity::ToolUse => format!("[{}*]", prefix),
        ccmux_protocol::AgentActivity::AwaitingConfirmation => format!("[{}?]", prefix),
        ccmux_protocol::AgentActivity::Custom(name) => format!("[{}:{}]", prefix, &name[..name.len().min(3)]),
    }
}
