# FEAT-062: Mirror Pane (Picture-in-Picture View)

**Priority**: P3
**Component**: ccmux-server, ccmux-client
**Type**: new_feature
**Estimated Effort**: medium
**Business Value**: high

## Overview

A pane that renders another pane's output in real-time, allowing visibility into other sessions without switching context. This enables "plate spinning" visibility - monitor multiple agents simultaneously without losing your place in your primary workflow.

## Use Case

**Scenario**: Orchestrator session on left, mirror pane on right showing worker output.

When working with multi-agent setups, users need to monitor what workers are doing while maintaining focus on the orchestrator session. Currently, this requires constantly switching between panes or sessions, losing context and scroll position. A mirror pane solves this by providing a read-only view of another pane's output.

## Proposed MCP Interface

```
ccmux_mirror_pane(source_pane_id, target_pane_id?)
```

**Parameters**:
- `source_pane_id` (required): The pane ID to mirror
- `target_pane_id` (optional): If provided, convert existing pane to mirror; if not, create new split that mirrors source

**Returns**:
- `mirror_pane_id`: The ID of the newly created or converted mirror pane
- `source_pane_id`: Echo of the source being mirrored

**Example Usage**:

```json
// Create new mirror pane (creates split)
{"tool": "ccmux_mirror_pane", "arguments": {"source_pane_id": "pane-abc123"}}

// Convert existing pane to mirror
{"tool": "ccmux_mirror_pane", "arguments": {"source_pane_id": "pane-abc123", "target_pane_id": "pane-def456"}}
```

## CLI Equivalent

```bash
ccmux mirror <source_pane_id>
```

**Options**:
- `--target <pane_id>`: Convert existing pane to mirror instead of creating new split
- `--direction <vertical|horizontal>`: Split direction when creating new pane (default: horizontal)

## Behavior

### Core Behavior

1. **Read-only view**: Mirror pane displays source pane output but does not accept input
2. **Real-time updates**: As source pane produces output, mirror updates immediately
3. **Independent scrollback**: User can scroll back in mirror without affecting source's scroll position
4. **Visual indicator**: Clear indication that pane is a mirror (not interactive)
   - Different border color (e.g., cyan or dim)
   - "[MIRROR]" in pane title/status
   - Source pane ID shown in status

### User Interaction

| Input | Behavior |
|-------|----------|
| Arrow keys / Page Up/Down | Scroll mirror's view (independent of source) |
| `q` | Close/unlink the mirror pane |
| `Escape` | Close/unlink the mirror pane |
| Other keys | Ignored (read-only) |
| Mouse scroll | Scroll mirror's view |
| Mouse click | Focus the mirror pane (for scrolling) |

### State Synchronization

- Mirror subscribes to source pane's output events
- Maintains its own terminal parser/buffer for independent scrollback
- No PTY needed - reads from source's buffer directly
- When source pane closes, mirror pane shows "[Source closed]" message

## Implementation Considerations

### Server-Side Changes (ccmux-server)

1. **Mirror Registry**: Track mirror relationships
   - Map of `mirror_pane_id -> source_pane_id`
   - Cleanup when source closes (notify mirrors, update state)
   - Cleanup when mirror closes (remove from registry)

2. **Output Broadcasting**: When pane produces output
   - Current: Send to connected clients viewing that pane
   - New: Also send to any mirrors of that pane

3. **New Pane Type**: Add `PaneType::Mirror { source_id }` variant
   - Mirrors don't spawn PTY
   - Mirrors have their own terminal buffer (for independent scrollback)
   - Mirrors receive output events from source

### Client-Side Changes (ccmux-client)

1. **Mirror Pane Rendering**:
   - Same rendering as regular pane
   - Different border color/style
   - "[MIRROR: pane-xxx]" in title

2. **Input Handling**:
   - If focused pane is mirror: only handle scroll/quit
   - Do not forward input to server for PTY

3. **Status Updates**:
   - Show mirror indicator in status bar
   - Show source pane identifier

### Protocol Changes (ccmux-protocol)

1. **New Messages**:
   ```rust
   // Request to create mirror
   CreateMirror { source_pane_id: PaneId, target_pane_id: Option<PaneId> }

   // Response with mirror info
   MirrorCreated { mirror_pane_id: PaneId, source_pane_id: PaneId }

   // Notification when source closes
   MirrorSourceClosed { mirror_pane_id: PaneId }
   ```

2. **Pane Info Extension**:
   - Add `is_mirror: bool` and `mirror_source: Option<PaneId>` to pane info

## Nice to Have (v1.2+)

These are out of scope for initial implementation but worth noting for future:

- **Mirror entire session**: Cycle through panes in a session
- **Multiple mirrors**: Multiple mirrors of the same source pane
- **Filter/highlight patterns**: Show only matching lines in mirrored output
- **Auto-scroll lock**: Option to always follow new output vs. stay at scroll position
- **Split view**: Show multiple sources in a grid within one pane

## Acceptance Criteria

- [ ] `ccmux_mirror_pane` MCP tool creates mirror pane
- [ ] CLI `ccmux mirror <pane_id>` creates mirror pane
- [ ] Mirror displays source pane output in real-time
- [ ] Mirror has independent scrollback from source
- [ ] Mirror shows visual indicator (border color, title)
- [ ] `q` or `Escape` closes mirror pane
- [ ] Arrow keys scroll mirror view without affecting source
- [ ] Source pane closure is handled gracefully
- [ ] Multiple mirrors of different sources work simultaneously
- [ ] Server tracks mirror relationships for cleanup

## Testing Approach

### Unit Tests

- Mirror registry add/remove operations
- Mirror pane creation logic
- Output forwarding to mirrors

### Integration Tests

- Create mirror via MCP tool
- Create mirror via CLI
- Verify output appears in mirror
- Verify independent scrollback
- Source close handling
- Multiple mirrors simultaneously

### Manual Testing

1. Create a pane running a process with continuous output (e.g., `watch date`)
2. Create mirror of that pane
3. Verify output appears in both
4. Scroll in mirror - verify source is unaffected
5. Close source - verify mirror shows closure message
6. Test with actual Claude sessions (orchestrator + worker)

## Location

Files to create/modify:

| File | Changes |
|------|---------|
| `ccmux-server/src/session/pane.rs` | Add `PaneType::Mirror` variant |
| `ccmux-server/src/session/mirror.rs` | New: Mirror registry and logic |
| `ccmux-server/src/mcp/tools.rs` | Add `ccmux_mirror_pane` tool |
| `ccmux-protocol/src/message.rs` | Add mirror-related messages |
| `ccmux-client/src/ui/pane.rs` | Mirror rendering logic |
| `ccmux-client/src/input/handler.rs` | Handle mirror-specific input |
| `ccmux/src/cli/commands.rs` | Add `mirror` subcommand |

## Dependencies

None - builds on existing pane infrastructure.

## Notes

- Similar concept to tmux's `link-window` but at the pane level with read-only semantics
- Consider whether mirrors should persist across server restarts (probably not for v1)
- Performance consideration: Large buffers duplicated for each mirror
  - Optimization: Share buffer, only track scroll position per-mirror
