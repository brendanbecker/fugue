# FEAT-058: Beads Query Integration - TUI Visibility into Work Queue

**Priority**: P3
**Component**: ccmux-server, ccmux-client
**Type**: new_feature
**Estimated Effort**: large
**Business Value**: medium
**Technical Complexity**: medium
**Status**: new

## Overview

Add beads query integration to ccmux's TUI, providing visibility into the beads work queue directly in the terminal multiplexer interface. This enables users and agents to see pending tasks at a glance without shelling out to beads CLI commands.

## Problem Statement

When working in beads-tracked repositories, users and agents must shell out to `bd ready`, `bd list`, etc. to see the work queue. There's no at-a-glance visibility into pending tasks, which breaks flow and requires context switching.

Current workflow:
```
User: Working in ccmux pane
User: Wonders what tasks are ready
User: Opens new terminal or shells out: bd ready
User: Reads output, switches back to work
User: Loses context, flow interrupted
```

Desired workflow:
```
User: Working in ccmux pane
User: Glances at status bar: "bd: 3 ready"
User: Presses Ctrl+B b to see full list
User: Selects task, continues working
```

## Architecture

```
+-------------+                              +---------------+
|  TUI Client | <-- status updates --------- |    Server     |
|             |                              |               |
| Status bar: |                              | BeadsClient   |
| "bd: 3 rdy" | -- Ctrl+B b popup request -> | per pane/repo |
|             |                              |               |
| Beads panel | <-- ready list response ---- +-------+-------+
+-------------+                                      |
                                                     | RPC
                                                     v
                                              +-------------+
                                              | .beads/     |
                                              | bd.sock     |
                                              | (daemon)    |
                                              +-------------+
```

## Requirements

### Part 1: Beads Daemon Socket Connection

Establish connection to beads daemon for each pane's working directory:

```rust
// ccmux-server/src/beads/client.rs

pub struct BeadsClient {
    socket_path: PathBuf,
    connection: Option<UnixStream>,
    last_error: Option<BeadsError>,
}

impl BeadsClient {
    /// Discover .beads/bd.sock in repo root
    pub fn discover(working_dir: &Path) -> Option<PathBuf> {
        // Walk up from working_dir looking for .beads/bd.sock
        let mut current = working_dir;
        loop {
            let socket = current.join(".beads/bd.sock");
            if socket.exists() {
                return Some(socket);
            }
            current = current.parent()?;
        }
    }

    /// Connect to daemon socket
    pub async fn connect(&mut self) -> Result<(), BeadsError> {
        let stream = UnixStream::connect(&self.socket_path).await?;
        self.connection = Some(stream);
        Ok(())
    }

    /// Query ready tasks
    pub async fn query_ready(&mut self) -> Result<Vec<BeadsTask>, BeadsError> {
        // Send RPC request to daemon
        // Parse response
    }

    /// Check daemon health/availability
    pub fn is_available(&self) -> bool {
        self.connection.is_some() && self.last_error.is_none()
    }
}
```

**Fallback behavior**: When daemon is unavailable:
- Status bar shows "bd: --" or nothing
- Popup shows "Daemon unavailable"
- No blocking operations

### Part 2: Status Bar Work Queue Indicator

Add beads status to the status bar display:

```rust
// ccmux-client/src/ui/status.rs

pub struct BeadsStatus {
    ready_count: usize,
    highest_priority_title: Option<String>,
    daemon_available: bool,
}

impl StatusBar {
    fn render_beads_status(&self, area: Rect, buf: &mut Buffer) {
        if !self.beads.daemon_available {
            // Don't show anything if no daemon
            return;
        }

        let text = if self.beads.ready_count == 0 {
            "bd: 0 ready".to_string()
        } else {
            format!("bd: {} ready", self.beads.ready_count)
        };

        // Render with appropriate styling
        // Green if 0 ready, yellow if >0, red if daemon error
    }
}
```

**Status bar content**:
- `bd: 0 ready` - No pending tasks (green)
- `bd: 3 ready` - 3 tasks pending (yellow)
- `bd: --` - Daemon unavailable (dim)

### Part 3: Beads Popup Panel (Ctrl+B b)

New overlay panel showing beads ready list:

```rust
// ccmux-client/src/ui/beads_panel.rs

pub struct BeadsPanel {
    tasks: Vec<BeadsTask>,
    selected: usize,
    loading: bool,
    error: Option<String>,
}

pub struct BeadsTask {
    pub id: String,           // e.g., "BUG-042"
    pub title: String,
    pub priority: String,     // "P0", "P1", etc.
    pub component: String,
    pub status: String,
}

impl BeadsPanel {
    pub fn render(&self, area: Rect, buf: &mut Buffer) {
        // Title bar: "Ready Tasks (3)"
        // Table: ID | Priority | Title | Component
        // Footer: [Enter] Claim  [v] View  [Esc] Close
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> Option<BeadsPanelAction> {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => self.select_previous(),
            KeyCode::Down | KeyCode::Char('j') => self.select_next(),
            KeyCode::Enter => Some(BeadsPanelAction::Claim(self.selected_task())),
            KeyCode::Char('v') => Some(BeadsPanelAction::ViewDetails(self.selected_task())),
            KeyCode::Esc => Some(BeadsPanelAction::Close),
            _ => None,
        }
    }
}

pub enum BeadsPanelAction {
    Close,
    Claim(String),        // Task ID to claim
    ViewDetails(String),  // Task ID to view
}
```

**Panel features**:
- Show all tasks from `bd ready`
- Navigate with j/k or arrow keys
- Enter to claim task
- v to view task details
- Esc to close

### Part 4: Query MCP Tools (Optional, Lower Priority)

Add MCP tools for agent access to beads data:

