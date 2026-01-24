# FEAT-032: Integrated MCP Server

**Priority**: P1 (important for Claude integration UX)
**Component**: fugue-server
**Type**: enhancement
**Estimated Effort**: large
**Business Value**: high
**Status**: new

## Overview

Integrate the MCP server so it connects to the main fugue-server daemon instead of running as a standalone process with its own session state. This enables Claude to control the same sessions the user is actively interacting with through the TUI client.

## Problem Statement

### Current Architecture

```
┌─────────────────────┐      ┌─────────────────────┐
│  fugue (TUI client) │      │  Claude Code        │
└──────────┬──────────┘      └──────────┬──────────┘
           │                            │
           │ Unix Socket                │ stdio
           ▼                            ▼
┌─────────────────────┐      ┌─────────────────────┐
│  fugue-server       │      │  fugue-server       │
│  (daemon mode)      │      │  (mcp-server mode)  │
│                     │      │                     │
│  Sessions A, B, C   │      │  Sessions X, Y, Z   │
└─────────────────────┘      └─────────────────────┘
        SEPARATE                   SEPARATE
        STATE                      STATE
```

**Problems**:
1. **Isolated Session State**: The MCP server runs as a separate process with its own sessions. Claude cannot see or interact with sessions the user created in the TUI.
2. **No Shared Context**: User creates a session in TUI, asks Claude "look at my session" - Claude sees nothing because it's talking to a different server instance.
3. **Resource Duplication**: Two server processes running, each with their own PTYs, parsers, and state.
4. **Confusing UX**: User expectations don't match reality - they assume Claude can see their terminals.

### Desired Architecture

```
┌─────────────────────┐      ┌─────────────────────┐
│  fugue (TUI client) │      │  Claude Code        │
└──────────┬──────────┘      └──────────┬──────────┘
           │                            │
           │ Unix Socket                │ stdio
           ▼                            ▼
           │              ┌─────────────────────┐
           │              │  MCP Bridge         │
           │              │  (stdio <-> socket) │
           │              └──────────┬──────────┘
           │                         │
           │         Unix Socket     │
           ▼                         ▼
      ┌────────────────────────────────────┐
      │  fugue-server (single daemon)      │
      │                                    │
      │  Sessions A, B, C (shared state)   │
      └────────────────────────────────────┘
```

**Benefits**:
1. **Shared Sessions**: Claude sees the same sessions as the user
2. **Single Source of Truth**: One daemon manages all state
3. **Unified UX**: "Claude, split my pane" works on the pane user is looking at
4. **Resource Efficiency**: Single server process

## Requirements

### Must Have

#### 1. Main Server Always Runs as Daemon

The primary fugue-server should always be the authoritative daemon. No changes needed here - this is the current behavior when running `fugue-server` or when auto-started by the client.

#### 2. MCP Server Connects to Daemon

Instead of running standalone, MCP mode should:
- Connect to the existing fugue-server daemon via Unix socket
- Act as a protocol translator (MCP JSON-RPC over stdio <-> fugue IPC over socket)
- Forward all tool calls to the daemon
- Return daemon responses to Claude

#### 3. Transparent Tool Operations

All existing MCP tools should work identically:
- `fugue_list_sessions` - Lists sessions from daemon
- `fugue_list_panes` - Lists panes from daemon
- `fugue_create_pane` - Creates pane in daemon (user sees it in TUI)
- `fugue_send_input` - Sends input to daemon's PTY
- `fugue_read_pane` - Reads from daemon's scrollback
- etc.

#### 4. Auto-Start Daemon if Needed

If MCP server starts and no daemon is running:
- Start the daemon automatically (or fail with clear error)
- Connect once daemon is ready

### Nice to Have

#### 5. MCP as Part of Daemon Process

Alternative architecture: Run MCP server in the same process as daemon:
- Daemon listens on both Unix socket AND launches stdio handler for Claude
- Avoids extra process, but complicates process model
- May complicate Claude Code's stdio expectations

#### 6. Client Identification

Daemon should distinguish between:
- TUI clients (regular users)
- MCP clients (Claude via MCP bridge)
- Could enable features like "notify TUI when Claude is interacting"

## Technical Approach

### Option A: MCP Bridge Process (Recommended)

Create a thin MCP bridge that:
1. Speaks MCP JSON-RPC on stdio (to Claude)
2. Speaks fugue IPC protocol on Unix socket (to daemon)
3. Translates between the two

**Pros**:
- Clean separation of concerns
- Minimal changes to existing code
- Easy to debug/test independently
- MCP protocol handling stays isolated

**Cons**:
- Extra process in the chain

**Implementation**:
```
fugue-server mcp-server  (old: standalone)
fugue-server mcp-bridge  (new: connects to daemon)
```

