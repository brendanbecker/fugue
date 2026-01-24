# FEAT-059: Beads Workflow Integration - Pane-Issue Correlation and Audit Trail

**Priority**: P3
**Component**: fugue-server, fugue-protocol
**Type**: new_feature
**Estimated Effort**: large
**Business Value**: high
**Technical Complexity**: high
**Status**: new

## Overview

Add deep workflow integration between fugue panes and beads issues, enabling automatic correlation, audit trails, and recovery hints for multi-agent development workflows.

## Problem Statement

In multi-agent systems like Gas Town, multiple panes work on different issues simultaneously. When something goes wrong (pane crash, stuck agent, merge conflict), there's no easy way to:

- Know which pane was working on which issue
- Trace what happened to a specific issue
- Recover/restart work on an issue
- Automatically attribute work to the correct pane/session

This creates operational blind spots in multi-agent workflows:

1. **Attribution**: When an issue is closed, which pane did the work? Which session?
2. **Debugging**: A pane crashes - what issue context was lost?
3. **Recovery**: Want to restart work on issue bd-456 - which pane had it?
4. **Coordination**: The orchestrator needs to know "who's working on what"

## Architecture

```
+---------------+                              +----------------+
|   TUI Client  |                              |   Orchestrator |
|               |                              |    (Claude)    |
|  [FEAT-050    |                              +-------+--------+
|   metadata]   |                                      |
+-------+-------+                                      |
        |                                              |
        | current_issue_id                             | StatusUpdate
        |                                              | (includes issue_id)
        v                                              v
+-------+-----------------------------------------------+-------+
|                        fugue-server                           |
|                                                               |
|  +------------------+     +------------------+                 |
|  | PaneMetadata     |     | BeadsWorkflow    |                 |
|  |                  |     |                  |                 |
|  | current_issue_id |<--->| track_assignment |                 |
|  | issue_history[]  |     | inject_context() |                 |
|  +------------------+     | on_pane_crash()  |                 |
|                           +--------+---------+                 |
|                                    |                           |
+------------------------------------+---------------------------+
                                     |
                                     | bd close --session <pane_id>
                                     |
                                     v
                            +--------+---------+
                            |   beads daemon   |
                            |                  |
                            | closed_by_session|
                            +------------------+
```

## Proposed Solution

### 1. Pane-Issue Assignment Tracking

Store `current_issue_id` in pane metadata (leveraging FEAT-050):

```rust
// In pane metadata
pub struct PaneWorkflowState {
    /// Currently assigned issue (e.g., "bd-456")
    pub current_issue_id: Option<String>,

    /// History of issues worked on by this pane
    pub issue_history: Vec<IssueHistoryEntry>,

    /// When current issue was assigned
    pub assigned_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssueHistoryEntry {
    pub issue_id: String,
    pub assigned_at: DateTime<Utc>,
    pub released_at: Option<DateTime<Utc>>,
    pub outcome: Option<IssueOutcome>, // completed, abandoned, crashed
}
```

### 2. MCP Tools for Issue Assignment

New MCP tool `fugue_beads_assign`:

```json
{
  "name": "fugue_beads_assign",
  "description": "Assign a beads issue to the current pane for tracking",
  "inputSchema": {
    "type": "object",
    "properties": {
      "issue_id": {
        "type": "string",
        "description": "The beads issue ID (e.g., 'bd-456')"
      },
      "pane_id": {
        "type": "string",
        "description": "Optional pane ID. Defaults to current focused pane."
      }
    },
    "required": ["issue_id"]
  }
}
```

New MCP tool `fugue_beads_release`:

```json
{
  "name": "fugue_beads_release",
  "description": "Release/unassign the current issue from a pane",
  "inputSchema": {
    "type": "object",
    "properties": {
      "pane_id": {
        "type": "string",
        "description": "Optional pane ID. Defaults to current focused pane."
      },
      "outcome": {
        "type": "string",
        "enum": ["completed", "abandoned", "transferred"],
        "description": "Why the issue is being released"
      }
    }
  }
}
```

