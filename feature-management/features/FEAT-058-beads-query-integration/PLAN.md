# Implementation Plan: FEAT-058

**Work Item**: [FEAT-058: Beads Query Integration - TUI Visibility into Work Queue](PROMPT.md)
**Component**: ccmux-server, ccmux-client
**Priority**: P3
**Created**: 2026-01-11

## Overview

Implement beads daemon query integration to provide TUI visibility into the work queue. Users will see ready task counts in the status bar and can access a full task list via Ctrl+B b popup panel.

## Architecture Decisions

- **Approach**: Server-side beads client with async daemon socket connection, protocol messages for status updates, client-side rendering
- **Trade-offs**:
  - Per-pane vs global beads client: Per-pane allows multiple repos, but adds complexity. Choose per-repo with caching.
  - Poll vs push: Daemon likely doesn't push updates, so poll on configurable interval (default 30s)
  - Status bar vs dedicated widget: Status bar for count, popup panel for full list
  - Blocking vs async: Must be async - daemon unavailability should never block TUI

## Affected Components

| Component | Type of Change | Risk Level |
|-----------|----------------|------------|
| ccmux-server/src/beads/ | New module | Low |
| ccmux-server/src/config.rs | Add configuration section | Low |
| ccmux-protocol/src/lib.rs | Add status message types | Low |
| ccmux-client/src/ui/status.rs | Add beads indicator | Low |
| ccmux-client/src/ui/beads_panel.rs | New panel component | Medium |
| ccmux-client/src/input/mod.rs | Add Ctrl+B b keybind | Low |
| ccmux-server/src/mcp/handlers.rs | Add MCP tools (optional) | Low |

## Implementation Details

### 1. Beads Client Module

New module structure:

```
ccmux-server/src/beads/
  mod.rs         - Module exports
  client.rs      - RPC client for daemon
  discovery.rs   - Socket path discovery
  types.rs       - BeadsTask, BeadsStatus types
```

**Client implementation**:

```rust
// ccmux-server/src/beads/client.rs

use std::path::{Path, PathBuf};
use tokio::net::UnixStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::time::{timeout, Duration};

pub struct BeadsClient {
    socket_path: PathBuf,
    timeout: Duration,
}

impl BeadsClient {
    pub fn new(socket_path: PathBuf, timeout_ms: u32) -> Self {
        Self {
            socket_path,
            timeout: Duration::from_millis(timeout_ms as u64),
        }
    }

    pub async fn query_ready(&self) -> Result<Vec<BeadsTask>, BeadsError> {
        let stream = timeout(self.timeout, UnixStream::connect(&self.socket_path))
            .await
            .map_err(|_| BeadsError::ConnectionTimeout)?
            .map_err(BeadsError::ConnectionFailed)?;

        // Send "ready" query
        // Parse JSON response
        // Return task list
    }

    pub fn is_socket_available(&self) -> bool {
        self.socket_path.exists()
    }
}
```

**Socket discovery**:

```rust
// ccmux-server/src/beads/discovery.rs

pub fn discover_beads_socket(working_dir: &Path) -> Option<PathBuf> {
    let mut current = working_dir.to_path_buf();

    loop {
        let socket_path = current.join(".beads/bd.sock");
        if socket_path.exists() {
            return Some(socket_path);
        }

        if !current.pop() {
            return None;
        }
    }
}

pub fn discover_beads_root(working_dir: &Path) -> Option<PathBuf> {
    let mut current = working_dir.to_path_buf();

    loop {
        let beads_dir = current.join(".beads");
        if beads_dir.is_dir() {
            return Some(current);
        }

        if !current.pop() {
            return None;
        }
    }
}
```

### 2. Protocol Messages

Add to `ccmux-protocol/src/lib.rs`:

