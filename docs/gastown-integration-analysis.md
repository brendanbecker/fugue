# fugue + Gas Town Integration Analysis

## Executive Summary

fugue provides a viable replacement for tmux as Gas Town's session layer, offering:

1. **Direct MCP Interface** - Agents can call tools directly without shelling out
2. **Claude State Detection** - Per-pane cognitive state tracking (Thinking, Coding, ToolUse, etc.)
3. **Built-in Orchestration Protocol** - Already has StatusUpdate, TaskAssignment, Broadcast primitives
4. **Session Persistence** - WAL + checkpoints with scrollback preservation

The primary gap is exposing a few missing MCP tools (`fugue_kill_session`, `fugue_set_environment`) and adding a CLI compatibility layer for incremental migration.

---

## 1. Compatibility Matrix

### Session Lifecycle

| Gas Town Usage | tmux Command | fugue Equivalent | Status | File:Line |
|----------------|--------------|------------------|--------|-----------|
| Create detached session | `new-session -d -s <name>` | `fugue_create_session` | **IMPLEMENTED** | tools.rs:158-169 |
| Create session with command | `new-session -d -s <name> <cmd>` | `fugue_create_session` (needs command param) | **PARTIAL** | CreateSession has optional command in protocol |
| Kill session | `kill-session -t <name>` | `DestroySession` in protocol | **NEEDS MCP TOOL** | messages.rs:149-150 |
| Check session exists | `has-session -t =<name>` | `fugue_list_sessions` + filter | **EQUIVALENT** | tools.rs:137-143 |
| Kill server | `kill-server` | Not needed (fugue daemon stays running) | **N/A** | |
| Rename session | `rename-session -t <old> <new>` | `fugue_rename_session` | **IMPLEMENTED** | tools.rs:192-208 |
| Attach session | `attach-session -t <session>` | `AttachSession` in protocol | **NEEDS MCP TOOL** | messages.rs:88-89 |
| Switch client | `switch-client -t <session>` | `fugue_focus_pane` (pane-level) | **EQUIVALENT** | tools.rs:123-135 |

### Message Sending (The Nudge Pattern)

| Gas Town Usage | tmux Command | fugue Equivalent | Status | Notes |
|----------------|--------------|------------------|--------|-------|
| NudgeSession | `send-keys -t <sess> -l <text>` + `send-keys Enter` | `fugue_send_input(submit: true)` | **IMPLEMENTED** | 500ms debounce not needed - direct IPC |
| NudgePane | `send-keys -t <pane> -l <text>` + Enter | `fugue_send_input(pane_id, submit: true)` | **IMPLEMENTED** | |
| SendKeysRaw | `send-keys -t <sess> <keys>` | `fugue_send_input(submit: false)` | **IMPLEMENTED** | |
| SendKeysReplace | `C-u` + `send-keys` | `fugue_send_input` (need Ctrl-U sequence) | **MANUAL** | Send `\x15` before text |
| Accept bypass dialog | `send-keys Down Enter` | `fugue_send_input` | **MANUAL** | Send key sequences |

**Key Insight**: fugue eliminates the need for debounce hacks. The 500ms debounce in Gas Town exists because `tmux send-keys` is asynchronous and unreliable for large pastes. fugue uses direct PTY writes via IPC - no debounce needed.

### Window/Pane Management

| Gas Town Usage | tmux Command | fugue Equivalent | Status | File:Line |
|----------------|--------------|------------------|--------|-----------|
| Create window | `new-window -t <session>` | `fugue_create_window` | **IMPLEMENTED** | tools.rs:171-190 |
| Create pane (split) | `split-window -t <window>` | `fugue_create_pane` | **IMPLEMENTED** | tools.rs:42-70 |
| Select window | `select-window -t <session>:<idx>` | `fugue_select_window` (bridge only) | **PARTIAL** | bridge.rs:738-751 |
| Respawn pane | `respawn-pane -k -t <pane> <cmd>` | Close + create | **MANUAL** | Compose close + create |
| Clear history | `clear-history -t <pane>` | Not implemented | **NEEDS WORK** | |
| Close pane | `kill-pane` (implied) | `fugue_close_pane` | **IMPLEMENTED** | tools.rs:109-121 |
| Focus pane | `select-pane -t <pane>` | `fugue_focus_pane` | **IMPLEMENTED** | tools.rs:123-135 |

