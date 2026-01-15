# BUG-041: Claude Code Crashes on Paste Inside ccmux

**Priority**: P1
**Component**: pty / client
**Severity**: high
**Status**: new

## Problem Statement

Pasting moderate-sized content (~3.5KB) into Claude Code running inside a ccmux pane causes Claude Code to crash and attempt to output the pasted content to the underlying shell. The same content pastes successfully into Claude Code running outside ccmux, and pastes successfully into vi running inside ccmux.

## Evidence

### Observations
- **File pasted**: `docs/scratch/grok/ccmux-peering-design.md` (3,501 bytes)
- **Size**: 3.5KB (well below the 64KB chunking threshold from BUG-011 fix)
- **Claude Code outside ccmux**: Handles hundreds of large pastes without any crashes
- **Claude Code inside ccmux**: Crashed on this 3.5KB paste
- **Vi inside ccmux**: Handled the same paste without issues
- **After crash**: Content attempted to output to shell

### Key Insight: Not a Chunking Issue

The paste size (3.5KB) is well below the 64KB chunking threshold introduced in BUG-011's fix. This means ccmux should have sent the paste as a **single message** without chunking, ruling out chunking-related hypotheses.

### Application-Specific Behavior

- **Claude Code**: Crashes inside ccmux, works perfectly outside ccmux
- **Vi**: Works fine inside ccmux
- **Conclusion**: This is specific to how ccmux's PTY layer interacts with Claude Code's input handling

## Root Cause Hypotheses

### Hypothesis 1: Bracketed Paste Mode Not Properly Forwarded
Claude Code likely expects bracketed paste mode escape sequences (`\e[200~` start, `\e[201~` end). If ccmux's PTY layer doesn't properly forward these sequences, Claude Code's paste parser may fail.

**Investigation**:
- Check if ccmux enables bracketed paste mode on PTY master
- Verify escape sequences are forwarded unmodified
- Test with `printf '\e[?2004h'` to manually enable bracketed paste

**Files to check**:
- `ccmux-server/src/pty/mod.rs` - PTY initialization
- `ccmux-client/src/input/mod.rs` - Input event handling

### Hypothesis 2: Character Encoding or Escape Sequence Mangling
The markdown content may contain characters (backticks, hyphens, unicode) that get corrupted through ccmux's PTY layer in a way that breaks Claude Code's parser but not vi's.

**Investigation**:
- Log raw bytes received by PTY
- Compare byte-for-byte against original content
- Test with simpler content (plain ASCII)

### Hypothesis 3: Input Delivery Rate/Timing
Even without chunking, ccmux might deliver paste data at a different rate or with different write() call boundaries than a native terminal. Claude Code may have timing assumptions that vi doesn't.

**Investigation**:
- Measure paste delivery timing (direct vs ccmux)
- Check if PTY writes are batched/buffered differently
- Test with synthetic paste events at varying speeds

### Hypothesis 4: PTY Termios Settings
Claude Code may rely on specific termios flags (like `ICANON`, `ECHO`, `ISIG`) that ccmux sets differently than a standard terminal.

**Investigation**:
- Dump termios settings from ccmux PTY vs normal terminal
- Compare with Claude Code's expected settings
- Check if Claude Code uses raw mode differently

### Hypothesis 5: Input Buffer Size Limits
Claude Code's PTY input handler may have a smaller buffer size expectation when running under certain PTY configurations, even for sizes well below 64KB.

**Investigation**:
- Check Claude Code's buffer allocation
- Test with progressively larger pastes (1KB, 2KB, 3KB, 4KB)
- Find exact size threshold where crash occurs

## Steps to Reproduce

1. Start ccmux daemon: `ccmux-server start`
2. Attach ccmux client: `ccmux-client`
3. Launch Claude Code in a pane
4. Paste `docs/scratch/grok/ccmux-peering-design.md` content (3,501 bytes)
5. Observe Claude Code crash with content echoing to shell

## Expected Behavior

Claude Code should handle the paste gracefully, just as it does outside ccmux and just as vi does inside ccmux.

## Actual Behavior

