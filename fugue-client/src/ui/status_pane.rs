//! Status Pane Rendering (FEAT-102)
//!
//! Renders a dedicated status pane showing real-time agent activity across all sessions.

use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph, Widget};

use fugue_protocol::{AgentActivity, PaneState};

use super::pane::Pane;
use super::state::ClientState;

/// Render the status pane
pub fn render_status_pane(pane: &Pane, area: Rect, buf: &mut Buffer, state: &ClientState) {
    // 1. Draw the border block
    let title = pane.display_title();
    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .border_style(Style::default().fg(Color::Blue));
    
    let inner_area = block.inner(area);
    block.render(area, buf);

    // 2. Split into Agent List (top/left) and Activity Feed (bottom/right)
    // For now, simple vertical split: Top 60% agents, Bottom 40% feed
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(inner_area);

    // 3. Render Agent List
    render_agent_list(state, chunks[0], buf);

    // 4. Render Activity Feed
    render_activity_feed(state, chunks[1], buf);
}

fn render_agent_list(state: &ClientState, area: Rect, buf: &mut Buffer) {
    let block = Block::default()
        .borders(Borders::BOTTOM)
        .title("Agents");
    
    let list_area = block.inner(area);
    block.render(area, buf);

    let mut items = Vec::new();

    // Iterate over all panes in current session (and maybe others if we had global visibility)
    // state.panes only contains panes in the current session.
    // If we want global visibility, we need data from `fugue_list_panes` equivalent.
    // For now, we use state.panes which is what we have.
    
    let mut panes: Vec<_> = state.panes.values().collect();
    // Sort by index
    panes.sort_by_key(|p| p.index);

    for pane_info in panes {
        // Skip the status pane itself
        if let PaneState::Status = pane_info.state {
            continue;
        }

        let name = pane_info.name.as_deref().unwrap_or("Pane");
        let (status_str, style) = match &pane_info.state {
            PaneState::Normal => ("Idle", Style::default().fg(Color::Blue)),
            PaneState::Agent(agent_state) => match agent_state.activity {
                AgentActivity::Idle => ("Idle", Style::default().fg(Color::Blue)),
                AgentActivity::Processing => ("Thinking...", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                AgentActivity::Generating => ("Generating", Style::default().fg(Color::Green)),
                AgentActivity::ToolUse => ("Tool Use", Style::default().fg(Color::Yellow)),
                AgentActivity::AwaitingConfirmation => ("Waiting Input", Style::default().fg(Color::Magenta)),
                AgentActivity::Custom(ref s) => (s.as_str(), Style::default().fg(Color::White)),
            },
            PaneState::Exited { code } => {
                if code == &Some(0) {
                    ("Exited (0)", Style::default().fg(Color::DarkGray))
                } else {
                    ("Failed", Style::default().fg(Color::Red))
                }
            }
            PaneState::Status => continue, 
        };

        let line = Line::from(vec![
            Span::styled(format!("{:<10}", name), Style::default().fg(Color::White)),
            Span::raw(" "),
            Span::styled(status_str, style),
        ]);

        items.push(ListItem::new(line));
    }

    if items.is_empty() {
        let p = Paragraph::new("No active agents")
            .style(Style::default().fg(Color::DarkGray));
        p.render(list_area, buf);
    } else {
        let list = List::new(items);
        list.render(list_area, buf);
    }
}

fn render_activity_feed(state: &ClientState, area: Rect, buf: &mut Buffer) {
    let block = Block::default()
        .title("Activity Feed");
    
    let feed_area = block.inner(area);
    block.render(area, buf);

    // Placeholder for activity feed
    // In a real implementation, we would need a list of recent events in ClientState.
    // For now, we can show mailbox messages as a proxy.
    
    let items: Vec<ListItem> = state.mailbox.iter().rev().take(10).map(|msg| {
        let style = match msg.priority {
            _c => Style::default().fg(Color::White), // Default
        };
        ListItem::new(format!("> {}", msg.summary)).style(style)
    }).collect();

    if items.is_empty() {
        let p = Paragraph::new("No recent activity")
            .style(Style::default().fg(Color::DarkGray));
        p.render(feed_area, buf);
    } else {
        let list = List::new(items);
        list.render(feed_area, buf);
    }
}
