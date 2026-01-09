# SESSION_PROMPT: FEAT-002 Per-Session Scrollback Configuration

**Feature**: FEAT-002 - Per-Session-Type Scrollback Configuration
**Priority**: P1 | **Effort**: Medium | **Component**: config

## Objective

Implement per-session-type scrollback buffer sizes instead of global-only configuration. This enables fine-grained memory management where orchestrator sessions get large buffers (50k lines) while ephemeral workers get minimal buffers (500 lines).

## Key Deliverables

1. **ScrollbackConfig schema** with per-type settings
2. **ScrollbackBuffer** circular buffer implementation
3. **Runtime override** via `<ccmux:spawn scrollback="N">`
4. **Memory management** with bounded usage

## Technical Design

```rust
// Config schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScrollbackConfig {
    pub default: usize,      // Default: 1000
    pub orchestrator: usize, // Default: 50000
    pub worker: usize,       // Default: 500
    #[serde(flatten)]
    pub custom: HashMap<String, usize>,
}

// Circular buffer
pub struct ScrollbackBuffer {
    lines: VecDeque<String>,
    max_lines: usize,
    total_bytes: usize,
}
```

## Existing Code Context

- `ccmux-server/src/config/schema.rs` - Has `TerminalConfig` with global `scrollback_lines: 10000`
- `ccmux-server/src/session/pane.rs` - Pane struct needs scrollback buffer field
- `ccmux-server/src/pty/handle.rs` - PTY output needs buffer integration
- No actual scrollback buffer implementation exists yet

## Implementation Sections

### Section 1: Config Schema Extension
- [ ] Add `ScrollbackConfig` struct to schema.rs
- [ ] Update `TerminalConfig` to use nested scrollback config
- [ ] Add validation for scrollback values
- [ ] Update config defaults

### Section 2: ScrollbackBuffer Implementation
- [ ] Create `ccmux-server/src/pty/buffer.rs`
- [ ] Implement `ScrollbackBuffer` with VecDeque
- [ ] Add `push_line()` with automatic truncation
- [ ] Add `get_lines()` for retrieval
- [ ] Track total bytes for memory monitoring

### Section 3: Pane Integration
- [ ] Add scrollback buffer field to Pane struct
- [ ] Initialize buffer based on session type
- [ ] Support per-pane override at creation time
- [ ] Connect buffer to PTY output stream

### Section 4: Spawn Directive Support
- [ ] Parse `scrollback="N"` attribute in spawn directives
- [ ] Override session-type default with spawn value
- [ ] Validate scrollback value range

### Section 5: Memory Management
- [ ] Track total scrollback memory across panes
- [ ] Log warnings approaching memory limits
- [ ] Consider lazy allocation strategy
- [ ] Document memory budget (~5MB per 50k line buffer)

### Section 6: Testing
- [ ] Unit tests for ScrollbackBuffer operations
- [ ] Unit tests for config parsing
- [ ] Integration tests for session-type defaults
- [ ] Test hot-reload behavior (new panes use new values)

## Acceptance Criteria

- [ ] Configuration supports per-session-type scrollback values
- [ ] Default, orchestrator, and worker types work correctly
- [ ] Custom session types can be defined in config
- [ ] `<ccmux:spawn scrollback="N">` overrides default
- [ ] Scrollback buffer stores and retrieves historical output
- [ ] Memory usage is bounded and predictable
- [ ] Hot-reload updates settings for new panes only
- [ ] All tests passing
- [ ] `cargo clippy` clean

## Commands

```bash
# Build
cargo build -p ccmux-server

# Test
cargo test -p ccmux-server

# Clippy
cargo clippy -p ccmux-server -- -D warnings
```

## Notes

- VT100 escape sequences should be preserved in scrollback for faithful replay
- Consider using `bytes::Bytes` for zero-copy line storage
- Large buffers: 50k lines @ ~100 bytes/line = ~5MB per pane
- With 10 orchestrator panes, budget ~50MB for scrollback alone