Claude Code crashes. Content attempts to output to the underlying shell instead of being processed by Claude Code.

## Investigation Tasks

### Section 1: Reproduce and Characterize

- [ ] Reproduce the crash with ccmux-peering-design.md content
- [ ] Test with progressively smaller content to find minimum crash size
- [ ] Test with progressively larger content to find pattern
- [ ] Test with plain ASCII vs markdown with special characters
- [ ] Test with other applications (vim, emacs, nano, bash)
- [ ] Record exact crash behavior (error messages, shell output)

### Section 2: PTY Layer Analysis

- [ ] Add debug logging to PTY input path
- [ ] Log raw bytes written to PTY master
- [ ] Compare PTY termios settings (ccmux vs native)
- [ ] Check bracketed paste mode support in PTY
- [ ] Trace escape sequence forwarding

### Section 3: Input Handling Comparison

- [ ] Strace Claude Code outside ccmux during paste
- [ ] Strace Claude Code inside ccmux during paste
- [ ] Compare read() syscalls, timing, and data chunks
- [ ] Check for differences in signal delivery (SIGWINCH, etc.)

### Section 4: Bracketed Paste Mode Testing

- [ ] Verify ccmux forwards `\e[200~` and `\e[201~` sequences
- [ ] Test manual bracketed paste: `printf '\e[200~<content>\e[201~'`
- [ ] Check if problem persists with bracketed paste disabled
- [ ] Compare with other terminals that support bracketed paste

## Potential Fixes

Based on root cause identified:

### If bracketed paste mode issue:
- [ ] Ensure PTY master has bracketed paste enabled
- [ ] Forward bracketed paste escape sequences unmodified
- [ ] Add integration test for bracketed paste

### If character encoding issue:
- [ ] Fix UTF-8 handling in PTY input path
- [ ] Ensure escape sequences aren't corrupted
- [ ] Add tests for markdown and special characters

### If timing/delivery issue:
- [ ] Match native terminal write() behavior
- [ ] Adjust PTY buffering strategy
- [ ] Add configurable paste rate limiting

### If termios issue:
- [ ] Adjust PTY termios to match standard terminal
- [ ] Allow applications to set their own termios flags
- [ ] Document ccmux PTY configuration

### General:
- [ ] Add error recovery for crashed applications
- [ ] Log detailed diagnostics when paste-related crashes occur
- [ ] Add integration tests with Claude Code

## Acceptance Criteria

- [ ] Root cause identified and documented
- [ ] Claude Code handles 3.5KB paste inside ccmux without crashing
- [ ] Claude Code handles larger pastes (10MB+) inside ccmux consistently
- [ ] No regression in vi or other applications
- [ ] Integration test added to prevent regression
- [ ] Debug logging available for future diagnosis

## Related Bugs

- **BUG-011**: Large paste crashes session (fixed with chunking)
  - Different issue: BUG-011 was ccmux itself crashing on >16MB pastes
  - This bug: Application crashes on 3.5KB paste, ccmux stays stable

## Files to Investigate

| File | Reason |
|------|--------|
| `ccmux-server/src/pty/mod.rs` | PTY initialization and termios setup |
| `ccmux-server/src/pty/output.rs` | PTY output handling |
| `ccmux-client/src/input/mod.rs` | Client input event processing |
| `ccmux-client/src/ui/app.rs` | Paste event handling (BUG-011 chunking code) |
| `ccmux-protocol/src/message.rs` | Message protocol for paste data |

## Notes

This is a **P1 bug** because:
- Affects the primary use case (Claude Code inside ccmux)
- Occurs with moderate-sized, common content (~3.5KB markdown file)
- Claude Code works perfectly outside ccmux (proven with hundreds of pastes)
- Suggests fundamental PTY incompatibility with Claude Code's expectations

The fact that **vi handles the same paste fine** suggests the issue is specific to how Claude Code processes PTY input, not a general ccmux PTY bug. However, since Claude Code works perfectly outside ccmux, this indicates ccmux's PTY implementation differs from standard terminals in a way that breaks Claude Code.

**Priority is high** because this undermines ccmux's value proposition: reliable AI-assisted terminal management with Claude Code.