The `mcp-bridge` subcommand would:
1. Connect to `~/.local/share/fugue/fugue.sock`
2. Set up stdio MCP server
3. For each tool call, translate to IPC message, send to daemon, get response, translate back

### Option B: In-Process MCP Handler

Add MCP stdio handling directly in the daemon:
1. Daemon spawns a task for MCP when configured
2. Task handles stdio directly
3. Calls same internal handlers as IPC

**Pros**:
- Single process
- Direct access to state (no IPC translation)

**Cons**:
- Complicates daemon lifecycle
- stdio for MCP conflicts with daemon's detached mode
- Harder to test

### Recommended: Option A

Start with MCP Bridge approach. Can optimize to Option B later if needed.

## Implementation Tasks

### Section 1: MCP Bridge Command
- [ ] Add `mcp-bridge` subcommand to fugue-server CLI
- [ ] Implement daemon socket connection with retry logic
- [ ] Handle daemon not running (auto-start or error message)

### Section 2: Protocol Translation Layer
- [ ] Create MCP-to-IPC message translator
- [ ] Create IPC-to-MCP response translator
- [ ] Handle async operations (some IPC may be fire-and-forget)

### Section 3: Tool Forwarding
- [ ] Forward `fugue_list_sessions` to daemon
- [ ] Forward `fugue_list_windows` to daemon
- [ ] Forward `fugue_list_panes` to daemon
- [ ] Forward `fugue_create_session` to daemon
- [ ] Forward `fugue_create_window` to daemon
- [ ] Forward `fugue_create_pane` to daemon
- [ ] Forward `fugue_close_pane` to daemon
- [ ] Forward `fugue_send_input` to daemon
- [ ] Forward `fugue_read_pane` to daemon
- [ ] Forward `fugue_get_status` to daemon
- [ ] Forward `fugue_focus_pane` to daemon

### Section 4: IPC Protocol Extensions
- [ ] Add IPC message types for any missing operations
- [ ] Ensure all MCP tool operations have IPC equivalents
- [ ] Handle session/window/pane listing over IPC

### Section 5: Error Handling
- [ ] Handle daemon disconnection gracefully
- [ ] Reconnect on transient failures
- [ ] Return MCP-formatted errors for all failure modes

### Section 6: Testing
- [ ] Test MCP bridge connects to running daemon
- [ ] Test tool operations work end-to-end
- [ ] Test error cases (no daemon, daemon crash)
- [ ] Test concurrent TUI and MCP operations

### Section 7: Documentation
- [ ] Update MCP configuration docs
- [ ] Document new `mcp-bridge` command
- [ ] Add troubleshooting section

## Acceptance Criteria

- [ ] Running `fugue-server mcp-bridge` connects to existing daemon
- [ ] All MCP tools work through the bridge
- [ ] User can create session in TUI, Claude can see it via MCP
- [ ] Claude can create pane via MCP, user sees it in TUI
- [ ] Claude can send input to user's visible panes
- [ ] Daemon crash is handled gracefully by bridge
- [ ] Clear error message if daemon not running and can't auto-start

## Affected Files

| File | Changes |
|------|---------|
| `fugue-server/src/main.rs` | Add `mcp-bridge` subcommand |
| `fugue-server/src/mcp/bridge.rs` | New - MCP bridge implementation |
| `fugue-server/src/mcp/mod.rs` | Export bridge module |
| `fugue-protocol/src/lib.rs` | May need new IPC message types |
| `docs/MCP.md` | Update configuration documentation |

## Migration Path

1. **Phase 1**: Implement `mcp-bridge` command alongside existing `mcp-server`
2. **Phase 2**: Update documentation to recommend `mcp-bridge`
3. **Phase 3**: Deprecate standalone `mcp-server` mode
4. **Phase 4**: Remove standalone mode (or keep for debugging)

## Dependencies

- **FEAT-018** (MCP Server): Existing MCP implementation to build on
- **FEAT-029** (MCP Natural Language Control): New tools that need forwarding

## Example Usage

### Claude Code MCP Configuration

```json
{
  "mcpServers": {
    "fugue": {
      "command": "fugue-server",
      "args": ["mcp-bridge"]
    }
  }
}
```

### User Workflow

1. User starts TUI: `fugue` (auto-starts daemon)
2. User creates session, opens panes, runs commands
3. User opens Claude Code in another terminal
4. Claude Code starts MCP: `fugue-server mcp-bridge` (connects to same daemon)
5. User asks Claude: "What's running in my main session?"
6. Claude calls `fugue_list_panes` - sees user's actual panes
7. User asks Claude: "Split this pane and run htop"
8. Claude calls `fugue_create_pane` + `fugue_send_input`
9. User sees new pane appear in their TUI with htop running

## Notes

- This is a significant architectural improvement for the Claude integration story
- Should be implemented after the core MCP tools are stable (FEAT-029)
- Consider security implications of allowing any process to connect to daemon
- May want to add client authentication in the future
