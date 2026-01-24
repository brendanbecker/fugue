# BUG-022: CLI Command Applied to All Sessions Instead of First Only

## Priority: P1
## Status: Completed
## Created: 2026-01-11

## Problem Summary

When launching fugue with a custom command (e.g., `fugue claude --resume`), the CLI command was incorrectly applied to ALL new sessions created during that client's lifetime, not just the first session. This caused:

1. Server startup hangs when restoring sessions (all Claude panes tried to resume with potentially stale session IDs)
2. Unexpected behavior where pressing 'n' to create a new session would use the CLI command instead of the default_command from config

## Symptoms Observed

1. **Server timeout on startup**: Server hung during session restoration trying to spawn `claude --resume <session_id>` for multiple saved panes
2. **All new sessions used CLI command**: Every session created (via 'n' key or command bar) used the same CLI command instead of config defaults
3. **Connection timeout**: Client reported "Connection timeout after 2s" because server never created socket

## Steps to Reproduce

1. Configure `default_command = "claude"` in config
2. Run `fugue claude --resume xyz` with existing persisted sessions
3. Server hangs during restoration
4. Or: Launch fugue, create multiple sessions - all use the CLI command

## Expected Behavior

- CLI command applies only to the FIRST session the user creates/attaches to
- Subsequent sessions (created via 'n' key, command bar, or restored) should use the `default_command` from config
- Session restoration should use saved Claude session IDs, not CLI arguments

## Actual Behavior

- CLI command was stored in `session_command` field
- Used `session_command.clone()` for EVERY `CreateSession` call
- Never cleared after first use

## Root Cause

In `fugue-client/src/ui/app.rs`, the `session_command` field was cloned (not consumed) when creating sessions:

```rust
// Line 668 - command bar session creation
command: self.session_command.clone(),

// Line 848 - 'n' key session creation
command: self.session_command.clone(),
```

This meant the CLI command persisted for the entire client session.

## Fix Applied

Changed `clone()` to `take()` to consume the command on first use:

```rust
// Line 669
command: self.session_command.take(),

// Line 849
command: self.session_command.take(),
```

Now the CLI command is used once and cleared, so subsequent sessions fall back to `None` (which uses server's `default_command` from config).

## Files Changed

| File | Change |
|------|--------|
| `fugue-client/src/ui/app.rs` | Changed `session_command.clone()` to `session_command.take()` in two places |

## Testing

1. Run `fugue` - first session uses default_command from config
2. Run `fugue bash` - first session uses bash, subsequent sessions use default_command
3. Press 'n' after first session - new session uses default_command, not CLI arg