### Information Gathering

| Gas Town Usage | tmux Command | fugue Equivalent | Status | File:Line |
|----------------|--------------|------------------|--------|-----------|
| List sessions | `list-sessions -F "#{...}"` | `fugue_list_sessions` | **IMPLEMENTED** | tools.rs:137-143 |
| List panes | `list-panes -t <sess> -F "#{...}"` | `fugue_list_panes` | **IMPLEMENTED** | tools.rs:11-22 |
| Capture pane | `capture-pane -p -t <sess> -S -N` | `fugue_read_pane(lines: N)` | **IMPLEMENTED** | tools.rs:24-40 |
| Get pane command | `list-panes -F "#{pane_current_command}"` | `fugue_get_status` | **IMPLEMENTED** | tools.rs:95-107 |
| Get pane ID | `list-panes -F "#{pane_id}"` | All panes return UUIDs | **IMPLEMENTED** | |
| Get pane workdir | `list-panes -F "#{pane_current_path}"` | `fugue_get_status.cwd` | **IMPLEMENTED** | messages.rs:326 |
| Find session by workdir | Custom logic | Not implemented | **NEEDS WORK** | Could add as MCP tool |

### Agent Detection

| Gas Town Usage | tmux Command | fugue Equivalent | Status | Notes |
|----------------|--------------|------------------|--------|-------|
| IsAgentRunning | `list-panes -F "#{pane_current_command}"` + parsing | `fugue_get_status.state` | **BETTER** | Native Claude detection |
| IsClaudeRunning | Parse for "node/claude" pattern | `PaneState::Claude(...)` | **BETTER** | Automatic, not polling |
| WaitForCommand | Poll `list-panes` in loop | `ClaudeStateChanged` events | **BETTER** | Event-driven |
| WaitForShellReady | Poll for shell prompt | Shell detection possible | **EQUIVALENT** | |

### Environment Variables

| Gas Town Usage | tmux Command | fugue Equivalent | Status | Notes |
|----------------|--------------|------------------|--------|-------|
| Set environment | `set-environment -t <sess> KEY VAL` | Not implemented | **NEEDS MCP TOOL** | Important for GT_RIG, BEADS_* |
| Get environment | `show-environment -t <sess> KEY` | Not implemented | **NEEDS MCP TOOL** | |

### Status Bar & Configuration

| Gas Town Usage | tmux Command | fugue Equivalent | Status | Notes |
|----------------|--------------|------------------|--------|-------|
| Status-left | `set-option -t <sess> status-left ...` | TUI renders its own status | **DIFFERENT** | fugue client handles this |
| Status-right | `set-option status-right "#(gt status-line)"` | fugue status bar built-in | **DIFFERENT** | |
| Theme/colors | `set-option status-style "bg=..."` | TUI theming | **DIFFERENT** | |
| Mouse mode | `set-option mouse on` | Always enabled in TUI | **BUILT-IN** | |

### Bindings & Hooks

| Gas Town Usage | tmux Command | fugue Equivalent | Status | Notes |
|----------------|--------------|------------------|--------|-------|
| Cycle bindings | `bind-key -T prefix n/p ...` | Client-side keybindings | **CLIENT** | |
| Feed binding | `bind-key -T prefix a ...` | Client-side | **CLIENT** | |
| Mail click binding | `bind-key MouseDown1StatusRight ...` | Client-side | **CLIENT** | |
| Pane died hook | `set-hook pane-died 'run-shell "gt log..."'` | `PaneClosed` event with exit_code | **EVENT** | messages.rs:259-262 |

---

## 2. Gaps to Address

### High Priority (Required for Basic Integration)

