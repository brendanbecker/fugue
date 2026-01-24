# FEAT-044: Claude Session Persistence and Auto-Resume

**Priority**: P1
**Component**: fugue-server
**Type**: new_feature
**Estimated Effort**: large
**Business Value**: high
**Technical Complexity**: high
**Status**: new

## Overview

When fugue server restarts, Claude instances running in panes lose their conversation context. This feature enables fugue to track Claude session IDs and automatically resume them on server restart, preserving AI conversation continuity.

## Problem Statement

Currently when fugue server is killed (for updates, crashes, etc.):
- Session/window/pane structure is preserved (WAL persistence via FEAT-016)
- PTY processes die (shells, Claude instances)
- On reattach, new shells spawn but Claude has no memory of previous conversation
- Users must manually run `claude --resume` and pick the right session

### Current Behavior

```
[fugue server running]
  -> Pane 1: Claude in conversation about feature X
  -> Pane 2: Claude working on bug Y

[Server killed for update]

[Server restarted, client reattaches]
  -> Pane 1: Fresh shell (Claude conversation lost)
  -> Pane 2: Fresh shell (Claude conversation lost)
  -> User must manually: claude --resume, pick correct session
```

### Desired Behavior

```
[fugue server running]
  -> Pane 1: Claude in conversation about feature X (session-id: abc123)
  -> Pane 2: Claude working on bug Y (session-id: def456)

[Server killed for update]

[Server restarted, client reattaches]
  -> Pane 1: claude --resume abc123 (auto-executed, conversation continues)
  -> Pane 2: claude --resume def456 (auto-executed, conversation continues)
  -> No user intervention needed
```

## Requirements

### Part 1: Claude Session ID Tracking

1. **Detect or Assign Session ID**
   - Option A: Generate UUID before launching Claude, use `--session-id <uuid>`
   - Option B: Detect session ID from Claude's output when it starts
   - Option C: Hybrid approach based on pane command detection

2. **Store in Pane Metadata**
   - Add `claude_session_id: Option<String>` field to Pane struct
   - Track whether pane is running a Claude instance

3. **Persist to Disk**
   - Include `claude_session_id` in WAL/checkpoint persistence
   - Load session ID on restore

### Part 2: Session Restoration

1. **Restore with Claude Resume**
   - On pane restoration, check if `claude_session_id` is set
   - If set, launch `claude --resume <session-id>` instead of default command
   - If not set, launch normal default command

2. **Graceful Fallback**
   - Handle case where Claude session no longer exists
   - Claude `--resume` with invalid session may error - detect and recover
   - Option to fall back to fresh Claude or fresh shell

3. **Preserve Working Directory**
   - Claude sessions are directory-scoped
   - Ensure restore happens in correct working directory

### Part 3: Persistent Session Flag (Optional Enhancement)

1. **Mark Sessions as Persistent**
   - New MCP tool: `fugue_mark_persistent` or similar
   - Persistent sessions get priority treatment
   - Could be used for "Orchestrator" pattern

2. **Configuration Options**
   - Config to auto-mark Claude panes as persistent
   - Config for resume behavior (auto, ask, manual)

## Claude CLI Options

Relevant Claude CLI flags for this feature:

```
--resume [value]     Resume by session ID, or open interactive picker
--session-id <uuid>  Use a specific session ID for the conversation
--continue           Continue most recent in current directory
--fork-session       When resuming, create new session ID instead of reusing
```

## Implementation Approach Options

### Option A: Proactive Session ID Assignment

**Flow**:
1. fugue detects pane command contains "claude"
2. Generates UUID before launching
3. Launches `claude --session-id <uuid>` (injects the flag)
4. Stores UUID in pane metadata immediately

**Pros**:
- Guaranteed to have session ID
- No output parsing needed
- Works immediately, no race conditions

**Cons**:
- May conflict with user's existing Claude usage patterns
- Modifies user's command
- User may have already specified `--session-id`

**Mitigation**:
- Only inject if no `--session-id` or `--resume` already present
- Make behavior configurable

### Option B: Reactive Session ID Detection

**Flow**:
1. Pane launches with user's original command
2. Monitor Claude output for session ID pattern
3. Parse and store when detected

**Pros**:
- Non-invasive, works with any Claude launch
- Respects user's existing flags
- No command modification

**Cons**:
- May miss session ID if output pattern changes
- Slight delay before session ID is captured
- Claude may not output session ID in all cases

### Option C: Hybrid (Recommended)

**Flow**:
1. Detect if pane command starts with `claude`
2. Check if `--session-id` or `--resume` already present
3. If not, inject `--session-id <generated-uuid>`
4. Also implement reactive detection as fallback
5. Store whichever session ID is detected/assigned

**Pros**:
- Best of both worlds
- Respects user preferences
- Has fallback for edge cases

**Cons**:
- More complex implementation

## Files Affected

| File | Changes |
|------|---------|
| `fugue-server/src/session/pane.rs` | Add `claude_session_id: Option<String>` field |
| `fugue-server/src/persistence/` | Serialize/deserialize claude_session_id |
| `fugue-server/src/pty/` | Command modification for session-id injection |
| `fugue-server/src/pty/` | Restore with `--resume` on restart |
| `fugue-server/src/mcp/` | Tools for marking sessions as persistent (optional) |
| `fugue-protocol/src/messages.rs` | Add session metadata to pane info messages |

## Use Cases

### 1. Developer Workflow

Kill server to update binary, resume exactly where you were:

