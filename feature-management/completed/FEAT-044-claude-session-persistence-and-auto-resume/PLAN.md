# Implementation Plan: FEAT-044

**Work Item**: [FEAT-044: Claude Session Persistence and Auto-Resume](PROMPT.md)
**Component**: fugue-server
**Priority**: P1
**Created**: 2026-01-10

## Overview

Enable fugue to track Claude session IDs and automatically resume them on server restart, preserving AI conversation continuity.

## Architecture Decisions

### Decision 1: Session ID Acquisition Strategy

**Choice**: Hybrid approach (proactive assignment with reactive fallback).

**Implementation**:
```rust
fn prepare_pane_command(original_cmd: &str, pane: &mut Pane) -> String {
    if !is_claude_command(original_cmd) {
        return original_cmd.to_string();
    }

    // Check if user already specified session handling
    if has_session_flag(original_cmd) {
        // Will need reactive detection for --resume case
        pane.needs_session_detection = true;
        return original_cmd.to_string();
    }

    // Inject session ID
    let uuid = Uuid::new_v4().to_string();
    pane.claude_session_id = Some(uuid.clone());
    inject_session_id(original_cmd, &uuid)
}
```

**Rationale**:
- Proactive assignment guarantees we have the session ID immediately
- Respects user's existing `--session-id` or `--resume` flags
- Reactive fallback handles edge cases where we couldn't inject

**Trade-offs**:
- Slight complexity in handling both paths
- Command modification may surprise users (document behavior)

### Decision 2: Storage Location

**Choice**: Store `claude_session_id` directly on the Pane struct.

**Fields to Add**:
```rust
pub struct Pane {
    // ... existing fields ...

    /// Claude session ID for --resume support
    pub claude_session_id: Option<String>,

    /// Original working directory for Claude sessions
    pub claude_working_dir: Option<PathBuf>,

    /// Whether this pane was originally a Claude command
    pub is_claude_pane: bool,
}
```

**Rationale**:
- Natural location for pane-specific metadata
- Already have persistence infrastructure for Pane
- Consistent with other pane properties

**Trade-offs**:
- Pane struct grows slightly
- All panes carry Claude-specific fields (negligible overhead)

### Decision 3: Restoration Command Construction

**Choice**: Use `--resume` flag with full session ID.

**Implementation**:
```rust
fn construct_restore_command(pane: &Pane) -> String {
    match (&pane.claude_session_id, &pane.claude_working_dir) {
        (Some(session_id), Some(dir)) => {
            format!("cd {} && claude --resume {}",
                shell_escape(dir),
                session_id)
        }
        (Some(session_id), None) => {
            format!("claude --resume {}", session_id)
        }
        _ => pane.default_command.clone()
    }
}
```

**Rationale**:
- `--resume` with session ID is explicit and deterministic
- Working directory preservation is critical (Claude sessions are dir-scoped)
- Fallback to default command if no session ID

**Trade-offs**:
- Assumes shell can handle `cd && claude` syntax
- May need platform-specific handling

### Decision 4: Fallback Behavior

**Choice**: Configurable fallback with sensible default (fresh shell).

**Implementation**:
```rust
pub enum ClaudeResumeFallback {
    Shell,        // Fall back to default shell
    FreshClaude,  // Start new Claude session
    Error,        // Show error, let user decide
}
```

**Config**:
```toml
[claude]
auto_resume = true
resume_fallback = "shell"  # shell | fresh_claude | error
```

**Rationale**:
- Users have different preferences
- Shell is safest default (non-destructive)
- Config allows customization

**Trade-offs**:
- Additional config complexity
- Need to detect resume failure reliably

### Decision 5: Command Detection

**Choice**: Simple prefix matching with common patterns.

**Implementation**:
```rust
fn is_claude_command(cmd: &str) -> bool {
    let cmd = cmd.trim();
    cmd == "claude" ||
    cmd.starts_with("claude ") ||
    cmd.contains("&& claude") ||
    cmd.contains("; claude")
}

fn has_session_flag(cmd: &str) -> bool {
    cmd.contains("--session-id") ||
    cmd.contains("--resume") ||
    cmd.contains("--continue")
}
```

**Rationale**:
- Covers common invocation patterns
- Simple and fast
- Easy to extend

**Trade-offs**:
- May miss edge cases (aliased commands, scripts)
- False positives possible but unlikely

## Affected Components

