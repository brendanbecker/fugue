# FEAT-073: Visibility dashboard (stuck detection, mailbox, graph pane)

## Overview
Add a terminal-native visibility dashboard for multi-agent oversight: stuck/swirl detection with badges, a mailbox/inbox widget for worker summaries via sideband, and an optional canvas graph pane showing relationships/status.

## Motivation
Operators need at-a-glance visibility into multi-agent progress, blocked/stuck work, and relationships without constant tab switching. A dashboard improves intervention speed and reduces confusion about agent state.

## Requirements
- Stuck/swirl detection flags panes after configurable timeouts or repeated tool cycles.
- Badges surface stuck/slow status in the pane list or status bar.
- Mailbox widget aggregates worker summaries sent via sideband messages with priority and timestamps.
- Selecting mailbox entries can jump to the source pane and expand details.
- **Activity Feed pane to view the stream of intent/events (decoded from WAL/Broadcasts).**
- Optional graph/info pane renders a simple relationship/status map via Ratatui Canvas.
- Dashboard stays terminal-native and performant for frequent updates.

## Design
Based on `docs/architecture/VISIBILITY_DASHBOARD.md`.
- Mailbox widget uses a sideband message format like `<fugue:mail priority="info|warn|error" summary="...">`.
- **Activity Feed listens to server broadcasts/events and renders a scrolling log of "Intents" (e.g., "Agent X focused Pane Y", "User Z created Window W").**
- Graph pane renders nodes (panes/tasks) colored by status and edges for relationships.
- Start with mailbox + stuck detection, then add a static graph and evolve to dynamic updates.

## Tasks
### Section 1: Stuck detection + badges
- [ ] Define stuck/swirl criteria (timeouts, tool-use loops, no progress).
- [ ] Add badge status to pane list/status bar.
- [ ] Emit mailbox entries when a pane becomes stuck.

### Section 2: Mailbox widget
- [ ] Parse sideband mail messages into a mailbox model.
- [ ] Render mailbox widget with priority color-coding and timestamps.
- [ ] Add interaction: select entry to jump/expand.

### Section 3: Activity Feed
- [ ] Create a new pane type or widget for the Activity Feed.
- [ ] Subscribe to `SessionState` or `Event` broadcasts.
- [ ] Render decoded events as a human-readable log stream.

### Section 4: Graph/info pane
- [ ] Add spawnable info/graph pane using Ratatui Canvas.
- [ ] Render nodes/edges with simple grid layout.
- [ ] Redraw on relevant state changes.

## Acceptance Criteria
- [ ] Stuck panes are visibly flagged with badges.
- [ ] Mailbox shows worker summaries and supports navigation.
- [ ] Graph pane renders pane/task relationships with status colors.
- [ ] Dashboard updates do not noticeably degrade TUI performance.

## Testing
- [ ] Unit tests for stuck detection criteria.
- [ ] Unit tests for mailbox parsing and sorting.
- [ ] Manual TUI validation for badge rendering and graph pane updates.

## Dependencies
- None