### 3. Auto-Inject Pane Context into Beads Operations

When a pane runs `bd close`, inject session information:

```rust
// In command interception layer
impl CommandInterceptor {
    fn intercept_bd_close(&self, cmd: &str, pane: &Pane) -> String {
        if !self.config.beads.workflow.auto_inject_session {
            return cmd.to_string();
        }

        // Parse existing command
        let mut args: Vec<&str> = cmd.split_whitespace().collect();

        // Inject --session if not already present
        if !args.contains(&"--session") {
            args.push("--session");
            args.push(&pane.id.to_string());
        }

        args.join(" ")
    }
}
```

The beads daemon would store this as `closed_by_session` in the issue metadata.

### 4. Issue Creation from Pane Events

On pane crash (non-zero exit), offer to create a bug:

```rust
impl PaneCrashHandler {
    async fn handle_crash(&self, pane: &Pane, exit_code: i32) -> Result<()> {
        if !self.config.beads.workflow.issue_on_crash {
            return Ok(());
        }

        // Capture scrollback context
        let context_lines = pane.scrollback.last_n_lines(
            self.config.beads.workflow.crash_context_lines
        );

        // Build issue creation prompt
        let crash_issue = CrashIssue {
            title: format!(
                "Pane crash: {} (exit code {})",
                pane.name.as_deref().unwrap_or("unnamed"),
                exit_code
            ),
            pane_id: pane.id,
            session_id: pane.session_id,
            exit_code,
            context: context_lines,
            working_issue: pane.workflow.current_issue_id.clone(),
            timestamp: Utc::now(),
        };

        // Notify orchestrator/user for issue creation decision
        self.broadcast_crash_event(crash_issue).await?;

        Ok(())
    }
}
```

### 5. Recovery and Query Tools

MCP tool `fugue_beads_find_pane`:

```json
{
  "name": "fugue_beads_find_pane",
  "description": "Find which pane is currently working on an issue",
  "inputSchema": {
    "type": "object",
    "properties": {
      "issue_id": {
        "type": "string",
        "description": "The beads issue ID to search for"
      }
    },
    "required": ["issue_id"]
  }
}
```

Response:
```json
{
  "found": true,
  "pane_id": "abc-123",
  "pane_name": "worker-1",
  "session_id": "def-456",
  "assigned_at": "2026-01-11T10:30:00Z"
}
```

MCP tool `fugue_beads_pane_history`:

```json
{
  "name": "fugue_beads_pane_history",
  "description": "Get the issue history for a pane",
  "inputSchema": {
    "type": "object",
    "properties": {
      "pane_id": {
        "type": "string",
        "description": "The pane ID. Defaults to current focused pane."
      }
    }
  }
}
```

Response:
```json
{
  "pane_id": "abc-123",
  "current_issue": "bd-789",
  "history": [
    {
      "issue_id": "bd-456",
      "assigned_at": "2026-01-11T09:00:00Z",
      "released_at": "2026-01-11T09:45:00Z",
      "outcome": "completed"
    },
    {
      "issue_id": "bd-789",
      "assigned_at": "2026-01-11T10:00:00Z",
      "released_at": null,
      "outcome": null
    }
  ]
}
```

### 6. Orchestration Integration

Include issue assignment in `StatusUpdate` messages:

```rust
// In orchestration protocol
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusUpdate {
    // ... existing fields ...

    /// Currently assigned beads issue (if any)
    pub current_issue_id: Option<String>,

    /// Issue status from beads (if available)
    pub issue_status: Option<String>,
}
```

This allows the orchestrator to:
- Query "who's working on what"
- Avoid assigning the same issue to multiple panes
- Track issue progress across the fleet

### 7. Pane Status Display

Show assigned issue in pane status (TUI and MCP responses):

