# BUG-054: send_input submit:true doesn't trigger Enter in Gemini CLI

**Priority**: P2
**Component**: mcp
**Severity**: medium
**Status**: fixed

## Problem

`fugue_send_input` with `submit: true` doesn't actually submit the input in Gemini CLI. The text appears in the input field but Enter is not triggered. A separate `key: "Enter"` call is required.

## Code Analysis

Both code paths send the same byte:

```rust
// submit: true (bridge/handlers.rs:410-412)
if submit {
    data.push(b'\r');  // 0x0D
}

// key: "Enter" (keys.rs:114)
m.insert("Enter", b"\x0d");  // 0x0D
```

Same byte, different behavior. Why?

## Hypothesis

### 1. Separate Write Theory
Maybe Gemini CLI's TUI framework (likely ratatui/crossterm) processes input character-by-character and only recognizes Enter when it arrives as a distinct input event, not appended to text.

### 2. Terminal Line Discipline
The PTY line discipline might buffer the combined write differently than separate writes. With cooked mode or specific ICANON settings, the CR might be processed differently.

### 3. Input Event Detection
TUI frameworks often read raw input via `crossterm::event::read()` which returns discrete events. A combined write might not generate the expected key event for Enter.

## Investigation Steps

### Section 1: Verify Byte-Level Behavior
- [ ] Add logging to show exact bytes written to PTY
- [ ] Compare byte sequences for submit:true vs key:Enter
- [ ] Use `strace` or PTY logging to verify writes

### Section 2: Test with Simple Shell
- [ ] Test submit:true with plain bash (not a TUI app)
- [ ] If it works with bash, issue is TUI-specific
- [ ] Test with other TUI apps (Claude Code, htop with input)

### Section 3: Analyze TUI Input Handling
- [ ] Check how crossterm reads input events
- [ ] Determine if combined write generates proper Enter event
- [ ] Test if small delay between text and Enter helps

### Section 4: Potential Fixes

**Option A: Always send Enter separately**
```rust
// Write text first
handle.write_all(text.as_bytes())?;
// Small flush/sync
handle.flush()?;
// Then send Enter
if submit {
    handle.write_all(b"\r")?;
}
```

**Option B: Use key sequence for submit**
```rust
if submit {
    // Use the same path as key:"Enter"
    let enter_sequence = get_key_sequence("Enter").unwrap();
    data.extend_from_slice(enter_sequence);
}
```

**Option C: Add newline after carriage return**
```rust
if submit {
    data.push(b'\r');
    data.push(b'\n');  // CR+LF might be more universally recognized
}
```

## Acceptance Criteria

- [ ] `submit: true` triggers Enter in Gemini CLI
- [ ] `submit: true` continues to work with bash/shell
- [ ] `submit: true` works with Claude Code
- [ ] No regression in existing send_input functionality

## Workaround

Send text and Enter separately:
```json
// First call
{"input": "hello"}

// Second call
{"key": "Enter"}
```

## Recommended Fix

**Option A is the simplest and most reliable approach:**

Always send Enter as a separate write when `submit: true`:

```rust
// In handlers.rs send_input handler
if let Some(text) = input {
    handle.write_all(text.as_bytes())?;
}

if submit {
    // Send Enter as separate write - TUI frameworks expect discrete key events
    handle.write_all(b"\r")?;
}
```

This mirrors what the workaround does (two separate calls) but internally, giving users the simple `submit: true` API they expect.

## History

- 2026-01-17: Originally marked "fixed" but fix only documented workaround
- 2026-01-17: Reopened after QA testing confirmed `submit:true` still fails with Gemini CLI
- Verified: separate `key: "Enter"` call works, so Option A should work

## Related

- BUG-017: MCP send_input doesn't handle Enter key (completed)
- BUG-049: send_input with submit: true unreliable (completed)
- FEAT-093: Special keys support (completed)