```rust
/// Beads work queue status for a pane/repo
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeadsStatus {
    /// Number of ready tasks
    pub ready_count: usize,
    /// Highest priority task title (if any)
    pub highest_priority_title: Option<String>,
    /// Whether daemon is available
    pub daemon_available: bool,
    /// Beads repo root path
    pub repo_root: Option<PathBuf>,
}

/// Server -> Client: Beads status update
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServerMessage {
    // ... existing variants ...

    /// Beads status for the focused pane
    BeadsStatusUpdate(BeadsStatus),
}

/// Client -> Server: Request beads ready list
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClientMessage {
    // ... existing variants ...

    /// Request full ready task list
    RequestBeadsReady,
}

/// Task from beads daemon
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeadsTask {
    pub id: String,
    pub title: String,
    pub priority: String,
    pub component: String,
    pub status: String,
    pub path: PathBuf,
}

/// Server -> Client: Full ready task list response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeadsReadyResponse {
    pub tasks: Vec<BeadsTask>,
    pub daemon_available: bool,
    pub error: Option<String>,
}
```

### 3. Server-Side Beads Manager

```rust
// ccmux-server/src/beads/manager.rs

use std::collections::HashMap;
use std::path::PathBuf;
use tokio::sync::RwLock;
use tokio::time::{interval, Duration};

pub struct BeadsManager {
    /// Cached clients per repo root
    clients: RwLock<HashMap<PathBuf, BeadsClient>>,
    /// Cached status per repo root
    status_cache: RwLock<HashMap<PathBuf, (BeadsStatus, Instant)>>,
    /// Configuration
    config: BeadsQueryConfig,
}

impl BeadsManager {
    pub async fn get_status(&self, working_dir: &Path) -> BeadsStatus {
        // Check cache first
        // If stale or missing, query daemon
        // Return cached or fresh status
    }

    pub async fn get_ready_tasks(&self, working_dir: &Path) -> Result<Vec<BeadsTask>, BeadsError> {
        // Always query daemon for full list (no caching)
    }

    pub fn start_refresh_loop(&self, tx: mpsc::Sender<BeadsStatusUpdate>) {
        // Spawn task that polls daemon on refresh_interval
        // Sends updates via channel
    }
}
```

### 4. Status Bar Integration

```rust
// ccmux-client/src/ui/status.rs

impl StatusBar {
    pub fn set_beads_status(&mut self, status: BeadsStatus) {
        self.beads_status = Some(status);
    }

    fn render_beads_indicator(&self, area: Rect, buf: &mut Buffer) -> u16 {
        let Some(status) = &self.beads_status else {
            return 0; // No space used
        };

        if !status.daemon_available {
            // Optionally show "bd: --" in dim style
            return 0;
        }

        let text = format!("bd: {} ready", status.ready_count);
        let style = if status.ready_count == 0 {
            Style::default().fg(Color::Green)
        } else {
            Style::default().fg(Color::Yellow)
        };

        let span = Span::styled(text, style);
        // Render at right side of status bar
        // Return width used
    }
}
```

### 5. Beads Panel Component

