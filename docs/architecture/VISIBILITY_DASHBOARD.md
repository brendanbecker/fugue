# fugue Visibility Dashboard

High-level design for oversight features addressing multi-agent tracking pain (progress, stuck tasks, relationships, review queue).

## Goals
- Give orchestrator at-a-glance visibility into swarm state without tab-switching.
- Solve Discord feedback: tracking work/relationships, detecting stuck/swirling paths, mailbox for outputs, intervention cues.
- Keep it terminal-native (Ratatui widgets, no external UI).

## Core Components
1. **Stuck / swirl detection**  
   - Flag panes: Thinking > timeout (e.g., 2min), repeated failed ToolUse cycles, no progress in N seconds.  
   - Visual: red/yellow badge in pane list or status bar.

2. **Mailbox / output inbox**  
   - Dedicated widget (Ratatui List/Table) collecting worker summaries/alerts.  
   - Workers send via sideband: `<fugue:mail priority="info/warn/error" summary="Task X complete">`.  
   - Fields: from_pane, timestamp, summary, priority (color-coded).  
   - Select → jump to pane or expand full log.

3. **Diagram / info pane (Canvas-based)**  
   - Spawnable "info" pane showing dependency graph or progress map.  
   - Nodes: panes/tasks (Rectangles/Circles colored by status: green=good, yellow=long, red=stuck).  
   - Edges: Lines for relationships (from beads or MCP reports).  
   - Dynamic redraw on state changes.  
   - Simple grid positioning (layer/depth from gastown data).

## Flows

### Stuck Detection
Worker pane → Thinking > timeout → fugue flags → red badge in TUI + mailbox entry "Pane-5 stuck in Thinking 3min"

### Mailbox
Worker → MCP mail "Task complete: refactored auth" → orchestrator inbox updates → user selects → jumps to pane

### Diagram Pane
Orchestrator → MCP spawn info-pane → Canvas redraws graph:  
- Node A (green) → edge → Node B (yellow) → edge → Node C (red, flagged stuck)

## Tradeoffs
| Aspect              | Pro                                         | Con                                      |
|---------------------|---------------------------------------------|------------------------------------------|
| Visibility          | At-a-glance swarm overview                  | Adds TUI complexity                      |
| Token burn          | Minimal (local TUI rendering)               | Redraws could lag on slow terminals      |
| User intervention   | Clear cues for stuck tasks                  | Requires users to watch dashboard        |
| Extensibility       | Easy to add more widgets (list, graph)      | Layout positioning needs manual logic    |

## Implementation Notes
- Start with mailbox (List widget) + stuck detection (state timeout check).
- Then add Canvas diagram (static test graph first, dynamic later).
- Use existing MCP broadcast for notifications.
- Acceptance: Demo shows stuck flag + mailbox entry + basic graph.

Next: FEAT-063/064/065 implementation.
