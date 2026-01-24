# Implementation Plan: FEAT-059

**Work Item**: [FEAT-059: Beads Workflow Integration - Pane-Issue Correlation and Audit Trail](PROMPT.md)
**Component**: fugue-server, fugue-protocol
**Priority**: P3
**Created**: 2026-01-11

## Overview

Add deep workflow integration between fugue panes and beads issues, enabling automatic correlation, audit trails, and recovery hints for multi-agent development workflows.

## Architecture Decisions

- **Approach**: Extend pane metadata with workflow state, add MCP tools for assignment, integrate with orchestration protocol
- **Trade-offs**:
  - Per-pane vs global issue tracking: Choosing per-pane for clear ownership
  - Explicit assignment vs auto-detection: Supporting both, explicit as primary
  - Synchronous vs async issue creation on crash: Async with notification, user confirms
  - Command interception layer: Minimal, only for `bd close` injection

## Affected Components

| Component | Type of Change | Risk Level |
|-----------|----------------|------------|
| fugue-server/src/session/pane.rs | Add PaneWorkflowState | Low |
| fugue-server/src/beads/workflow.rs | New module | Low |
| fugue-server/src/mcp/tools.rs | Add 4 new tools | Medium |
| fugue-server/src/mcp/handlers.rs | Implement handlers | Medium |
| fugue-server/src/handlers/pane.rs | Crash detection hooks | Medium |
| fugue-server/src/handlers/pty.rs | Command interception | Medium |
| fugue-protocol/src/messages.rs | Add issue to StatusUpdate | Low |
| fugue-server/src/config.rs | Add workflow config | Low |
| fugue-client/src/ui/pane.rs | Display issue in status | Low |

## Implementation Details

### 1. Core Data Structures

