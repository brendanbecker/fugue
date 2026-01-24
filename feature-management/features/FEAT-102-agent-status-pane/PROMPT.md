# FEAT-102: Agent Status Pane

## Summary

A dedicated status pane that displays real-time agent activity across all fugue sessions. Unlike the full-screen dashboard (FEAT-073), this is a **pane** that lives alongside other panes, enabling "orchestrator + status view" layouts for multi-agent supervision.

## Problem Statement

When orchestrating multiple AI agents across fugue sessions, the orchestrator needs continuous visibility into agent states without switching away from their working context. The existing dashboard (prefix + D) is a full-screen view mode that replaces the terminal view, forcing a context switch.

**Target Layout:**
```
┌─────────────────────────┬─────────────────────────┐
│                         │                         │
│   Orchestrator Pane     │   Agent Status Pane     │
│   (Claude/user work)    │   (live agent feed)     │
│                         │                         │
└─────────────────────────┴─────────────────────────┘
```

## Requirements

### Core Display Elements

1. **Agent List Panel**
   - All sessions with detected agents (Claude, Gemini, Codex)
   - Session name and tags
   - Current cognitive state with duration timer:
     - Idle (blue)
     - Thinking (cyan, animated)
     - Tool use (yellow)
     - Waiting for input (orange)
     - Error (red)
   - Beads issue assignment (if any)

2. **Activity Feed Panel**
   - Scrolling log of recent agent actions
   - Timestamps
   - Action types: file edits, bash commands, tool calls
   - Truncated content preview

3. **Quick Stats Header**
   - Total agents: X active, Y idle, Z waiting
   - Issues in progress (from beads metadata)

### Interaction

- **Keyboard navigation**: Arrow keys to select agent, Enter to focus that pane
- **Auto-scroll**: Activity feed scrolls automatically, pauses on hover/selection
- **Refresh rate**: Configurable polling interval (default 500ms)

### Data Sources

All data comes from existing fugue primitives (no new protocol required):

| Data | Source |
|------|--------|
| Agent list | `fugue_list_panes` (filter `is_claude` or `state: agent`) |
| Cognitive state | `PaneInfo.state` → `AgentState.activity` |
| Session tags | `fugue_get_tags` |
| Beads assignment | Session metadata (`beads.current_issue`) |
| Recent output | `fugue_read_pane` (tail) |

### MCP Tool

```json
{
  "name": "fugue_create_status_pane",
  "description": "Create an agent status pane showing real-time activity across all sessions",
  "inputSchema": {
    "type": "object",
    "properties": {
      "position": {
        "type": "string",
        "enum": ["left", "right", "top", "bottom"],
        "default": "right",
        "description": "Where to place the status pane relative to current pane"
      },
      "width_percent": {
        "type": "integer",
        "default": 40,
        "description": "Width of status pane as percentage (10-90)"
      },
      "show_activity_feed": {
        "type": "boolean",
        "default": true,
        "description": "Include scrolling activity feed"
      },
      "show_output_preview": {
        "type": "boolean",
        "default": false,
        "description": "Show last few lines of each agent's output"
      },
      "filter_tags": {
        "type": "array",
        "items": {"type": "string"},
        "description": "Only show agents with these tags (empty = all)"
      }
    }
  }
}
```

## Implementation Approach

### Option A: Special Pane Type (Recommended)

Add a new pane type `StatusPane` that renders agent status instead of PTY output.

**Pros:**
- Native TUI rendering with full ratatui capabilities
- Efficient - no PTY overhead
- Keyboard navigation built-in

**Cons:**
- New pane type requires protocol changes
- Client-side rendering logic

### Option B: Widget-Based Sideband

Use the existing widget system (FEAT-083) to push status data to a designated pane.

**Pros:**
- Uses existing infrastructure
- Server-driven updates

**Cons:**
- Limited to widget rendering capabilities
- May need widget enhancements

### Option C: Self-Updating Script Pane

Create a regular pane running a status script that polls and renders.

**Pros:**
- No core changes needed
- Could be a separate binary (`fugue-status`)

**Cons:**
- Higher resource usage (polling loop)
- Less integrated feel

## Acceptance Criteria

1. [ ] `fugue_create_status_pane` tool creates a split pane showing agent status
2. [ ] Status pane displays all detected agents with cognitive state
3. [ ] Cognitive state includes duration timer (e.g., "Thinking (45s)")
4. [ ] Activity feed shows recent tool calls and file operations
5. [ ] Beads issue assignment shown when present
6. [ ] Arrow keys navigate agent list, Enter focuses selected agent's pane
7. [ ] Status updates at configurable interval without flicker
8. [ ] Works with local sessions (not dependent on Gastown/remote)

## Non-Goals

- Real-time output streaming (use mirror pane for that)
- Remote-only features (must work locally)
- Replacing the full-screen dashboard (complementary feature)

## Dependencies

- FEAT-073: Visibility dashboard (existing patterns to reuse)
- FEAT-083: Generic widget system (potential Option B)
- FEAT-098: Gemini agent detection (multi-agent support)

## Estimated Effort

Medium - 2-3 sessions depending on implementation approach.

## Open Questions

1. Should status pane auto-hide non-agent panes, or show all panes with agent highlighting?
2. Include output preview (last N lines) per agent, or keep it minimal?
3. Support multiple status panes (e.g., filtered by tag)?
