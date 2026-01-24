# FEAT-002: Per-Session-Type Scrollback Configuration

**Priority**: P1
**Component**: config
**Type**: enhancement
**Estimated Effort**: medium
**Business Value**: high

## Overview

Support per-session-type buffer sizes instead of global-only scrollback. This enables fine-grained memory management for multi-agent orchestration scenarios where orchestrator sessions need large scrollback buffers while ephemeral worker sessions need minimal memory footprint.

## Current State

- Global `scrollback_lines: 10000` exists in `TerminalConfig` (`fugue-server/src/config/schema.rs`)
- Config is hot-reloadable via ArcSwap
- No actual scrollback buffer implementation exists (parser.rs was planned but not created)
- No per-pane override capability in `Pane` struct (`fugue-server/src/session/pane.rs`)

## Requirements

### 1. Config Schema Extension

Extend the configuration schema to support per-session-type scrollback settings:

```toml
[terminal.scrollback]
default = 1000
orchestrator = 50000
worker = 500
```

Or in JSON representation:
```json
{
  "scrollback": {
    "orchestrator": 50000,
    "worker": 500,
    "default": 1000
  }
}
```

### 2. Runtime Override via Spawn Directive

Allow scrollback override at spawn time via the `<fugue:spawn>` directive:

```xml
<fugue:spawn scrollback="10000">command args</fugue:spawn>
```

This should take precedence over session-type defaults.

### 3. Actual Scrollback Buffer Implementation

Implement the actual scrollback buffer that currently does not exist:

- Create scrollback buffer in PTY/parser layer
- Use circular buffer or efficient truncation strategy
- Support line-based storage with proper line wrapping handling

### 4. Memory Management

Implement memory management for large buffers across many panes:

- Track total memory usage across all pane buffers
- Consider lazy allocation (allocate on demand)
- Implement buffer trimming when memory pressure detected
- Log warnings when approaching memory limits

## Affected Files

| File | Changes Required |
|------|------------------|
| `fugue-server/src/config/schema.rs` | Add `ScrollbackConfig` struct, update `TerminalConfig` |
| `fugue-server/src/pty/` | Add new `buffer.rs` module for scrollback buffer |
| `fugue-server/src/session/pane.rs` | Add scrollback buffer field, per-pane override |
| `fugue-server/src/pty/handle.rs` | Integrate scrollback buffer with PTY output |

## Technical Approach

### ScrollbackConfig Schema

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ScrollbackConfig {
    /// Default scrollback lines for unspecified session types
    pub default: usize,
    /// Scrollback for orchestrator sessions (large)
    pub orchestrator: usize,
    /// Scrollback for worker sessions (small)
    pub worker: usize,
    /// Custom per-type overrides
    #[serde(flatten)]
    pub custom: HashMap<String, usize>,
}
```

### Circular Buffer Implementation

```rust
pub struct ScrollbackBuffer {
    lines: VecDeque<String>,
    max_lines: usize,
    total_bytes: usize,
}

impl ScrollbackBuffer {
    pub fn push_line(&mut self, line: String) {
        self.total_bytes += line.len();
        self.lines.push_back(line);
        while self.lines.len() > self.max_lines {
            if let Some(removed) = self.lines.pop_front() {
                self.total_bytes -= removed.len();
            }
        }
    }
}
```

## Acceptance Criteria

- [ ] Configuration supports per-session-type scrollback values
- [ ] Default, orchestrator, and worker types work correctly
- [ ] Custom session types can be defined in config
- [ ] `<fugue:spawn scrollback="N">` overrides session-type default
- [ ] Scrollback buffer actually stores and retrieves historical output
- [ ] Memory usage is bounded and predictable
- [ ] Hot-reload updates scrollback settings (new panes use new values)
- [ ] Existing panes retain their buffer on config reload
- [ ] Unit tests cover buffer operations and config parsing
- [ ] Integration tests verify end-to-end scrollback behavior

## Notes

- Consider using `bytes::Bytes` for zero-copy line storage
- VT100 escape sequences should be preserved in scrollback for faithful replay
- Large orchestrator buffers (50k lines) at ~100 bytes/line = ~5MB per pane
- With 10 orchestrator panes, budget ~50MB for scrollback alone
