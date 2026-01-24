# Implementation Plan: FEAT-025

**Work Item**: [FEAT-025: Pane Output Rendering](PROMPT.md)
**Component**: fugue-client
**Priority**: P0
**Created**: 2026-01-09

## Overview

Wire ServerMessage::Output data to pane rendering in the client UI. This connects PTY output from the server to the client's terminal emulation and display.

## Architecture Decisions

### Pane Type Bridging

There are two "pane" types that need to be reconciled:

1. **`PaneInfo`** (from `fugue-protocol`) - Lightweight struct with metadata:
   - `id: Uuid`
   - `window_id: Uuid`
   - `state: PaneState`
   - `title: Option<String>`
   - `cwd: Option<String>`

2. **`Pane`** (from `fugue-client/src/ui/pane.rs`) - Full UI pane with terminal:
   - `id: Uuid`
   - `parser: Parser` (VT100 terminal emulation)
   - `title, cwd, focus_state, pane_state`
   - `scroll_offset, show_scrollbar`

**Decision**: Add a `PaneManager` to `App` that maintains UI `Pane` instances. Sync with `PaneInfo` on lifecycle events.

```rust
// In App struct
panes: HashMap<Uuid, PaneInfo>,      // Protocol pane metadata
pane_manager: PaneManager,            // UI panes with terminal state
```

### Output Flow

```
Server PTY Output
    |
    v
ServerMessage::Output { pane_id, data }
    |
    v
App::handle_server_message()
    |
    v
pane_manager.process_output(pane_id, &data)
    |
    v
Pane::process_output() -> Parser::process()
    |
    v
Screen state updated (ready for render)
```

### Rendering Flow

```
App::draw()
    |
    v
layout_manager.calculate_rects(area)
    |
    v
for (pane_id, rect) in rects:
    pane = pane_manager.get(pane_id)
    render_pane(pane, rect, buf, tick_count)
    |
    v
PseudoTerminal::render() -> Screen to Buffer
```

## Affected Components

| Component | Type of Change | Risk Level |
|-----------|----------------|------------|
| fugue-client/src/ui/app.rs | Modify - add PaneManager, wire output | Medium |
| fugue-client/src/ui/pane.rs | None - already complete | Low |
| fugue-client/src/ui/layout.rs | None - already complete | Low |

## Dependencies

- FEAT-023 (PTY Output Polling) - Server sends `ServerMessage::Output`
- FEAT-022 (Message Routing) - Messages reach the client

Both dependencies provide the `ServerMessage::Output { pane_id, data }` that this feature consumes.

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Performance issues with large output | Medium | Medium | tui-term is optimized; test with large files |
| UTF-8 boundary issues | Low | Low | tui-term handles partial sequences |
| State sync between PaneInfo and Pane | Medium | Medium | Single source of truth in PaneManager |
| Resize race conditions | Low | Medium | Queue resize events, handle in order |

## Implementation Phases

### Phase 1: Add PaneManager to App
- Add `pane_manager: PaneManager` field to `App`
- Initialize in `App::new()`
- No behavior change yet

### Phase 2: Wire Pane Lifecycle
- On `PaneCreated`: create UI Pane via `pane_manager.add_pane()`
- On `PaneClosed`: remove via `pane_manager.remove_pane()`
- On `Attached`: sync all panes

### Phase 3: Wire Output
- In `ServerMessage::Output` handler:
  ```rust
  if let Some(pane) = self.pane_manager.get_mut(&pane_id) {
      pane.process_output(&data);
  }
  ```
- Remove TODO comment

### Phase 4: Update Rendering
- Modify `draw()` to iterate over `pane_manager` panes
- Use `render_pane()` from `ui/pane.rs`
- Integrate with layout system

### Phase 5: Handle Resize
- On terminal resize event, resize all panes
- Send `ClientMessage::Resize` to server

## Rollback Strategy

If implementation causes issues:
1. Revert to stub behavior (ignore output)
2. Verify server still works without rendering
3. Debug in isolation with mock output

## Testing Strategy

1. **Manual Testing**:
   - Run `ls -la` and verify output appears
   - Run `cat /etc/passwd` and verify scrolling
   - Open vim/nano and verify cursor movement

2. **Multi-Pane Testing**:
   - Create two panes
   - Run commands in each
   - Verify independent rendering

3. **Performance Testing**:
   - Run `cat large_file.txt`
   - Run `yes | head -1000`
   - Verify no UI lag

4. **Resize Testing**:
   - Resize terminal window
   - Verify panes resize correctly
   - Verify no visual corruption

## Implementation Notes

<!-- Add notes during implementation -->

---
*This plan should be updated as implementation progresses.*
