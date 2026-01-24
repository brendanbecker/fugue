# Troubleshooting

Common issues and solutions for fugue.

## Terminal Issues

### Terminal displays garbage after crash

**Symptom**: Terminal shows corrupted characters, wrong colors, or doesn't respond properly after fugue exits unexpectedly.

**Cause**: fugue puts the terminal in raw mode. If it exits without restoring terminal state (e.g., `kill -9`), the terminal remains corrupted.

**Solution**:
```bash
reset
```

Or if `reset` doesn't work:
```bash
stty sane
```

### Can't type after large output

**Symptom**: After a command produces lots of output, keyboard input stops working.

**Cause**: This was a bug (BUG-014) that has been fixed. The event loop could get starved by output processing.

**Solution**: Update to the latest version. If you're on the latest version and still experiencing this, please file a bug.

## Mouse and Selection

### Can't select text with mouse

**Symptom**: Clicking and dragging doesn't select text for copy/paste.

**Cause**: fugue captures mouse events for scrollback navigation.

**Solution**: Hold `Shift` while clicking/dragging to bypass fugue and use native terminal selection.

### Mouse scroll not working

**Symptom**: Mouse wheel doesn't scroll through history.

**Cause**: This was a bug (BUG-013) that has been fixed.

**Solution**: Update to the latest version.

## Session Issues

### Old sessions appearing that shouldn't exist

**Symptom**: Session list shows sessions from previous versions or sessions that should have been deleted.

**Cause**: Legacy state from before certain bug fixes (especially BUG-004) may persist.

**Solution**:
```bash
rm -rf ~/.local/share/.fugue/state/
```

Then restart fugue. This clears all saved state.

### Session hangs on attach

**Symptom**: Selecting a session causes fugue to hang or become unresponsive.

**Cause**: Zombie pane from a dead PTY process (fixed in BUG-004).

**Solution**: Update to latest version and clear state if needed:
```bash
rm -rf ~/.local/share/.fugue/state/
```

### Session not persisting after server restart

**Symptom**: Sessions disappear when the server restarts.

**Cause**: Persistence might not be working correctly or state directory has permission issues.

**Solution**: Check that `~/.local/share/.fugue/state/` exists and is writable:
```bash
ls -la ~/.local/share/.fugue/
```

## MCP Issues

### Claude can't connect to fugue

**Symptom**: MCP tools fail with connection errors.

**Cause**: The MCP bridge isn't running or path is wrong.

**Solution**:
1. Verify the path in `~/.claude/mcp.json` is absolute and correct
2. Ensure fugue-server is running: `pgrep fugue-server`
3. Check the socket exists: `ls ~/.fugue/fugue.sock`

### MCP tools not appearing in Claude

**Symptom**: Claude doesn't have fugue tools available.

**Cause**: MCP configuration not loaded.

**Solution**:
1. Restart Claude Code completely
2. Verify `~/.claude/mcp.json` syntax is valid JSON
3. Check Claude's MCP log for errors

### fugue_send_input not working

**Symptom**: `fugue_send_input` tool doesn't send text to pane.

**Cause**: This was a bug (BUG-017) that has been fixed.

**Solution**: Update to latest version. Use `\n` in the input string to send Enter.

## Key Binding Issues

### Shift+Tab not working

**Symptom**: Shift+Tab doesn't register or does the wrong thing.

**Cause**: This was a bug (BUG-007) that has been fixed.

**Solution**: Update to latest version.

### Prefix key not responding

**Symptom**: `Ctrl+b` doesn't enter prefix mode.

**Cause**: Could be terminal emulator intercepting the key, or fugue in an unusual state.

**Solution**:
1. Try a different terminal emulator to test
2. Check if your terminal has `Ctrl+b` mapped to something else
3. Detach and reattach: close terminal, run `fugue` again

## Performance Issues

### High memory usage

**Symptom**: fugue using more memory than expected.

**Cause**: Large scrollback buffers or many panes.

**Solution**: Scrollback is limited to 1000 lines by default. If you have many panes open, memory usage will increase proportionally. Close unused panes.

### Slow rendering

**Symptom**: Terminal feels sluggish or laggy.

**Cause**: Very large output or many split panes.

**Solution**:
1. Reduce number of visible panes
2. Clear scrollback in busy panes
3. Check if a pane is producing continuous output

## File Locations

| Path | Purpose |
|------|---------|
| `~/.config/fugue/config.toml` | Configuration file |
| `~/.local/share/.fugue/state/` | Session persistence (WAL + checkpoints) |
| `~/.fugue/fugue.sock` | Unix socket for client-server communication |
| `~/.fugue/claude-configs/` | Isolated Claude config directories per pane |

## Getting Help

If your issue isn't covered here:

1. Check if it's a known issue in the [README](../README.md#known-issues)
2. Search existing issues in the repository
3. File a new issue with:
   - fugue version (`git rev-parse HEAD`)
   - Terminal emulator and version
   - Steps to reproduce
   - Expected vs actual behavior