Add to `fugue-server/src/session/pane.rs` or new `workflow.rs`:

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Workflow state for pane-issue correlation
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PaneWorkflowState {
    /// Currently assigned issue ID (e.g., "bd-456")
    pub current_issue_id: Option<String>,

    /// When the current issue was assigned
    pub assigned_at: Option<DateTime<Utc>>,

    /// History of issues worked on by this pane
    pub issue_history: Vec<IssueHistoryEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssueHistoryEntry {
    pub issue_id: String,
    pub assigned_at: DateTime<Utc>,
    pub released_at: Option<DateTime<Utc>>,
    pub outcome: Option<IssueOutcome>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum IssueOutcome {
    Completed,
    Abandoned,
    Transferred,
    Crashed,
}

impl PaneWorkflowState {
    pub fn assign_issue(&mut self, issue_id: String) {
        // Release current issue if any
        if let Some(current) = self.current_issue_id.take() {
            self.release_current(IssueOutcome::Transferred);
        }

        self.current_issue_id = Some(issue_id.clone());
        self.assigned_at = Some(Utc::now());

        // Start new history entry
        self.issue_history.push(IssueHistoryEntry {
            issue_id,
            assigned_at: Utc::now(),
            released_at: None,
            outcome: None,
        });
    }

    pub fn release_current(&mut self, outcome: IssueOutcome) {
        if let Some(entry) = self.issue_history.last_mut() {
            if entry.released_at.is_none() {
                entry.released_at = Some(Utc::now());
                entry.outcome = Some(outcome);
            }
        }
        self.current_issue_id = None;
        self.assigned_at = None;
    }
}
```

### 2. MCP Tools

Add to `fugue-server/src/mcp/tools.rs`:

```rust
pub const BEADS_ASSIGN_TOOL: Tool = Tool {
    name: "fugue_beads_assign",
    description: "Assign a beads issue to a pane for tracking",
    input_schema: json!({
        "type": "object",
        "properties": {
            "issue_id": {
                "type": "string",
                "description": "The beads issue ID (e.g., 'bd-456')"
            },
            "pane_id": {
                "type": "string",
                "description": "Optional pane ID. Defaults to focused pane."
            }
        },
        "required": ["issue_id"]
    }),
};

pub const BEADS_RELEASE_TOOL: Tool = Tool {
    name: "fugue_beads_release",
    description: "Release/unassign the current issue from a pane",
    input_schema: json!({
        "type": "object",
        "properties": {
            "pane_id": {
                "type": "string",
                "description": "Optional pane ID. Defaults to focused pane."
            },
            "outcome": {
                "type": "string",
                "enum": ["completed", "abandoned", "transferred"],
                "description": "Why the issue is being released"
            }
        }
    }),
};

pub const BEADS_FIND_PANE_TOOL: Tool = Tool {
    name: "fugue_beads_find_pane",
    description: "Find which pane is working on an issue",
    input_schema: json!({
        "type": "object",
        "properties": {
            "issue_id": {
                "type": "string",
                "description": "The beads issue ID to search for"
            }
        },
        "required": ["issue_id"]
    }),
};

pub const BEADS_PANE_HISTORY_TOOL: Tool = Tool {
    name: "fugue_beads_pane_history",
    description: "Get issue history for a pane",
    input_schema: json!({
        "type": "object",
        "properties": {
            "pane_id": {
                "type": "string",
                "description": "Optional pane ID. Defaults to focused pane."
            }
        }
    }),
};
```

### 3. MCP Handlers

Add to `fugue-server/src/mcp/handlers.rs`:

```rust
async fn handle_beads_assign(&self, params: BeadsAssignParams) -> Result<Value, McpError> {
    let pane_id = params.pane_id
        .map(|s| PaneId::parse(&s))
        .transpose()?
        .unwrap_or_else(|| self.get_focused_pane_id());

    let mut panes = self.panes.write().await;
    let pane = panes.get_mut(&pane_id)
        .ok_or(McpError::PaneNotFound(pane_id))?;

    pane.workflow.assign_issue(params.issue_id.clone());

    // Broadcast assignment change
    self.broadcast_pane_update(&pane_id).await;

    Ok(json!({
        "status": "assigned",
        "pane_id": pane_id.to_string(),
        "issue_id": params.issue_id
    }))
}

async fn handle_beads_find_pane(&self, params: BeadsFindPaneParams) -> Result<Value, McpError> {
    let panes = self.panes.read().await;

    for (pane_id, pane) in panes.iter() {
        if pane.workflow.current_issue_id.as_ref() == Some(&params.issue_id) {
            return Ok(json!({
                "found": true,
                "pane_id": pane_id.to_string(),
                "pane_name": pane.name,
                "session_id": pane.session_id.to_string(),
                "assigned_at": pane.workflow.assigned_at
            }));
        }
    }

    Ok(json!({
        "found": false,
        "issue_id": params.issue_id
    }))
}
```

### 4. Command Interception

Add to `fugue-server/src/handlers/pty.rs`:

```rust
impl CommandInterceptor {
    pub fn new(config: &BeadsWorkflowConfig) -> Self {
        Self {
            auto_inject_session: config.auto_inject_session,
        }
    }

    /// Intercept and potentially modify commands before execution
    pub fn intercept(&self, cmd: &str, pane: &Pane) -> String {
        if !self.auto_inject_session {
            return cmd.to_string();
        }

        // Check if this is a `bd close` command
        if self.is_bd_close_command(cmd) {
            return self.inject_session(cmd, pane);
        }

        cmd.to_string()
    }

    fn is_bd_close_command(&self, cmd: &str) -> bool {
        let trimmed = cmd.trim();
        trimmed.starts_with("bd close") || trimmed.starts_with("bd c ")
    }

    fn inject_session(&self, cmd: &str, pane: &Pane) -> String {
        // Don't inject if --session already present
        if cmd.contains("--session") {
            return cmd.to_string();
        }

        // Insert --session before any trailing arguments
        format!("{} --session {}", cmd.trim(), pane.id)
    }
}
```

### 5. Crash Handling

Add to `fugue-server/src/handlers/pane.rs`:

```rust
#[derive(Debug, Clone, Serialize)]
pub struct CrashIssue {
    pub title: String,
    pub pane_id: PaneId,
    pub pane_name: Option<String>,
    pub session_id: SessionId,
    pub exit_code: i32,
    pub context: Vec<String>,
    pub working_issue: Option<String>,
    pub timestamp: DateTime<Utc>,
}

impl PaneCrashHandler {
    pub async fn handle_exit(&self, pane: &Pane, exit_code: i32) -> Result<()> {
        // Mark issue as crashed if one was assigned
        if let Some(ref issue_id) = pane.workflow.current_issue_id {
            tracing::warn!(
                pane_id = %pane.id,
                issue_id = %issue_id,
                exit_code = exit_code,
                "Pane crashed while working on issue"
            );
        }

        // Only create crash issue for non-zero exits if configured
        if exit_code == 0 || !self.config.beads.workflow.issue_on_crash {
            return Ok(());
        }

        let context_lines = pane.scrollback
            .last_n_lines(self.config.beads.workflow.crash_context_lines);

        let crash_issue = CrashIssue {
            title: format!(
                "Pane crash: {} (exit code {})",
                pane.name.as_deref().unwrap_or("unnamed"),
                exit_code
            ),
            pane_id: pane.id,
            pane_name: pane.name.clone(),
            session_id: pane.session_id,
            exit_code,
            context: context_lines,
            working_issue: pane.workflow.current_issue_id.clone(),
            timestamp: Utc::now(),
        };

        // Broadcast crash event for orchestrator/user to handle
        self.event_tx.send(ServerEvent::PaneCrash(crash_issue)).await?;

        Ok(())
    }
}
```

### 6. Orchestration Protocol Update

Modify `fugue-protocol/src/messages.rs`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusUpdate {
    pub pane_id: String,
    pub status: PaneStatus,
    pub timestamp: DateTime<Utc>,

    // ... existing fields ...

    /// Currently assigned beads issue (if any)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_issue_id: Option<String>,

    /// Issue status from beads (if available)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub issue_status: Option<String>,
}
```

### 7. Configuration

Add to `fugue-server/src/config.rs`:

```rust
#[derive(Debug, Clone, Deserialize)]
pub struct BeadsWorkflowConfig {
    /// Enable workflow tracking (default: true)
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Auto-inject pane session into bd close commands
    #[serde(default = "default_true")]
    pub auto_inject_session: bool,

    /// Offer to create issue on pane crash
    #[serde(default = "default_false")]
    pub issue_on_crash: bool,

    /// Lines of scrollback to capture for crash context
    #[serde(default = "default_crash_context_lines")]
    pub crash_context_lines: usize,

    /// Show issue in pane status bar
    #[serde(default = "default_true")]
    pub show_issue_in_status: bool,

    /// Auto-detect issue from bd commands
    #[serde(default = "default_false")]
    pub auto_detect_issue: bool,
}

fn default_true() -> bool { true }
fn default_false() -> bool { false }
fn default_crash_context_lines() -> usize { 50 }

impl Default for BeadsWorkflowConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            auto_inject_session: true,
            issue_on_crash: false,
            crash_context_lines: 50,
            show_issue_in_status: true,
            auto_detect_issue: false,
        }
    }
}
```

## Dependencies

- **FEAT-057** (Beads Passive Awareness): Detection of `.beads/` directory to know when workflow features apply
- **FEAT-058** (Beads Query Integration): Communication with beads daemon to fetch issue status
- **FEAT-050** (Session Metadata Storage): Foundation for storing workflow state in pane metadata

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Command interception latency | Low | Medium | Make interception fast, async where possible |
| History growth unbounded | Low | Low | Add max history config, periodic cleanup |
| Issue ID format changes | Low | Low | Keep validation lenient, just store strings |
| Crash handler misses events | Medium | Medium | Comprehensive exit code handling, logging |
| Persistence serialization changes | Low | Medium | Version workflow state, migration support |

## Rollback Strategy

If implementation causes issues:
1. Revert commits associated with this work item
2. Workflow tracking disabled (panes work without issue correlation)
3. Orchestration messages return to previous format (no issue_id)
4. Document issues and plan fixes

## Testing Strategy

1. **Unit tests**: PaneWorkflowState assignment/release logic, history tracking
2. **MCP tests**: All 4 new tools with various inputs
3. **Integration tests**: Full flow from assignment through release
4. **Crash tests**: Simulate pane crashes, verify event broadcast
5. **Persistence tests**: Workflow state survives restart
6. **Manual testing**:
   - Assign issue, verify status display
   - Run `bd close`, verify session injection
   - Kill pane, verify crash event

## Implementation Notes

<!-- Add notes during implementation -->

---
*This plan should be updated as implementation progresses.*