| Gap | Impact | Implementation Effort | Location |
|-----|--------|----------------------|----------|
| **fugue_kill_session** | Can't stop workers | Low - protocol exists, just add MCP tool | Add to tools.rs |
| **fugue_set_environment** | Can't set GT_RIG, BEADS_* | Medium - need to add to session struct | session.rs + new tool |
| **fugue_get_environment** | Can't read env vars | Medium - paired with set | session.rs + new tool |
| **CLI wrapper** | Incremental migration | Medium - add `fugue send-keys` compat | New binary |

### Medium Priority (Enhanced Integration)

| Gap | Impact | Implementation Effort | Notes |
|-----|--------|----------------------|-------|
| **fugue_clear_history** | Can't clear scrollback on respawn | Low - already tracked, just truncate | |
| **fugue_find_session_by_cwd** | Session discovery by workdir | Medium - need index | Useful for Gas Town patterns |
| **Session metadata** | Store agent identity per-session | Medium - extend SessionInfo | For GT_RIG, role, etc. |
| **fugue_respawn_pane** | Hot reload pattern | Low - compose close + create | Could be convenience tool |

### Low Priority (Nice to Have)

| Gap | Impact | Notes |
|-----|--------|-------|
| `fugue_display_message` | Status notifications | Could use orchestration instead |
| `fugue_display_popup` | Mail viewer | Different UI paradigm |
| Custom key bindings | User convenience | fugue client handles |

---

## 3. Novel Capabilities (fugue Advantages)

### 3.1 Claude Cognitive State Detection

**File**: `fugue-server/src/claude/detector.rs`

Gas Town's Witness patrol currently does blind polling - "is there output?" followed by arbitrary nudges. fugue detects Claude's actual cognitive state:

```rust
pub enum ClaudeActivity {
    Idle,                    // Waiting for input (safe to nudge)
    Thinking,                // Processing (DO NOT nudge)
    Coding,                  // Writing code (DO NOT nudge)
    ToolUse,                 // Executing tools (DO NOT nudge)
    AwaitingConfirmation,    // Needs user response (SHOULD nudge)
}
```

**Integration Value**:
- Witness can query `fugue_get_status(polecat-7).claude_state.activity`
- "polecat-7 thinking for 45s" = fine, let it work
- "polecat-3 AwaitingConfirmation for 2 min" = stuck, needs nudge
- Eliminates blind nudging that disrupts working agents

### 3.2 Built-in Orchestration Protocol

**File**: `fugue-protocol/src/messages.rs:8-67`

fugue already has primitives matching Gas Town's messaging needs:

```rust
pub enum OrchestrationMessage {
    StatusUpdate { session_id, status: WorkerStatus, message },
    TaskAssignment { task_id, description, files },
    TaskComplete { task_id, success, summary },
    HelpRequest { session_id, context },
    Broadcast { from_session_id, message },
    SyncRequest,
}

pub enum WorkerStatus {
    Idle, Working, WaitingForInput, Blocked, Complete, Error
}

pub enum OrchestrationTarget {
    Orchestrator,           // Send to mayor
    Session(Uuid),          // Send to specific worker
    Broadcast,              // Send to all in same repo
    Worktree(String),       // Send to worktree cluster
}
```

**Integration Value**:
- Mayor can `SendOrchestration(Session(polecat-3), TaskAssignment{...})`
- Polecats can `SendOrchestration(Orchestrator, TaskComplete{...})`
- Witness broadcasts `SyncRequest` to get all statuses
- **Flattens the messaging hierarchy** - no more `gt nudge` relay chain

### 3.3 Event-Driven State Changes

Instead of polling `tmux capture-pane` and parsing output, fugue pushes:

```rust
ServerMessage::ClaudeStateChanged { pane_id, state: ClaudeState }
ServerMessage::PaneClosed { pane_id, exit_code }
ServerMessage::OrchestrationReceived { from_session_id, message }
```

**Integration Value**:
- Witness subscribes to events rather than polling
- Instant notification when Claude finishes
- Crash detection via `PaneClosed` with exit code