```
+-----------------------------------------+
| Session: gas-town  Window: workers      |
+-----------------------------------------+
| [worker-1]         | [worker-2]         |
| Issue: bd-456      | Issue: bd-789      |
| Status: in_progress| Status: in_progress|
|                    |                    |
```

### 8. Configuration

```toml
[beads.workflow]
# Enable workflow tracking (default: true)
enabled = true

# Auto-inject pane context into bd close
auto_inject_session = true

# Offer to create issue on pane crash
issue_on_crash = true

# Lines of scrollback to capture for crash issues
crash_context_lines = 50

# Show issue assignment in pane status bar
show_issue_in_status = true

# Auto-detect issue from `bd` commands (parse bd work, bd close, etc.)
auto_detect_issue = true
```

## Files to Modify

| File | Changes |
|------|---------|
| `fugue-server/src/session/pane.rs` | Add `PaneWorkflowState` to pane metadata |
| `fugue-server/src/beads/workflow.rs` | New module for workflow tracking logic |
| `fugue-server/src/beads/mod.rs` | Add workflow submodule |
| `fugue-server/src/mcp/tools.rs` | Add workflow MCP tools (4 new tools) |
| `fugue-server/src/mcp/handlers.rs` | Implement workflow tool handlers |
| `fugue-server/src/handlers/pane.rs` | Add crash detection and issue creation hooks |
| `fugue-server/src/handlers/pty.rs` | Add command interception for bd commands |
| `fugue-protocol/src/messages.rs` | Include issue_id in StatusUpdate |
| `fugue-protocol/src/orchestration.rs` | Add issue tracking to orchestration messages |
| `fugue-server/src/config.rs` | Add `[beads.workflow]` configuration |
| `fugue-client/src/ui/pane.rs` | Display issue assignment in status |

## Implementation Tasks

### Section 1: Core Data Structures
- [ ] Define `PaneWorkflowState` struct
- [ ] Define `IssueHistoryEntry` struct
- [ ] Define `IssueOutcome` enum
- [ ] Add workflow state to pane metadata
- [ ] Implement serialization for persistence

### Section 2: MCP Assignment Tools
- [ ] Implement `fugue_beads_assign` handler
- [ ] Implement `fugue_beads_release` handler
- [ ] Add validation for issue ID format
- [ ] Update pane history on assignment/release
- [ ] Broadcast assignment changes to clients

### Section 3: Auto-Injection
- [ ] Create command interception layer
- [ ] Detect `bd close` commands
- [ ] Inject `--session` parameter
- [ ] Make injection configurable
- [ ] Log injected commands for debugging

### Section 4: Crash Handling
- [ ] Detect non-zero pane exits
- [ ] Capture scrollback context
- [ ] Build crash issue structure
- [ ] Broadcast crash event
- [ ] Implement issue creation flow (optional)

### Section 5: Query Tools
- [ ] Implement `fugue_beads_find_pane` handler
- [ ] Implement `fugue_beads_pane_history` handler
- [ ] Add index for fast issue -> pane lookup
- [ ] Include history in pane serialization

### Section 6: Orchestration Integration
- [ ] Add `current_issue_id` to StatusUpdate
- [ ] Add `issue_status` to StatusUpdate (optional)
- [ ] Update orchestration message handlers
- [ ] Document protocol changes

### Section 7: UI Integration
- [ ] Add issue display to pane status bar
- [ ] Make display configurable
- [ ] Handle long issue IDs gracefully

### Section 8: Configuration
- [ ] Add `BeadsWorkflowConfig` struct
- [ ] Add TOML deserialization
- [ ] Add default values
- [ ] Document configuration options

### Section 9: Testing
- [ ] Unit tests for PaneWorkflowState
- [ ] Unit tests for issue history tracking
- [ ] Integration tests for MCP tools
- [ ] Integration tests for crash handling
- [ ] Integration tests for auto-injection
- [ ] Manual testing with beads daemon

## Acceptance Criteria