```rust
// ccmux-client/src/ui/beads_panel.rs

pub struct BeadsPanel {
    visible: bool,
    tasks: Vec<BeadsTask>,
    selected: usize,
    loading: bool,
    error: Option<String>,
    scroll_offset: usize,
}

impl BeadsPanel {
    pub fn show(&mut self) {
        self.visible = true;
        self.loading = true;
        self.error = None;
    }

    pub fn hide(&mut self) {
        self.visible = false;
    }

    pub fn set_tasks(&mut self, tasks: Vec<BeadsTask>) {
        self.tasks = tasks;
        self.loading = false;
        self.selected = 0;
        self.scroll_offset = 0;
    }

    pub fn set_error(&mut self, error: String) {
        self.error = Some(error);
        self.loading = false;
    }

    pub fn render(&self, area: Rect, buf: &mut Buffer) {
        if !self.visible {
            return;
        }

        // Centered overlay (60% width, 70% height)
        let popup_area = centered_rect(60, 70, area);

        // Clear background
        Clear.render(popup_area, buf);

        // Border and title
        let block = Block::default()
            .title(format!(" Ready Tasks ({}) ", self.tasks.len()))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan));

        let inner = block.inner(popup_area);
        block.render(popup_area, buf);

        if self.loading {
            // Render "Loading..."
            return;
        }

        if let Some(error) = &self.error {
            // Render error message
            return;
        }

        if self.tasks.is_empty() {
            // Render "No ready tasks"
            return;
        }

        // Render task table
        let header = Row::new(vec!["ID", "Priority", "Title", "Component"]);
        let rows: Vec<Row> = self.tasks.iter().enumerate().map(|(i, task)| {
            let style = if i == self.selected {
                Style::default().bg(Color::DarkGray)
            } else {
                Style::default()
            };
            Row::new(vec![
                task.id.clone(),
                task.priority.clone(),
                truncate(&task.title, 40),
                task.component.clone(),
            ]).style(style)
        }).collect();

        let table = Table::new(rows)
            .header(header)
            .widths(&[
                Constraint::Length(12),
                Constraint::Length(8),
                Constraint::Min(20),
                Constraint::Length(15),
            ]);

        table.render(inner, buf);

        // Footer with keybinds
        let footer = " [j/k] Navigate  [Enter] Claim  [v] View  [Esc] Close ";
        // Render at bottom of popup
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> Option<BeadsPanelAction> {
        if !self.visible {
            return None;
        }

        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                self.hide();
                Some(BeadsPanelAction::Close)
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.select_previous();
                None
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.select_next();
                None
            }
            KeyCode::Enter => {
                self.tasks.get(self.selected)
                    .map(|t| BeadsPanelAction::Claim(t.id.clone()))
            }
            KeyCode::Char('v') => {
                self.tasks.get(self.selected)
                    .map(|t| BeadsPanelAction::ViewDetails(t.id.clone()))
            }
            _ => None,
        }
    }

    fn select_next(&mut self) {
        if self.selected < self.tasks.len().saturating_sub(1) {
            self.selected += 1;
        }
    }

    fn select_previous(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }
}

pub enum BeadsPanelAction {
    Close,
    Claim(String),
    ViewDetails(String),
}
```

### 6. Input Handling

Add to `ccmux-client/src/input/mod.rs`:

```rust
// In prefix command handling (after Ctrl+B pressed)
KeyCode::Char('b') => {
    // Open beads panel
    Some(Action::OpenBeadsPanel)
}
```

### 7. Configuration

Add to `ccmux-server/src/config.rs`:

```rust
#[derive(Debug, Clone, Deserialize, Default)]
pub struct BeadsConfig {
    #[serde(default)]
    pub query: BeadsQueryConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BeadsQueryConfig {
    #[serde(default = "default_enabled")]
    pub enabled: bool,

    #[serde(default = "default_show_ready_count")]
    pub show_ready_count: bool,

    #[serde(default = "default_refresh_interval")]
    pub refresh_interval: u32,

    #[serde(default = "default_socket_timeout")]
    pub socket_timeout: u32,

    #[serde(default = "default_auto_discover")]
    pub auto_discover: bool,
}

impl Default for BeadsQueryConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            show_ready_count: true,
            refresh_interval: 30,
            socket_timeout: 1000,
            auto_discover: true,
        }
    }
}
```

## Dependencies

- FEAT-057 (Beads Passive Awareness) - provides `.beads/` detection infrastructure that this feature builds upon

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Daemon protocol changes | Medium | Medium | Version detection, graceful degradation |
| Daemon unavailable blocks TUI | Low | High | Strict async with timeouts, never block main loop |
| Status bar space exhaustion | Low | Low | Conditional display, priority to essential info |
| Memory from cached clients | Low | Low | Client per repo (not per pane), cleanup on repo exit |
| Performance from frequent polling | Medium | Low | Configurable interval, cache status |

## Rollback Strategy

If implementation causes issues:
1. Revert commits associated with this work item
2. Disable beads integration via config (`beads.query.enabled = false`)
3. Status bar returns to previous display
4. Document issues in comments.md

## Testing Strategy

1. **Unit tests**: Socket discovery, client timeout handling, status caching
2. **Integration tests**: Mock daemon responses, protocol serialization
3. **Manual testing**:
   - Test in beads repo with running daemon
   - Test in non-beads repo (no indicator)
   - Test with daemon stopped (graceful fallback)
   - Test panel navigation and actions
4. **Edge cases**: Multiple repos, daemon restart, config changes

## Implementation Notes

<!-- Add notes during implementation -->

---
*This plan should be updated as implementation progresses.*
