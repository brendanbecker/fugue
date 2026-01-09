# Implementation Plan: BUG-006 - Viewport not sizing to terminal dimensions

## Overview

Fix the viewport sizing issue where ccmux-client renders panes at 80x24 regardless of actual terminal size when attaching to an existing session.

## Root Cause Summary

The client creates UI panes using the server's reported dimensions (80x24 default) instead of the client's actual terminal size, and no resize message is sent during the initial attach flow.

## Implementation Approach: Option A (Client-side resize on attach)

### Design Decision

We will implement Option A from PROMPT.md: the client sends resize messages immediately after attaching to a session. This approach was chosen because:

1. **No protocol changes** - Works with existing server code
2. **Simple implementation** - Single file change
3. **Correct semantics** - Client is the source of truth for its own terminal size
4. **Multi-client safe** - Each client sizes panes for its own viewport

### Trade-offs

| Consideration | Impact |
|--------------|--------|
| Brief visual flash | Low - resize happens immediately after attach |
| Extra network traffic | Low - only O(n) messages for n panes |
| Backward compatibility | Full - no protocol changes |

## Architecture

### Current Flow (Buggy)
```
Client                          Server
  |                               |
  |-- AttachSession ------------>|
  |                               |
  |<---- Attached (panes@80x24) --|
  |                               |
  | create UI panes @80x24        |
  | render @80x24 (WRONG!)        |
```

### Fixed Flow
```
Client                          Server
  |                               |
  |-- AttachSession ------------>|
  |                               |
  |<---- Attached (panes@80x24) --|
  |                               |
  | create UI panes @terminal_size|
  |                               |
  |-- Resize(pane1, term_size) ->|
  |-- Resize(pane2, term_size) ->|
  |     ...                       |
  |                               |
  | render @terminal_size (CORRECT)|
```

## Implementation Details

### File: `ccmux-client/src/ui/app.rs`

#### Change 1: Calculate pane dimensions from terminal size

In the `ServerMessage::Attached` handler, calculate pane dimensions based on the client's terminal size instead of using server-reported dimensions.

**Location**: `handle_server_message()` function, `ServerMessage::Attached` match arm (~line 622)

**Before**:
```rust
for pane_info in self.panes.values() {
    self.pane_manager.add_pane(pane_info.id, pane_info.rows, pane_info.cols);
    // ...
}
```

**After**:
```rust
// Calculate UI pane size from client terminal dimensions
let (term_cols, term_rows) = self.terminal_size;
let pane_rows = term_rows.saturating_sub(3);  // Account for borders (2) + status (1)
let pane_cols = term_cols.saturating_sub(2);  // Account for side borders

for pane_info in self.panes.values() {
    self.pane_manager.add_pane(pane_info.id, pane_rows, pane_cols);
    // ...
}
```

#### Change 2: Send resize messages to server

After creating UI panes, send resize messages to inform the server of the correct dimensions for PTY sizing.

**After the pane creation loop, add**:
```rust
// Notify server of correct pane sizes for PTY
for &pane_id in self.panes.keys() {
    self.connection
        .send(ClientMessage::Resize {
            pane_id,
            cols: pane_cols,
            rows: pane_rows,
        })
        .await?;
}
```

### Edge Cases

1. **Single-pane session**: Normal case, one resize sent
2. **Multi-pane session**: All panes resized to same dimensions (limitation: doesn't preserve split ratios)
3. **Tiny terminal**: `saturating_sub` prevents underflow
4. **Zero terminal size**: Unlikely but safe due to saturating operations

### Future Improvements (Out of Scope)

- Preserve split pane ratios on resize
- Support different sizes per pane
- Protocol enhancement for terminal size in AttachSession

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Race condition with user resize | Low | Low | Resize events are idempotent |
| Flash of incorrect size | Medium | Low | Resize happens immediately |
| Regression in resize behavior | Low | Medium | Comprehensive test coverage |

## Rollback Strategy

If the fix causes issues:
1. Revert the commit
2. The system returns to previous behavior (wrong initial size, correct after resize)
3. No data loss or corruption possible

## Testing Notes

Key scenarios to verify:
1. Large terminal attaching to session created in small terminal
2. Small terminal attaching to session created in large terminal
3. Multiple clients with different terminal sizes
4. Resize during attach (race condition)
5. Session with multiple panes
