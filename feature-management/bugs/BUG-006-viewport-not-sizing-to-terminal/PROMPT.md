# BUG-006: Viewport not sizing to terminal dimensions

## Summary

When ccmux-client starts in a full-screen terminal, the viewport renders at approximately quarter-screen size (80x24). The bug only manifests when attaching to an existing session; it works correctly if the terminal is already at the expected size.

## Symptom Details

- Full-screen terminal: Viewport renders at ~80x24 (quarter-screen)
- Quarter-screen terminal: Works correctly
- Resizing the terminal AFTER attach works (triggers proper resize)
- Issue is the initial sizing, not dynamic resize

## Root Cause Analysis

### Investigation Findings

The root cause is a **chicken-and-egg problem** in the session creation and attachment flow:

#### 1. Pane Default Dimensions (Server-side)

In `ccmux-server/src/session/pane.rs` (lines 86-88):
```rust
impl Pane {
    pub fn with_scrollback(...) -> Self {
        Self {
            cols: 80,
            rows: 24,  // <-- HARDCODED DEFAULT
            ...
        }
    }
}
```

When a pane is created, it defaults to 80x24. This is correct behavior for a newly created pane since the server doesn't know the client's terminal size.

#### 2. Session Creation Flow (Server-side)

In `ccmux-server/src/handlers/session.rs` (lines 69-78):
```rust
// Spawn PTY for the default pane
let mut pty_config = if let Some(ref cmd) = self.config.general.default_command {
    PtyConfig::command(cmd).with_size(cols, rows)  // Uses pane's 80x24
} else {
    PtyConfig::shell().with_size(cols, rows)       // Uses pane's 80x24
};
```

The PTY is spawned with the pane's default dimensions (80x24), not the client's actual terminal size.

#### 3. Client Terminal Size Detection (Client-side)

In `ccmux-client/src/ui/app.rs` (lines 137-141):
```rust
pub async fn run(&mut self) -> Result<()> {
    let mut terminal = Terminal::new()?;
    self.terminal_size = terminal.size()?;  // Correctly detects terminal size
    ...
}
```

The client **does** correctly detect its terminal size at startup.

#### 4. AttachSession Response (Server-side)

In `ccmux-server/src/handlers/session.rs` (lines 150-178):
```rust
let panes: Vec<_> = session
    .windows()
    .flat_map(|w| w.panes().map(|p| p.to_info()))  // Returns 80x24
    .collect();

HandlerResult::Response(ServerMessage::Attached {
    session: session_info,
    windows,
    panes,  // All panes have 80x24 dimensions
})
```

The server sends pane info with the stored dimensions (80x24).

#### 5. Client Pane Manager Setup (Client-side)

In `ccmux-client/src/ui/app.rs` (lines 634-646):
```rust
ServerMessage::Attached { ... } => {
    // Create UI panes for all panes in the session
    for pane_info in self.panes.values() {
        self.pane_manager.add_pane(pane_info.id, pane_info.rows, pane_info.cols);
        // ^^^ Uses server's 80x24, NOT terminal_size
    }
}
```

**This is the bug location.** The client creates UI panes using the server's reported dimensions rather than the client's actual terminal size.

#### 6. Resize Only Sent for Active Pane (Client-side)

In `ccmux-client/src/ui/app.rs` (lines 201-222):
```rust
AppEvent::Resize { cols, rows } => {
    // Resize all UI panes
    for pane_id in self.pane_manager.pane_ids() {
        self.pane_manager.resize_pane(pane_id, pane_rows, pane_cols);
    }

    // Notify server of resize if attached
    if let Some(pane_id) = self.active_pane_id {
        self.connection
            .send(ClientMessage::Resize {
                pane_id,
                cols: pane_cols,
                rows: pane_rows,
            })
            .await?;
    }
}
```

The client sends resize messages when the terminal size changes, but **no resize is sent during initial attach**.

### The Bug Flow

1. Server creates session with pane at 80x24
2. PTY is spawned at 80x24
3. Client starts, detects terminal is 200x50
4. Client attaches to session
5. Server sends `Attached` with panes at 80x24
6. Client creates UI panes at 80x24 (ignoring its own terminal_size)
7. No resize message is sent because terminal didn't "change"
8. Viewport renders at 80x24 inside the 200x50 terminal

### Why Resize After Attach Works

If the user resizes their terminal after attaching, the `AppEvent::Resize` handler fires, which:
1. Updates UI pane dimensions
2. Sends `ClientMessage::Resize` to server
3. Server resizes the PTY

The terminal then displays correctly.

## Proposed Fix

### Option A: Client sends resize immediately after attach (Recommended)

In `ccmux-client/src/ui/app.rs`, after handling `ServerMessage::Attached`:

```rust
ServerMessage::Attached { session, windows, panes } => {
    // ... existing code ...

    // Create UI panes with CLIENT's terminal size, not server's
    let (term_cols, term_rows) = self.terminal_size;
    let pane_rows = term_rows.saturating_sub(3);  // Account for borders/status
    let pane_cols = term_cols.saturating_sub(2);

    for pane_info in self.panes.values() {
        self.pane_manager.add_pane(pane_info.id, pane_rows, pane_cols);
        // ... existing metadata setup ...
    }

    // Send resize to server for all panes
    for pane_id in self.pane_manager.pane_ids() {
        self.connection
            .send(ClientMessage::Resize {
                pane_id,
                cols: pane_cols,
                rows: pane_rows,
            })
            .await?;
    }
}
```

**Pros:**
- Simple fix
- Maintains server's PTY dimensions as authoritative
- Works for all clients, even with different terminal sizes

**Cons:**
- Brief flash of wrong size during attach
- Extra network traffic (resize messages)

### Option B: Server uses client's terminal size on attach

Modify `ClientMessage::AttachSession` to include the client's terminal dimensions:

```rust
ClientMessage::AttachSession {
    session_id: Uuid,
    terminal_cols: u16,
    terminal_rows: u16,
}
```

Server would then resize all panes before responding with `Attached`.

**Pros:**
- No flash of wrong size
- Server has accurate dimensions immediately

**Cons:**
- Protocol change required
- May cause issues with multiple clients of different sizes

### Option C: Server sends resize command as part of attach response

Add a new response type that includes resize instructions, or have server send resize after Attached.

## Recommended Implementation

**Use Option A** for the immediate fix as it requires no protocol changes and is the simplest solution.

## Files to Modify

1. `ccmux-client/src/ui/app.rs`
   - In `handle_server_message()`, `ServerMessage::Attached` handler
   - Create UI panes at terminal size, not server-reported size
   - Send resize messages to server after attach

## Test Plan

1. Start ccmux-server
2. Create a session from a small terminal (80x24)
3. Start ccmux-client in a large terminal (e.g., full-screen)
4. Verify viewport fills the terminal correctly
5. Verify PTY programs see correct dimensions (`stty size`)
6. Test with multiple clients of different sizes
7. Test detach and reattach

## Related Files

| File | Role |
|------|------|
| `ccmux-client/src/ui/app.rs` | Client state, event handling, UI pane creation |
| `ccmux-client/src/ui/pane.rs` | UI pane management and rendering |
| `ccmux-client/src/ui/terminal.rs` | Terminal size detection |
| `ccmux-server/src/session/pane.rs` | Server-side pane model (default 80x24) |
| `ccmux-server/src/handlers/session.rs` | AttachSession handler |
| `ccmux-server/src/handlers/pane.rs` | Resize handler |
| `ccmux-protocol/src/messages.rs` | ClientMessage::Resize definition |
