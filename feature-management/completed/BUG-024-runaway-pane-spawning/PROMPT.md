# BUG-024: Runaway Pane Spawning via Sideband Command Feedback Loop

**Priority**: P0 (Critical)
**Component**: sideband-parser
**Status**: fixed
**Created**: 2026-01-11
**Fixed**: 2026-01-11

## Summary

The sideband parser uses plain XML-like tags (`<fugue:spawn ... />`) that can appear in normal terminal output (e.g., grep results searching source code). When this happens, the parser interprets the output as actual spawn commands, creating new panes. Since spawned panes also have sideband parsing enabled, this creates an infinite feedback loop that crashes fugue.

## Root Cause

1. Sideband parser regex `<fugue:(\w+)([^>]*)/>` matches plain text patterns
2. Grep/cat of source files containing sideband commands triggers false matches
3. `PtyOutputPoller::spawn_with_sideband()` enables sideband parsing on newly spawned panes (line 477 of output.rs)
4. New panes inherit the ability to spawn more panes, creating exponential growth

## Reproduction

1. Start fugue with a Claude Code session
2. Run a grep command that searches sideband source files:
   ```
   grep -r "spawn" fugue-server/src/sideband/
   ```
3. Output contains literal `<fugue:spawn ... />` strings
4. Parser executes them as commands, spawning panes uncontrollably
5. System crashes from resource exhaustion

## Fix Strategy

### Part 1: Escape Sequence Prefix (Primary Fix)

Change sideband commands to require an OSC (Operating System Command) escape sequence prefix that won't appear in normal text output:

**Current format:**
```
<fugue:spawn direction="vertical" />
```

**New format:**
```
\x1b]fugue:spawn direction="vertical"\x07
```

Using OSC format: `ESC ] ... BEL` or `ESC ] ... ESC \`

This ensures commands can only be triggered by programs that intentionally emit escape sequences, not by grep/cat of source files.

### Part 2: Configurable Spawn Limits (Safety Net)

Add configurable limits as a safety net for intentional high-spawn scenarios:

1. **`max_spawn_depth`** - Maximum chain depth (pane spawning pane spawning pane...)
2. **`max_panes_per_session`** - Maximum total panes in a session
3. **`spawn_rate_limit`** - Optional: max spawns per second

Make these configurable so power users can adjust limits for legitimate multi-pane workflows.

## Implementation Tasks

### Section 1: Escape Sequence Parser

- [ ] Update `SidebandParser` regex to match OSC format: `\x1b]fugue:(\w+)([^\x07\x1b]*?)(?:\x07|\x1b\\)`
- [ ] Support both BEL (`\x07`) and ST (`ESC \`) terminators
- [ ] Update all documentation and tests
- [ ] Ensure backward compatibility period (warn on old format, still parse it)

### Section 2: Spawn Depth Tracking

- [ ] Add `spawn_depth` field to `PtyOutputPoller` or pass through executor
- [ ] Track depth when spawning new panes (parent_depth + 1)
- [ ] Add `max_spawn_depth` config option (default: 5)
- [ ] Reject spawn commands that exceed depth limit with warning

### Section 3: Session Pane Limits

- [ ] Add `max_panes_per_session` config option (default: 50)
- [ ] Check pane count before spawning
- [ ] Reject spawn commands that exceed limit with warning

### Section 4: Configuration

- [ ] Add spawn limits to server config
- [ ] Allow per-session overrides if needed
- [ ] Document configuration options

### Section 5: Testing

- [ ] Test that old XML format no longer triggers commands
- [ ] Test that new OSC format works correctly
- [ ] Test spawn depth limit enforcement
- [ ] Test pane count limit enforcement
- [ ] Test that grep output no longer causes runaway spawning

## Acceptance Criteria

- [ ] Grep/cat of source files does not trigger sideband commands
- [ ] New OSC escape sequence format is required for commands
- [ ] Spawn depth is tracked and limited (configurable)
- [ ] Total panes per session are limited (configurable)
- [ ] Warnings logged when limits are hit
- [ ] Existing Claude Code integration continues to work (update escape sequences)

## Notes

- Claude Code will need to be updated to emit the new escape sequence format
- Consider a transition period where both formats work but old format logs deprecation warning
- The OSC format is standard terminal escape sequence protocol, widely supported

## Resolution

### Fix Applied

**Part 1: OSC Escape Sequence Format (Primary Fix)**

Changed sideband command format from plain XML to OSC escape sequences:

- **Old format** (vulnerable): `<fugue:spawn direction="vertical" />`
- **New format** (secure): `\x1b]fugue:spawn direction="vertical"\x07`

The OSC format (ESC ] ... BEL) won't appear in grep/cat output of source files since it requires actual escape characters.

**Files changed:**
- `fugue-server/src/sideband/parser.rs`: Updated regexes and parsing logic
- `fugue-server/src/sideband/mod.rs`: Updated documentation and integration tests

**Part 2: Configurable Spawn Limits (Safety Net)**

Added `SpawnLimits` configuration with:
- `max_spawn_depth`: Maximum chain depth (default: 5)
- `max_panes_per_session`: Maximum sideband-spawned panes (default: 50)

Spawn attempts exceeding limits return `ExecuteError::ExecutionFailed` with descriptive message.

**Files changed:**
- `fugue-server/src/sideband/async_executor.rs`: Added `SpawnLimits` struct and limit checking

### Test Coverage

- 95 sideband tests pass
- Added explicit tests for old XML format being ignored
- Added test for grep output not triggering commands

### Breaking Change

This is a breaking change for any existing Claude Code integration using sideband commands. Clients must update to use the new OSC format.