| Component | Type of Change | Risk Level |
|-----------|----------------|------------|
| fugue-server/src/session/pane.rs | Add fields | Low |
| fugue-server/src/pty/spawn.rs | Command modification | Medium |
| fugue-server/src/pty/restore.rs | Resume logic | Medium |
| fugue-server/src/persistence/state.rs | Serialize new fields | Low |
| fugue-server/src/config.rs | Add claude config | Low |

## Implementation Order

### Phase 1: Pane Metadata Extension

1. Add `claude_session_id`, `claude_working_dir`, `is_claude_pane` to Pane
2. Initialize to None/false on pane creation
3. Update persistence serialization/deserialization
4. Verify round-trip works with tests
5. **Deliverable**: Pane can store and persist Claude session metadata

### Phase 2: Command Detection and Injection

1. Implement `is_claude_command()` detection
2. Implement `has_session_flag()` check
3. Implement `inject_session_id()` command modification
4. Hook into pane/PTY spawn path
5. Store generated session ID in pane
6. **Deliverable**: Claude panes get auto-assigned session IDs

### Phase 3: Restoration Logic

1. Implement `construct_restore_command()`
2. Hook into pane restoration path
3. Use `--resume <id>` when claude_session_id is set
4. Preserve working directory in restore command
5. **Deliverable**: Server restart triggers Claude resume

### Phase 4: Fallback Handling

1. Add config options for fallback behavior
2. Detect resume failure (exit code, timing, error patterns)
3. Implement fallback based on config
4. Log failures for debugging
5. **Deliverable**: Graceful handling of invalid sessions

### Phase 5: Testing and Polish

1. Comprehensive unit tests
2. Integration tests with mock Claude
3. Real-world testing with actual Claude
4. Documentation and examples
5. **Deliverable**: Production-ready feature

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Command injection breaks edge cases | Medium | Low | Thorough testing, config to disable |
| Session ID not captured before crash | Low | Medium | Write to WAL immediately after assignment |
| Claude resume fails silently | Low | Medium | Detect exit code, implement fallback |
| Working directory mismatch | Medium | Medium | Capture CWD at spawn time |
| User confusion about modified commands | Low | Low | Clear documentation, logging |

## Rollback Strategy

If implementation causes issues:
1. Remove command modification (disable auto-injection)
2. Remove restoration logic (spawn default command)
3. Fields can remain in Pane (unused but harmless)
4. No data migration needed - fields simply unused
5. Users fall back to manual `claude --resume`

## Testing Strategy

### Unit Tests

- `is_claude_command()` with various inputs
- `has_session_flag()` with various flag positions
- `inject_session_id()` preserves existing arguments
- `construct_restore_command()` output format
- Serialization/deserialization of new pane fields

### Integration Tests

- Spawn Claude pane, verify session ID captured
- Restart server, verify `--resume` used in command
- Invalid session ID fallback behavior
- Working directory preservation

### Manual Testing

- Full workflow: start Claude, have conversation, restart server, verify resume
- Test with various Claude CLI flags
- Test fallback scenarios
- Test with persistent session marking (if implemented)

## Implementation Notes

### Command Injection Location

Best place to inject is in the PTY spawn path:

```rust
// In pty/spawn.rs or similar
pub fn spawn_pty(&mut self, command: String, pane: &mut Pane) -> Result<()> {
    let final_command = prepare_pane_command(&command, pane);

    // Capture working directory
    pane.claude_working_dir = Some(std::env::current_dir()?);

    self.pty = portable_pty::native_pty_system()
        .openpty(PtySize { ... })?;

    self.child = self.pty.slave.spawn_command(
        CommandBuilder::new(shell)
            .arg("-c")
            .arg(&final_command)
    )?;

    Ok(())
}
```

### WAL Integration

Ensure session ID is captured in WAL immediately:

```rust
fn set_pane_session_id(&mut self, pane_id: PaneId, session_id: String) {
    if let Some(pane) = self.panes.get_mut(&pane_id) {
        pane.claude_session_id = Some(session_id);
        self.wal.write(WalEntry::PaneSessionId {
            pane_id,
            session_id
        })?;
    }
}
```

### Resume Failure Detection

```rust
async fn spawn_with_resume(pane: &Pane) -> Result<Child> {
    let cmd = construct_restore_command(pane);
    let child = spawn_command(&cmd)?;

    // Give Claude a moment to start or fail
    tokio::time::sleep(Duration::from_secs(2)).await;

    match child.try_wait() {
        Ok(Some(status)) if !status.success() => {
            // Resume failed, trigger fallback
            Err(Error::ResumeFailed)
        }
        _ => Ok(child)
    }
}
```

---
*This plan should be updated as implementation progresses.*
