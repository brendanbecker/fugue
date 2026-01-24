# BUG-017: MCP send_input doesn't handle Enter key

**Priority**: P1
**Component**: fugue-server/mcp
**Severity**: high
**Status**: new

## Problem Statement

The MCP `fugue_send_input` tool sends text to a pane but cannot submit it with Enter. Neither `\n` (literal), actual newline characters, nor carriage returns trigger an Enter keypress in the PTY.

## Evidence

### Test 1: Escaped newline
```json
{"input": "some text\\n"}
```
**Result**: Sends literal `\n` characters to the pane, not Enter keypress.

### Test 2: Actual newline in JSON
```json
{"input": "some text\n"}
```
**Result**: Creates a new line in the input buffer but doesn't submit.

### Test 3: Carriage return
```json
{"input": "some text\r"}
```
**Result**: No effect or same as newline.

## Steps to Reproduce

1. Start fugue server and TUI client
2. Create a pane with Claude Code running
3. Use MCP tool `fugue_send_input` to send text
4. Attempt to send Enter to submit the input
5. Observe: Text appears but is never submitted

## Expected Behavior

- `\n` in the input string should be converted to Enter keypress (0x0D or 0x0A)
- Text followed by `\n` should submit to the running process

## Actual Behavior

- `\n` is either sent literally or creates a visual newline without submitting
- Cannot programmatically submit input to Claude Code via MCP

## Root Cause (Suspected)

The `send_input` handler likely writes the string directly to PTY stdin without interpreting escape sequences. Need to:
1. Parse `\n` escape sequences and convert to actual newline bytes
2. Or send `\r` (0x0D) which is what Enter typically sends to a PTY

## Affected Files

- `fugue-server/src/mcp/handlers.rs` - send_input implementation
- `fugue-server/src/mcp/tools.rs` - tool definition

## Implementation Tasks

### Section 1: Investigation
- [ ] Trace send_input flow from MCP handler to PTY write
- [ ] Check how input string is processed before PTY write
- [ ] Verify what byte sequence Enter should send (likely `\r` = 0x0D)

### Section 2: Fix
- [ ] Add escape sequence parsing to send_input handler
- [ ] Convert `\n` to `\r` (carriage return) for PTY submission
- [ ] Consider supporting other escape sequences (`\t`, `\x1b` for Escape, etc.)

### Section 3: Testing
- [ ] Add test for send_input with newline
- [ ] Test that Enter submits input to shell
- [ ] Test with Claude Code prompt submission

## Acceptance Criteria

- [ ] `fugue_send_input` with `\n` submits input to the PTY process
- [ ] Text can be programmatically sent and submitted to Claude Code
- [ ] Existing send_input functionality unchanged for regular text
- [ ] Tests added to prevent regression

## Notes

This is a critical bug for MCP-based orchestration. Without working Enter key support, Claude instances cannot be programmatically controlled via MCP tools.

## Workaround

User must manually press Enter in the TUI to submit input sent via MCP.