```rust
// ccmux-server/src/mcp/handlers.rs

/// Get ready tasks for the current pane's repository
async fn ccmux_beads_ready(&self, params: BeadsReadyParams) -> Result<BeadsReadyResponse> {
    let pane = self.get_pane(params.pane_id)?;
    let working_dir = pane.working_directory();

    let client = self.get_beads_client(working_dir)?;
    let tasks = client.query_ready().await?;

    Ok(BeadsReadyResponse {
        tasks,
        daemon_available: true,
    })
}

/// Get status of a specific issue
async fn ccmux_beads_status(&self, params: BeadsStatusParams) -> Result<BeadsStatusResponse> {
    let pane = self.get_pane(params.pane_id)?;
    let working_dir = pane.working_directory();

    let client = self.get_beads_client(working_dir)?;
    let status = client.query_status(&params.issue_id).await?;

    Ok(BeadsStatusResponse { status })
}
```

**Note**: These MCP tools query the beads daemon. They do NOT duplicate beads functionality - they provide ccmux-context-aware access to beads data.

### Part 5: Configuration

```toml
# ~/.config/ccmux/config.toml

[beads.query]
# Enable daemon connection (default: true)
enabled = true

# Show ready count in status bar
show_ready_count = true

# Refresh interval for status bar (seconds)
refresh_interval = 30

# Socket connection timeout (ms)
socket_timeout = 1000

# Auto-discover .beads in pane working directories
auto_discover = true
```

```rust
// ccmux-server/src/config.rs

#[derive(Debug, Clone, Deserialize)]
pub struct BeadsQueryConfig {
    #[serde(default = "default_enabled")]
    pub enabled: bool,

    #[serde(default = "default_show_ready_count")]
    pub show_ready_count: bool,

    #[serde(default = "default_refresh_interval")]
    pub refresh_interval: u32,  // seconds

    #[serde(default = "default_socket_timeout")]
    pub socket_timeout: u32,    // ms

    #[serde(default = "default_auto_discover")]
    pub auto_discover: bool,
}

fn default_enabled() -> bool { true }
fn default_show_ready_count() -> bool { true }
fn default_refresh_interval() -> u32 { 30 }
fn default_socket_timeout() -> u32 { 1000 }
fn default_auto_discover() -> bool { true }
```

## Files Affected

| File | Changes |
|------|---------|
| `ccmux-server/src/beads/mod.rs` | New module for beads integration |
| `ccmux-server/src/beads/client.rs` | RPC client for daemon socket |
| `ccmux-server/src/beads/discovery.rs` | Socket discovery logic |
| `ccmux-server/src/beads/types.rs` | BeadsTask and related types |
| `ccmux-server/src/config.rs` | Add beads.query configuration |
| `ccmux-server/src/mcp/handlers.rs` | Add ccmux_beads_* tools (optional) |
| `ccmux-client/src/ui/status.rs` | Status bar ready count |
| `ccmux-client/src/ui/beads_panel.rs` | New beads popup panel |
| `ccmux-client/src/input/mod.rs` | Add Ctrl+B b keybind |
| `ccmux-protocol/src/lib.rs` | Add BeadsStatus message types |

## Implementation Tasks

### Section 1: Beads Client Module
- [ ] Create `ccmux-server/src/beads/mod.rs` module
- [ ] Implement socket discovery (walk up directory tree)
- [ ] Implement async Unix socket connection
- [ ] Implement RPC protocol for daemon queries
- [ ] Add connection pooling/caching per repo
- [ ] Implement graceful fallback when daemon unavailable

### Section 2: Status Bar Integration
- [ ] Add BeadsStatus struct to protocol
- [ ] Server sends beads status updates to clients
- [ ] Client renders status in status bar
- [ ] Implement configurable refresh interval
- [ ] Handle daemon unavailable state

### Section 3: Beads Panel
- [ ] Create BeadsPanel component
- [ ] Implement task list rendering
- [ ] Implement keyboard navigation
- [ ] Add Ctrl+B b keybind to open panel
- [ ] Implement claim action
- [ ] Implement view details action

### Section 4: Configuration
- [ ] Add BeadsQueryConfig struct
- [ ] Add to main config loading
- [ ] Honor enabled flag
- [ ] Honor refresh_interval
- [ ] Honor socket_timeout

### Section 5: MCP Tools (Optional)
- [ ] Implement ccmux_beads_ready tool
- [ ] Implement ccmux_beads_status tool
- [ ] Add tool schemas
- [ ] Test with Claude

### Section 6: Testing
- [ ] Unit tests for socket discovery
- [ ] Unit tests for RPC client
- [ ] Integration tests with mock daemon
- [ ] Test fallback behavior
- [ ] Test status bar rendering
- [ ] Test panel keyboard navigation

## Acceptance Criteria

- [ ] Status bar shows ready task count when in beads repo
- [ ] Daemon connection established when socket available
- [ ] Graceful fallback when daemon unavailable (no errors, silent)
- [ ] Beads popup shows ready tasks with navigation
- [ ] Ctrl+B b opens beads panel
- [ ] Refresh interval is configurable
- [ ] No blocking on daemon communication (async)
- [ ] Socket timeout prevents hanging
- [ ] All existing tests pass
- [ ] New feature has test coverage

## Dependencies

- FEAT-057 (Beads Passive Awareness) - for `.beads/` detection infrastructure

## Notes

- Beads daemon uses Unix socket RPC - need to understand the protocol
- Status bar real estate is limited - keep indicator concise
- Consider caching ready count to avoid frequent daemon queries
- Panel should load quickly - async fetch while showing "Loading..."
- MCP tools are lower priority than TUI integration
- Future enhancement: Task claiming directly from panel
- Future enhancement: Auto-refresh panel while open
