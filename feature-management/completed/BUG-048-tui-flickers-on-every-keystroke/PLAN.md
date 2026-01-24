# Implementation Plan: BUG-048

**Work Item**: [BUG-048: TUI flickers on every keystroke when Claude Code is detected](PROMPT.md)
**Component**: tui
**Priority**: P1
**Created**: 2026-01-16

## Overview

Once Claude Code is detected in a pane, every keystroke causes the entire screen to flash/flicker. This is caused by the agent detector returning state on every call instead of only when state changes.

## Architecture Decisions

- **Approach**: Fix the `analyze()` method in `ClaudeAgentDetector` to respect the change-detection return value from the inner `ClaudeDetector`
- **Trade-offs**: None - this is a bug fix that restores intended behavior

## Affected Components

| Component | Type of Change | Risk Level |
|-----------|----------------|------------|
| `fugue-server/src/agents/claude/mod.rs` | Bug fix | Low |

## Dependencies

None - this is a self-contained fix.

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Breaking agent detection | Low | High | Verify detection still works after fix |
| Missing state updates | Low | Medium | Test that state transitions are still reported |

## Rollback Strategy

If implementation causes issues:
1. Revert the single-line change to `analyze()`
2. Verify system returns to previous (flickering) state
3. Document what went wrong in comments.md

## Implementation Notes

The fix is a one-line change:

**Before:**
```rust
fn analyze(&mut self, text: &str) -> Option<AgentState> {
    let _change = self.inner.analyze(text);
    if self.inner.is_claude() {
        self.state()
    } else {
        None
    }
}
```

**After:**
```rust
fn analyze(&mut self, text: &str) -> Option<AgentState> {
    if self.inner.analyze(text).is_some() {
        self.state()
    } else {
        None
    }
}
```

This preserves the change-detection semantics that the inner `ClaudeDetector` was designed to provide.

---
*This plan should be updated as implementation progresses.*