```
# Before update
Claude: "I've analyzed the code and here's my plan for fixing the bug..."

# Server restarts

# After update (auto-resume)
Claude: "As I was saying about the bug fix..."
```

### 2. Orchestrator Pattern

Named "Orchestrator" session always resumes its Claude context:

```
# Orchestrator pane (marked persistent)
Claude: "I'm coordinating 3 worker agents on FEAT-045..."

# Server restart

# Orchestrator auto-resumes
Claude: "Continuing coordination of FEAT-045 workers..."
```

### 3. Multi-Agent Development

Each worker Claude resumes its specific task context:

```
# Worker-1 pane
Claude: "Working on backend API implementation..."

# Worker-2 pane
Claude: "Working on frontend component..."

# Server restart - both workers resume their specific tasks
```

### 4. Crash Recovery

Unexpected server crash doesn't lose AI conversation history:

```
# Before crash
Claude: "I've made 47 file changes and am almost done..."

# Crash occurs, server auto-restarts

# Claude resumes with full context
Claude: "Continuing from where I was..."
```

## Implementation Tasks

### Section 1: Pane Metadata Extension
- [ ] Add `claude_session_id: Option<String>` to Pane struct
- [ ] Add `is_claude_pane: bool` detection flag
- [ ] Add `claude_working_dir: Option<PathBuf>` for directory tracking
- [ ] Update pane creation to initialize these fields

### Section 2: Command Detection and Injection
- [ ] Implement `is_claude_command(cmd: &str) -> bool`
- [ ] Implement `has_session_flag(cmd: &str) -> bool`
- [ ] Implement `inject_session_id(cmd: &str, uuid: &str) -> String`
- [ ] Generate UUID and inject when spawning Claude pane
- [ ] Store injected session ID in pane metadata

### Section 3: Persistence Integration
- [ ] Add claude_session_id to pane serialization
- [ ] Add claude_session_id to pane deserialization
- [ ] Ensure WAL captures session ID changes
- [ ] Test persistence across restarts

### Section 4: Restoration Logic
- [ ] On pane restore, check for claude_session_id
- [ ] If present, construct `claude --resume <id>` command
- [ ] If in specific directory, ensure `cd <dir> && claude --resume <id>`
- [ ] Implement graceful fallback if resume fails
- [ ] Test restoration with valid and invalid session IDs

### Section 5: Fallback Handling
- [ ] Detect when `claude --resume` fails (exit code, error output)
- [ ] Implement fallback behavior (fresh shell, fresh claude, retry)
- [ ] Log failures for debugging
- [ ] Consider config option for fallback behavior

### Section 6: MCP Tools (Optional)
- [ ] Add `fugue_mark_persistent` tool to mark panes for priority restore
- [ ] Add `fugue_get_claude_session` tool to query session ID
- [ ] Add `fugue_set_claude_session` tool to manually set session ID
- [ ] Update `fugue_list_panes` to include claude_session_id

### Section 7: Configuration
- [ ] Add config option: `claude.auto_assign_session_id: bool`
- [ ] Add config option: `claude.auto_resume_on_restore: bool`
- [ ] Add config option: `claude.resume_fallback: "shell" | "fresh_claude" | "error"`
- [ ] Document configuration options

### Section 8: Testing
- [ ] Unit tests for command detection
- [ ] Unit tests for session ID injection
- [ ] Integration test: spawn Claude, capture session ID
- [ ] Integration test: restart server, verify resume command
- [ ] Integration test: invalid session ID fallback
- [ ] Test with various Claude CLI flag combinations

## Acceptance Criteria

- [ ] Claude session IDs are tracked in pane metadata
- [ ] Session IDs persist across server restarts
- [ ] On restore, Claude panes launch with `--resume <session-id>`
- [ ] Working directory is preserved on restore
- [ ] Graceful fallback when session no longer exists
- [ ] Configuration options for behavior customization
- [ ] All existing tests pass
- [ ] Documentation updated

## Example Flows

### Spawning a Claude Pane

```rust
// User command: "claude"
// Detected as Claude command, no --session-id present
let uuid = Uuid::new_v4().to_string();
let command = format!("claude --session-id {}", uuid);
pane.claude_session_id = Some(uuid);
pty.spawn(command);
```

### Restoring a Claude Pane

```rust
// On pane restore
if let Some(session_id) = pane.claude_session_id {
    let command = format!("claude --resume {}", session_id);
    pty.spawn(command);
} else {
    pty.spawn(pane.default_command);
}
```

### Handling Resume Failure

```rust
// If claude --resume exits with error
match config.claude.resume_fallback {
    "shell" => pty.spawn(default_shell),
    "fresh_claude" => pty.spawn("claude"),
    "error" => show_error_to_user(),
}
```

## Related Work Items

- **FEAT-016**: Persistence - Checkpoint and WAL for Crash Recovery (dependency)
- **FEAT-015**: Claude Detection - State Detection from PTY Output (related)
- **FEAT-043**: MCP Session Rename Tool (for naming persistent sessions)
- **FEAT-020**: Session Isolation - Per-Pane CLAUDE_CONFIG_DIR (related)

## Dependencies

- **FEAT-016**: Persistence system must be in place for session ID storage

## Notes

- Claude's `--session-id` flag allows specifying arbitrary session IDs
- Claude's `--resume` flag with a session ID will restore that specific conversation
- Sessions are scoped by working directory in Claude
- Consider edge case: same session ID used in different directories
- Future enhancement: detect when user manually runs `claude --resume` and capture that session ID
- Future enhancement: MCP command to "hand off" a Claude session to a new pane