### 3.4 Session Persistence with Identity

**File**: `fugue-server/src/persistence/`

fugue persists:
- Session hierarchy (sessions → windows → panes) with UUIDs
- Scrollback content (configurable lines, default 1000)
- Claude session IDs (for `gt seance` resume)
- Pane state (including ClaudeState)

**Integration Value**:
- `gt seance` can find Claude session IDs directly from fugue state
- Workers survive daemon restarts with full context
- No more "orphan session" detection - fugue tracks everything

### 3.5 No Debounce Required

Gas Town's `NudgeSession` has these hacks (from `internal/tmux/tmux.go:286-308`):
- Send text with `-l` literal mode
- Sleep 500ms (tested, required)
- Send Enter with 3-retry loop (200ms between attempts, 3 max)

fugue doesn't need any of this:
- Direct IPC to daemon
- Daemon writes directly to PTY
- Atomic, synchronous operation

---

## 4. Recommended Migration Path

### Phase 1: CLI Compatibility Layer

Create `fugue-compat` binary that mimics tmux commands:

```bash
# Gas Town calls:
fugue-compat new-session -d -s gt-alpha-toast -c /repo claude

# Translates to:
fugue_create_session(name: "gt-alpha-toast")
# + fugue_send_input for initial command (or add command param)

# Gas Town calls:
fugue-compat send-keys -t gt-alpha-toast -l "do your job"
fugue-compat send-keys -t gt-alpha-toast Enter

# Translates to:
fugue_send_input(pane_id: resolve("gt-alpha-toast"), input: "do your job", submit: true)
```

**One-line swap in Gas Town**: Change `exec.Command("tmux", args...)` to `exec.Command("fugue-compat", args...)` in `internal/tmux/tmux.go:35`.

### Phase 2: Native MCP Integration

Gas Town's `gt` binary embeds an MCP client:

```go
// internal/fugue/client.go
type Client struct {
    conn *McpConnection
}

func (c *Client) NudgeSession(session, message string) error {
    paneId := c.resolvePaneId(session)
    return c.conn.Call("fugue_send_input", map[string]any{
        "pane_id": paneId,
        "input": message,
        "submit": true,
    })
}

func (c *Client) GetClaudeState(session string) (*ClaudeState, error) {
    paneId := c.resolvePaneId(session)
    result := c.conn.Call("fugue_get_status", map[string]any{
        "pane_id": paneId,
    })
    return parseClaudeState(result)
}
```

No shell exec. Direct protocol.

### Phase 3: Agent Self-Service

Workers get MCP access to fugue tools:

```
# Mayor spawns polecats directly
fugue_create_session(name: "gt-alpha-new-polecat")

# Polecats check each other
fugue_get_status(pane_id: "polecat-7").claude_state

# Witness queries all states
for pane in fugue_list_panes(session: "gt-alpha-*"):
    if pane.claude_state.activity == "AwaitingConfirmation":
        fugue_send_input(pane.id, "continue", submit: true)

# Agents send orchestration messages
SendOrchestration(Orchestrator, TaskComplete{...})
```

The daemon hierarchy (Daemon → Deacon → Witness → Polecats) becomes optional. Agents are peers on a shared messaging bus.

---

## 5. Minimal PoC Specification

### Goal
Prove fugue can replace tmux for `gt nudge` without behavioral changes.

### Scope
1. Run Gas Town's mayor with fugue instead of tmux
2. Nudge a polecat and verify message delivery
3. Check Claude state via MCP instead of parsing output

### Prerequisites
1. Add `fugue_kill_session` MCP tool
2. Add `fugue_set_environment` MCP tool
3. Create minimal CLI wrapper for `send-keys`

### Implementation

**Step 1**: Add missing MCP tools

