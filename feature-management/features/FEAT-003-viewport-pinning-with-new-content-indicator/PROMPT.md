# FEAT-003: Viewport Pinning with New Content Indicator

**Priority**: P2
**Component**: tui
**Type**: new_feature
**Estimated Effort**: medium
**Business Value**: high

## Overview

When user scrolls up in a pane, new output buffers without yanking viewport. A visual indicator shows the number of new lines below (e.g., "▼ 47 new lines below") with click or keypress to jump to bottom. Supports both smooth scroll and instant jump options.

## Benefits

- Users can review previous output without losing context when new content arrives
- Significantly improves experience when monitoring long-running processes
- Essential for reviewing Claude Code output without being yanked to latest line
- Visual feedback ensures users know when new content is available
- Multiple navigation options (keyboard, click) accommodate different workflows

## Requirements

### Core Functionality
- Track scroll position per-pane (viewport offset from bottom)
- Detect when new content arrives while scrolled up
- Buffer new output without auto-scrolling when viewport is pinned
- Render "▼ N new lines below" indicator when pinned with new content

### Navigation
- Keybinding to jump to bottom (e.g., `G` or `Ctrl+End`)
- Click handler on indicator to jump to bottom
- Configuration option for smooth scroll vs instant jump

### Current State Analysis
- Pane dimensions (cols, rows) are already tracked
- Server broadcasts PaneOutput messages
- No scroll position tracking exists
- Client UI is a stub ("UI components - to be implemented")
- No viewport API in protocol

## Affected Files

- `ccmux-protocol/src/lib.rs` - Add viewport messages to protocol
- `ccmux-client/src/ui.rs` - Implement rendering and indicator
- `ccmux-server/src/session/pane.rs` - Track scroll state server-side

## Implementation Tasks

### Section 1: Protocol Extension
- [ ] Add `ViewportPosition` message type to protocol
- [ ] Add `SetViewportOffset` client-to-server message
- [ ] Add `ViewportUpdated` server-to-client message
- [ ] Define viewport state structure (offset, pinned status, new_lines_count)

### Section 2: Server-Side Scroll State
- [ ] Add scroll position tracking to pane state
- [ ] Track "pinned" status when user scrolls up
- [ ] Count new lines received while pinned
- [ ] Handle viewport position updates from client

### Section 3: Client UI Implementation
- [ ] Implement scroll position tracking in UI state
- [ ] Render pane content with viewport offset
- [ ] Implement scroll input handling (mouse wheel, keyboard)
- [ ] Detect and set "pinned" state on scroll up

### Section 4: New Content Indicator
- [ ] Design indicator widget ("▼ N new lines below")
- [ ] Position indicator at bottom of pane
- [ ] Update line count as new content arrives
- [ ] Style indicator to be noticeable but not intrusive

### Section 5: Navigation Actions
- [ ] Implement "jump to bottom" action
- [ ] Add keybinding (G and/or Ctrl+End)
- [ ] Add click handler on indicator
- [ ] Implement smooth scroll option
- [ ] Clear indicator and unpin on jump

### Section 6: Configuration
- [ ] Add scroll behavior options to config schema
- [ ] Implement smooth_scroll vs instant_jump setting
- [ ] Add configurable keybindings for scroll actions

### Section 7: Testing
- [ ] Unit tests for viewport state management
- [ ] Integration tests for scroll behavior
- [ ] Test indicator updates with rapid output
- [ ] Test edge cases (empty pane, single line, overflow)

## Acceptance Criteria

- [ ] Scrolling up pins the viewport and prevents auto-scroll
- [ ] New lines are counted and displayed in indicator
- [ ] Indicator shows correct count (e.g., "▼ 47 new lines below")
- [ ] Pressing `G` or `Ctrl+End` jumps to bottom
- [ ] Clicking indicator jumps to bottom
- [ ] Indicator disappears when at bottom
- [ ] Smooth scroll option works when configured
- [ ] No performance degradation with rapid output

## Technical Notes

### Viewport State Model
```rust
struct ViewportState {
    /// Lines from bottom (0 = at bottom, following new content)
    offset_from_bottom: usize,
    /// Whether viewport is pinned (user scrolled up)
    is_pinned: bool,
    /// New lines received while pinned
    new_lines_since_pin: usize,
}
```

### Indicator Rendering
The indicator should appear at the bottom-right of the pane content area:
```
┌─ Pane Title ────────────────────┐
│ ... previous output ...         │
│ ... more output ...             │
│ ... visible content ...         │
│                 ▼ 47 new lines  │
└─────────────────────────────────┘
```

### Protocol Messages
```rust
// Client -> Server
SetViewportOffset { pane_id: PaneId, offset: usize }

// Server -> Client
ViewportUpdated { pane_id: PaneId, state: ViewportState }
```

## Dependencies

None - this is a foundational UI feature.

## Notes

- Consider debouncing indicator updates during rapid output to prevent flicker
- The indicator should use a distinct color (e.g., yellow/orange) to be noticeable
- May want to show indicator briefly even when following to acknowledge new content
- Future enhancement: click on indicator could show preview of latest content
