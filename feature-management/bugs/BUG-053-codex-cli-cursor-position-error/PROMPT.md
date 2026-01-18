# BUG-053: Codex CLI fails with cursor position error inside ccmux pane

**Priority**: P1
**Component**: pty
**Severity**: high
**Status**: new

## Problem

Codex CLI crashes immediately when launched inside a ccmux pane with:
```
Error: The cursor position could not be read within a normal duration
EXIT CODE: 1
```

## Technical Analysis

### Terminal Escape Sequence

Codex sends this sequence on startup:
```
[?2004h  - Enable bracketed paste mode
[>7u     - Request secondary device attributes (xterm)
[?1004h  - Enable focus reporting
[6n      - DSR (Device Status Report) - REQUEST CURSOR POSITION
```

The `[6n` (ESC [ 6 n) sequence is a cursor position request. The terminal should respond with:
```
ESC [ <row> ; <col> R
```

### Suspected Cause

ccmux's PTY/terminal emulation layer may:
1. Not respond to DSR `[6n` requests at all
2. Respond too slowly (Codex has a timeout)
3. Not route the response back through the PTY correctly

### Comparison with Other CLIs

| CLI | DSR [6n | Status in ccmux |
|-----|---------|-----------------|
| Claude Code | Unknown | Works |
| Gemini CLI | Unknown | Works |
| Codex CLI | Required | **Fails** |

## Investigation Steps

### Section 1: Verify DSR Handling

- [ ] Check if ccmux terminal emulator handles `[6n`
- [ ] Search codebase for DSR or cursor position handling
- [ ] Check `ccmux-server/src/pty/` for escape sequence processing

### Section 2: Test DSR Response

- [ ] Create test pane and send `printf '\033[6n'`
- [ ] Check if response appears in pane output
- [ ] Measure response latency

### Section 3: Implement Fix

- [ ] If DSR not handled: implement cursor position response
- [ ] If response slow: optimize the response path
- [ ] If routing issue: fix PTY read/write flow

### Section 4: Verify Fix

- [ ] Test Codex CLI launches successfully
- [ ] Ensure no regressions for Claude/Gemini
- [ ] Add integration test for DSR handling

## Files to Investigate

| File | Purpose |
|------|---------|
| `ccmux-server/src/pty/` | PTY handling |
| `ccmux-server/src/session/pane.rs` | Pane I/O |
| Possibly using external crate | Check dependencies for terminal emulation |

## Acceptance Criteria

- [ ] Codex CLI launches successfully inside ccmux pane
- [ ] DSR `[6n` returns cursor position within acceptable timeout
- [ ] No regressions for existing CLI tools (Claude, Gemini)
- [ ] Integration test verifies DSR response

## Notes

### DSR Escape Sequence Reference

```
Request:  ESC [ 6 n       (CSI 6 n)
Response: ESC [ Pr ; Pc R (CSI Pr ; Pc R)

Where:
  Pr = cursor row (1-based)
  Pc = cursor column (1-based)
```

### Workaround

None known. Codex CLI requires cursor position to initialize.

## Related

- Terminal emulation standards (ECMA-48, VT100)
- Similar issue may affect other CLIs that require DSR
