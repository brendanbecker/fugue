# BUG-048: TUI flickers during Claude Code spinner animation

**Priority**: P1
**Component**: tui, agents
**Severity**: medium (reduced from high after partial fix)
**Status**: in_progress

## Problem Statement

Once Claude Code is detected in a pane, the TUI flickers during "Thinking..." state due to spinner animation.

### Partial Fix Applied (commit `40402d1`)

Keystroke flicker is now fixed - `analyze()` only returns state on actual state changes.

### Remaining Issue

The spinner animation (pulsing snowflake/flower that expands and contracts down to a dot) in Claude's terminal output during "Thinking..." state causes rapid activity state transitions that still trigger excessive redraws.

## Evidence

**File**: `fugue-server/src/agents/claude/mod.rs` (lines 127-143)

The fix applied:
```rust
fn analyze(&mut self, text: &str) -> Option<AgentState> {
    // Only return state if inner detector reports a change
    if self.inner.analyze(text).is_some() {
        self.state()
    } else {
        None
    }
}
```

This correctly prevents returning state on every call. However, the inner `ClaudeDetector` reports a state change on each spinner frame because it detects activity transitions.

## Chain of Events Causing Spinner Flicker

1. Claude enters "Thinking..." state with spinner animation
2. Each spinner frame (pulsing expand/contract animation) is written to PTY
3. Inner detector sees activity and reports state change
4. `analyze()` returns `Some(state)` (correctly, per the fix)
5. Server broadcasts `PaneStateChanged` to TUI
6. TUI receives `PaneStateChanged` and sets `needs_redraw = true`
7. Screen flickers ~10 times per second during thinking

## Steps to Reproduce

1. Start fugue and attach to a session
2. Launch Claude Code in a pane (`claude-code` or `cc`)
3. Wait for Claude Code to be detected
4. Submit a prompt that requires thinking time
5. Observe screen flicker during the "Thinking..." spinner animation

## Expected Behavior

TUI should only redraw when meaningful state changes occur, not on spinner animation frames.

## Actual Behavior

TUI flickers rapidly during thinking/processing state due to spinner animation triggering state changes.

## Root Cause

The spinner animation causes the inner `ClaudeDetector` to report activity state changes on each frame. While these are technically "state changes", they are not meaningful changes that require a full TUI redraw.

## Proposed Fix: Debounce State Changes

Add debouncing to suppress rapid-fire state changes from spinner animation:

```rust
use std::time::{Duration, Instant};

const STATE_DEBOUNCE_MS: u64 = 100; // 100ms debounce window

struct ClaudeAgentDetector {
    inner: ClaudeDetector,
    last_state_change: Option<Instant>,
}

impl AgentDetector for ClaudeAgentDetector {
    fn analyze(&mut self, text: &str) -> Option<AgentState> {
        if self.inner.analyze(text).is_some() {
            let now = Instant::now();

            // Debounce: only report change if enough time has passed
            let should_report = self.last_state_change
                .map(|last| now.duration_since(last) > Duration::from_millis(STATE_DEBOUNCE_MS))
                .unwrap_or(true);

            if should_report {
                self.last_state_change = Some(now);
                self.state()
            } else {
                None
            }
        } else {
            None
        }
    }
}
```

### Alternative Approaches

1. **Debounce in TUI**: Debounce `needs_redraw` instead of at the detector level
2. **Filter spinner patterns**: Detect and ignore braille spinner characters in the detector
3. **Coalesce broadcasts**: Have server coalesce rapid `PaneStateChanged` broadcasts

## Implementation Tasks

### Section 1: Fix Implementation
- [ ] Add debounce mechanism to `ClaudeAgentDetector.analyze()`
- [ ] Choose appropriate debounce window (100ms suggested)
- [ ] Ensure first state change is always reported immediately

### Section 2: Testing
- [ ] Add unit test for debounce behavior
- [ ] Test that rapid calls within debounce window return `None`
- [ ] Test that calls after debounce window return `Some`
- [ ] Manual test: verify no flicker during Claude "Thinking..." state

### Section 3: Verification
- [ ] Confirm TUI no longer flickers during spinner animation
- [ ] Verify agent state still updates correctly for real transitions
- [ ] Verify initial Claude detection still works immediately
- [ ] No regressions in existing functionality

## Acceptance Criteria

- [x] `analyze()` returns `None` when no state change occurs (DONE - commit 40402d1)
- [ ] Rapid state changes from spinner are debounced
- [ ] TUI does not flicker during "Thinking..." spinner animation
- [ ] Initial state detection is not delayed by debounce
- [ ] Real state transitions (idle → working → complete) are still detected
- [ ] No regressions in agent detection functionality

## Related Work Items

- **FEAT-084**: Abstract agent state (introduced the wrapper)
- **FEAT-082**: Adaptive layout engine (added `needs_redraw` to `PaneStateChanged` handler)

## Notes

The 100ms debounce window is a reasonable starting point:
- Spinner typically animates at ~10 fps (100ms per frame)
- Human perception of "instant" is ~100ms
- Real state transitions (idle → working → complete) happen on longer timescales

If 100ms proves too aggressive, consider 50ms or making it configurable.