```rust
// tools.rs - add:
Tool {
    name: "fugue_kill_session",
    description: "Kill/destroy a session",
    input_schema: json!({
        "type": "object",
        "properties": {
            "session": { "type": "string", "description": "Session UUID or name" }
        },
        "required": ["session"]
    }),
}

Tool {
    name: "fugue_set_environment",
    description: "Set environment variable for a session",
    input_schema: json!({
        "type": "object",
        "properties": {
            "session": { "type": "string" },
            "key": { "type": "string" },
            "value": { "type": "string" }
        },
        "required": ["session", "key", "value"]
    }),
}
```

**Step 2**: CLI compatibility wrapper

```rust
// fugue-compat/src/main.rs
fn main() {
    let args: Vec<String> = std::env::args().collect();

    match args.get(1).map(String::as_str) {
        Some("new-session") => handle_new_session(&args[2..]),
        Some("send-keys") => handle_send_keys(&args[2..]),
        Some("kill-session") => handle_kill_session(&args[2..]),
        Some("has-session") => handle_has_session(&args[2..]),
        Some("list-sessions") => handle_list_sessions(&args[2..]),
        Some("capture-pane") => handle_capture_pane(&args[2..]),
        _ => { eprintln!("Unknown command"); std::process::exit(1); }
    }
}

fn handle_send_keys(args: &[String]) {
    // Parse -t <target>, -l flag, remaining text
    // Connect to fugue daemon
    // Call fugue_send_input
}
```

**Step 3**: Gas Town integration test

```go
// internal/integration/fugue_test.go
func TestCcmuxNudge(t *testing.T) {
    // Start fugue daemon
    // Create session via fugue_create_session
    // Set GT_RIG via fugue_set_environment
    // Start Claude in pane
    // Send nudge via fugue_send_input
    // Verify response in fugue_read_pane
    // Check ClaudeState via fugue_get_status
    // Kill session via fugue_kill_session
}
```

### Success Criteria
1. `gt nudge mayor "status"` works with fugue backend
2. Claude state detection returns accurate `ClaudeActivity`
3. Session creation/destruction matches tmux semantics
4. No debounce needed - messages arrive immediately

---

## 6. Key File References

### Gas Town

| Component | File | Key Lines |
|-----------|------|-----------|
| Core tmux wrapper | `internal/tmux/tmux.go` | 1-981 |
| NudgeSession (canonical) | `internal/tmux/tmux.go` | 286-334 |
| Session identity | `internal/session/identity.go` | 21-150 |
| Polecat session manager | `internal/polecat/session_manager.go` | 34-243 |
| gt nudge command | `internal/cmd/nudge.go` | 27-494 |
| gt seance command | `internal/cmd/seance.go` | 29-290 |
| Constants (debounce) | `internal/constants/constants.go` | 19-29 |

### fugue

| Component | File | Key Lines |
|-----------|------|-----------|
| MCP tool definitions | `fugue-server/src/mcp/tools.rs` | 1-268 |
| MCP bridge | `fugue-server/src/mcp/bridge.rs` | 1-899 |
| MCP handlers | `fugue-server/src/mcp/handlers.rs` | 1-1090 |
| Protocol messages | `fugue-protocol/src/messages.rs` | 1-400 |
| Orchestration types | `fugue-protocol/src/messages.rs` | 8-67 |
| Claude detector | `fugue-server/src/claude/detector.rs` | 1-450+ |
| Session manager | `fugue-server/src/session/manager.rs` | 1-200+ |
| Persistence | `fugue-server/src/persistence/mod.rs` | 1-200+ |

---

## 7. Conclusion

fugue is not just a tmux replacement - it's the "Orchestrator API Surface" that Yegge explicitly called for. The integration is viable with modest effort:

| Phase | Effort | Value |
|-------|--------|-------|
| CLI compat layer | 1-2 days | Drop-in replacement, zero Gas Town changes |
| Missing MCP tools | 1 day | Full feature parity |
| Native MCP integration | 3-5 days | Eliminate shell exec, direct protocol |
| Agent self-service | 2-3 days | Flatten hierarchy, enable peer messaging |

The novel capabilities (Claude state detection, orchestration protocol, event-driven updates) provide immediate value to Gas Town's current pain points around blind nudging and worker state visibility.
