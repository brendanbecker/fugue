# BUG-070: Session Switch Rendering Corruption

## Status: RESOLVED

## Summary
When switching between sessions OR when a session is killed/closed, the terminal display becomes severely corrupted with visual artifacts including:
- Characters from the old session bleeding into the new display
- Text arranged diagonally across the screen
- Overlapping content from multiple panes
- Numbers and fragments scattered randomly

The `:redraw` command did not fix the corruption, and closing a session appeared to "hang" the client (actually just rendering the old content over the session selection UI).

## Root Cause

The issue stems from how Ratatui's differential rendering interacts with session switching:

1. When switching sessions, the client receives `ServerMessage::Attached` with completely new pane content
2. New VT100 parsers are created for the new session's panes
3. However, Ratatui's internal buffer still contains the OLD session's rendered content
4. Ratatui computes a diff between the new frame and its stale buffer
5. This diff is incomplete/incorrect because the content changed entirely
6. Result: visual corruption with artifacts from both sessions

The `terminal.clear()` call that would fix this was **commented out** in the main loop (line 145 of app.rs) to prevent flicker on minor state changes like `ClaudeStateChanged` events.

## Solution

Add a separate `needs_clear` flag to distinguish between:
- **Minor updates** (`needs_redraw`): State changes where differential rendering works fine
- **Major layout changes** (`needs_clear`): Session switches where a full clear is required

### Changes Made

1. **state.rs**: Added `needs_clear: bool` field to `ClientState`

2. **app.rs** main loop: Check `needs_clear` and call `terminal.clear()` when true
   ```rust
   if self.state.needs_clear {
       terminal.clear()?;
       self.state.needs_clear = false;
   }
   ```

3. **app.rs** `ServerMessage::Attached` handler: Set `needs_clear = true`

4. **app.rs** `ServerMessage::StateSnapshot` handler: Set `needs_clear = true`

5. **app.rs** `ClientCommand::Redraw` handler: Use `needs_clear` instead of `needs_redraw`

6. **app.rs** `ServerMessage::SessionEnded` handler: Set `needs_clear = true`

7. **app.rs** `InputAction::Detach` handler: Set `needs_clear = true`

8. **app.rs** "panes empty" handler (PaneClosed): Set `needs_clear = true`

9. **app.rs** `ClientCommand::ListSessions` handler: Set `needs_clear = true`

## Testing

- Build: `cargo build --package fugue-client` - PASSED
- Tests: `cargo test --package fugue-client` - 358 tests passed

Manual testing: Switch between sessions and verify clean rendering without artifacts.

## Files Modified

- `fugue-client/src/ui/state.rs`
- `fugue-client/src/ui/app.rs`
