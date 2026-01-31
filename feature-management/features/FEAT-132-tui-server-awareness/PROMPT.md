# FEAT-132: TUI Server Awareness

**Priority**: P2
**Component**: fugue-client/ui
**Effort**: Medium
**Status**: new
**Depends**: FEAT-130, FEAT-131

## Summary

Update the TUI to display sessions from multiple servers with clear visual distinction, grouping, and server status indicators.

## Problem

With multi-server connections, the session list needs to show which server each session belongs to, connection status per server, and allow easy navigation between servers.

## Proposed UI

### Session List View

```
┌─ Sessions ─────────────────────────────────────────────┐
│                                                         │
│ ▼ local (3 sessions) ● connected                       │
│   ├─ nexus                    claude-opus   73k tokens │
│   ├─ fugue-orch              claude-sonnet  45k tokens │
│   └─ watchdog                claude-haiku   12k tokens │
│                                                         │
│ ▼ polecats (2 sessions) ● connected                    │
│   ├─ worker-feat-127          claude-sonnet 28k tokens │
│   └─ worker-feat-128          claude-sonnet 31k tokens │
│                                                         │
│ ▶ workstation (0 sessions) ○ disconnected              │
│                                                         │
└─────────────────────────────────────────────────────────┘
```

### Status Bar

```
[local:nexus] 73k tokens │ Servers: local● polecats● workstation○
```

### Keyboard Navigation

| Key | Action |
|-----|--------|
| `Tab` | Cycle through servers |
| `Enter` | Attach to selected session |
| `c` | Create session (on current server) |
| `C` | Create session (choose server) |
| `r` | Reconnect to disconnected server |

## Implementation

### Key Files

| File | Changes |
|------|---------|
| `fugue-client/src/ui/session_list.rs` | Grouped display, server sections |
| `fugue-client/src/ui/status_bar.rs` | Multi-server status indicators |
| `fugue-client/src/ui/app.rs` | Server selection state |
| `fugue-client/src/ui/theme.rs` | Server-specific colors |

### Server Groups

```rust
pub struct ServerGroup {
    pub name: String,
    pub status: ConnectionStatus,
    pub sessions: Vec<SessionInfo>,
    pub collapsed: bool,
}

pub enum ConnectionStatus {
    Connected,
    Connecting,
    Disconnected,
    Error(String),
}

impl SessionListWidget {
    fn render_server_group(&self, group: &ServerGroup, area: Rect, buf: &mut Buffer) {
        // Server header with expand/collapse indicator
        let indicator = if group.collapsed { "▶" } else { "▼" };
        let status_dot = match group.status {
            Connected => "●".green(),
            Connecting => "◐".yellow(),
            Disconnected => "○".dark_gray(),
            Error(_) => "●".red(),
        };

        // Render header
        let header = format!("{} {} ({} sessions) {}",
            indicator, group.name, group.sessions.len(), status_dot);

        // Render sessions if not collapsed
        if !group.collapsed {
            for session in &group.sessions {
                // Indent and render session
            }
        }
    }
}
```

### Color Scheme

Each server gets a consistent accent color for easy identification:

```rust
fn server_color(name: &str) -> Color {
    // Hash-based consistent color assignment
    let hash = name.bytes().fold(0u8, |acc, b| acc.wrapping_add(b));
    match hash % 6 {
        0 => Color::Cyan,
        1 => Color::Magenta,
        2 => Color::Yellow,
        3 => Color::Green,
        4 => Color::Blue,
        _ => Color::Red,
    }
}
```

## Acceptance Criteria

- [ ] Sessions grouped by server
- [ ] Server connection status visible (●/○)
- [ ] Collapsible server groups
- [ ] Tab to cycle servers
- [ ] Create session on chosen server
- [ ] Reconnect action for disconnected servers
- [ ] Status bar shows all server states
- [ ] Consistent color per server

## Related

- FEAT-130: Multi-connection client (provides connection state)
- FEAT-131: Namespaced sessions (display format)