- [ ] Panes can be assigned to issues via `fugue_beads_assign` MCP tool
- [ ] Pane status shows assigned issue (when configured)
- [ ] `bd close` auto-injects pane session ID when `auto_inject_session` is enabled
- [ ] Pane crash offers to create issue with scrollback context
- [ ] `fugue_beads_find_pane` returns the pane working on a given issue
- [ ] `fugue_beads_pane_history` returns issue history for a pane
- [ ] Orchestration `StatusUpdate` messages include issue assignment
- [ ] All tracking persists across server restarts (via pane persistence)
- [ ] All features are configurable via `[beads.workflow]` section
- [ ] All existing tests pass
- [ ] New features have test coverage

## Dependencies

- **FEAT-057** (Beads Passive Awareness): Required - For `.beads/` directory detection
- **FEAT-058** (Beads Query Integration): Required - For daemon communication to get issue status
- **FEAT-050** (Session Metadata Storage): COMPLETED - Foundation for storing issue assignments

## Leveraging Completed Features

### FEAT-050 Session Metadata (Completed)

FEAT-050 provides the exact infrastructure needed for pane-issue correlation. The implementation can use existing MCP tools:

```rust
// Assign issue to pane via metadata
fugue_set_metadata(session, "beads.current_issue", "bd-456")
fugue_set_metadata(session, "beads.assigned_at", "2026-01-11T10:30:00Z")

// Query assignment
let issue = fugue_get_metadata(session, "beads.current_issue")
```

**Implementation Simplification**: Much of the proposed `PaneWorkflowState` struct can be implemented using the existing metadata system rather than new dedicated fields:

| Proposed Field | Metadata Key |
|----------------|--------------|
| `current_issue_id` | `beads.current_issue` |
| `assigned_at` | `beads.assigned_at` |
| `issue_history` | `beads.issue_history` (JSON array) |

This means:
- **Reduced implementation scope** - Use existing metadata infrastructure
- **MCP tools already exist** - `fugue_set_metadata`, `fugue_get_metadata`
- **Persistence handled** - Session metadata persists with session state

### FEAT-028 Tag-based Routing (Completed)

The orchestration integration (Section 6) can use tag-based routing:

```rust
// StatusUpdate with issue assignment
let msg = OrchestrationMessage::new("status.update", json!({
    "session_id": session_id,
    "status": "working",
    "current_issue_id": "bd-456"
}));
send_orchestration(OrchestrationTarget::Tagged("orchestrator".to_string()), msg);
```

### Scope Reduction

Given FEAT-050's completion, consider these scope reductions:

1. **Section 1 (Core Data Structures)**: Simplify - use metadata instead of new structs
2. **Section 2 (MCP Assignment Tools)**: May just be thin wrappers around `fugue_set_metadata`
3. **Section 5 (Query Tools)**: Can use `fugue_get_metadata` with iteration
4. **Section 6 (Orchestration Integration)**: Already supported via FEAT-028

## Benefits

| Benefit | Description |
|---------|-------------|
| **Complete audit trail** | Know exactly which pane worked on which issue, when |
| **Easy debugging** | "What was pane X working on when it crashed?" |
| **Recovery support** | "Restart work on issue Y" with context |
| **Attribution** | Accurate tracking of which pane/session completed each task |
| **Coordination** | Orchestrator can avoid duplicate assignments |
| **Visibility** | UI shows current work assignment per pane |

## Future Enhancements

- **Auto-detect from `bd` commands**: Parse `bd work bd-456` to auto-assign issue
- **Conflict detection**: Warn if multiple panes assigned to same issue
- **Time tracking**: Track time spent per issue per pane
- **Integration with beads search**: Query beads for issue details to display
- **Recovery wizard**: Interactive flow to restart work on crashed issue

## Notes

- Issue IDs are expected to be beads format (e.g., "bd-456") but validation is lenient
- History is kept per-pane and persisted with pane state
- Auto-injection requires command interception which may have latency implications
- Crash issue creation is opt-in and requires user/orchestrator confirmation
