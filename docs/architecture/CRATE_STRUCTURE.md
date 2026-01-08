# ccmux Crate Structure

> Rust workspace organization and dependency management

## Workspace Layout

```
ccmux/
├── Cargo.toml              # Workspace root
├── ccmux-client/           # TUI client binary
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs
│       ├── ui/             # Ratatui components
│       ├── input/          # Keyboard/mouse handling
│       └── connection/     # Server communication
├── ccmux-server/           # Daemon binary
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs
│       ├── session/        # Session management
│       ├── pty/            # PTY lifecycle
│       ├── parser/         # Terminal parsing
│       ├── claude/         # Claude detection
│       ├── persistence/    # State checkpointing
│       └── config/         # Configuration management
├── ccmux-protocol/         # Shared IPC definitions
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── messages.rs     # Client/server messages
│       ├── types.rs        # Shared data types
│       └── codec.rs        # Serialization
└── ccmux-utils/            # Shared utilities
    ├── Cargo.toml
    └── src/
        ├── lib.rs
        ├── error.rs        # Error types
        ├── logging.rs      # Logging setup
        └── paths.rs        # Path utilities
```

## Crate Descriptions

### ccmux-client

The terminal UI client. Renders pane contents and handles user input.

**Binary**: `ccmux` (or `ccmux attach`)

**Key modules**:
- `ui/`: Ratatui widget implementations
- `input/`: Crossterm event processing
- `connection/`: Unix socket client

**Dependencies**:
```toml
[dependencies]
ccmux-protocol = { path = "../ccmux-protocol" }
ccmux-utils = { path = "../ccmux-utils" }
ratatui = "0.29"
crossterm = "0.28"
tokio = { version = "1", features = ["full"] }
tui-term = "0.2"  # vt100 → ratatui widget
```

### ccmux-server

The background daemon that manages PTYs, sessions, and persistence.

**Binary**: `ccmux-server` (or `ccmux server`)

**Key modules**:
- `session/`: Session/window/pane hierarchy
- `pty/`: PTY spawning and I/O via portable-pty
- `parser/`: Terminal state via vt100
- `claude/`: Claude Code state detection
- `persistence/`: Checkpoint and WAL
- `config/`: Hot-reload configuration

**Dependencies**:
```toml
[dependencies]
ccmux-protocol = { path = "../ccmux-protocol" }
ccmux-utils = { path = "../ccmux-utils" }
portable-pty = "0.9"
vt100 = "0.15"
tokio = { version = "1", features = ["full"] }
notify = "6"
notify-debouncer-full = "0.3"
arc_swap = "1"
okaywal = "0.3"
bincode = "1"
serde = { version = "1", features = ["derive"] }
uuid = { version = "1", features = ["v4"] }
```

### ccmux-protocol

Shared type definitions for client-server communication.

**Library only** (no binary)

**Key types**:
```rust
// messages.rs
pub enum ClientMessage { ... }
pub enum ServerMessage { ... }

// types.rs
pub struct SessionInfo { ... }
pub struct PaneInfo { ... }
pub struct ClaudeState { ... }

// codec.rs
pub struct MessageCodec { ... }
```

**Dependencies**:
```toml
[dependencies]
serde = { version = "1", features = ["derive"] }
bincode = "1"
uuid = { version = "1", features = ["v4", "serde"] }
tokio-util = { version = "0.7", features = ["codec"] }
bytes = "1"
```

### ccmux-utils

Common utilities shared across crates.

**Library only** (no binary)

**Key modules**:
- `error.rs`: Common error types with thiserror
- `logging.rs`: Tracing setup
- `paths.rs`: XDG paths, socket location

**Dependencies**:
```toml
[dependencies]
thiserror = "1"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
directories = "5"
```

## Dependency Graph

```
                    ┌─────────────────┐
                    │  ccmux-client   │
                    │    (binary)     │
                    └────────┬────────┘
                             │
              ┌──────────────┼──────────────┐
              │              │              │
              ▼              │              ▼
    ┌─────────────────┐      │     ┌─────────────────┐
    │ ccmux-protocol  │◄─────┘     │  ccmux-utils    │
    │   (library)     │            │   (library)     │
    └────────┬────────┘            └────────┬────────┘
             │                              │
             │    ┌─────────────────┐       │
             └───►│  ccmux-server   │◄──────┘
                  │    (binary)     │
                  └─────────────────┘
```

**Dependency rules**:
1. `ccmux-protocol` and `ccmux-utils` are leaf crates (no internal deps)
2. `ccmux-client` depends on protocol and utils
3. `ccmux-server` depends on protocol and utils
4. Client and server never depend on each other

## Public API Boundaries

### ccmux-protocol (public)

Everything in this crate is public - it defines the contract:

```rust
// Re-exported at crate root
pub use messages::{ClientMessage, ServerMessage};
pub use types::{SessionInfo, WindowInfo, PaneInfo, ClaudeState, PaneState};
pub use codec::MessageCodec;
```

### ccmux-utils (public)

Common utilities exposed to all crates:

```rust
pub use error::{CcmuxError, Result};
pub use paths::{socket_path, config_dir, state_dir};
pub fn init_logging() -> Result<()>;
```

### ccmux-server (internal)

Server internals are not exposed. Only the binary entry point:

```rust
// main.rs
#[tokio::main]
async fn main() -> Result<()> {
    ccmux_utils::init_logging()?;
    server::run().await
}
```

### ccmux-client (internal)

Client internals are not exposed. Only the binary entry point:

```rust
// main.rs
#[tokio::main]
async fn main() -> Result<()> {
    ccmux_utils::init_logging()?;
    client::run().await
}
```

## Workspace Cargo.toml

```toml
[workspace]
resolver = "2"
members = [
    "ccmux-client",
    "ccmux-server",
    "ccmux-protocol",
    "ccmux-utils",
]

[workspace.package]
version = "0.1.0"
edition = "2021"
license = "MIT"
repository = "https://github.com/user/ccmux"

[workspace.dependencies]
# Shared version pins
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
bincode = "1"
uuid = { version = "1", features = ["v4", "serde"] }
thiserror = "1"
tracing = "0.1"
```

## External Dependencies Summary

### Core Stack

| Crate | Version | Used By | Purpose |
|-------|---------|---------|---------|
| `portable-pty` | 0.9 | server | PTY management |
| `vt100` | 0.15 | server | Terminal parsing |
| `ratatui` | 0.29 | client | TUI framework |
| `crossterm` | 0.28 | client | Terminal backend |
| `tokio` | 1.x | all | Async runtime |

### Persistence

| Crate | Version | Used By | Purpose |
|-------|---------|---------|---------|
| `okaywal` | 0.3 | server | Write-ahead log |
| `bincode` | 1 | server, protocol | Binary serialization |
| `serde` | 1 | all | Serialization framework |

### Configuration

| Crate | Version | Used By | Purpose |
|-------|---------|---------|---------|
| `notify` | 6 | server | File watching |
| `notify-debouncer-full` | 0.3 | server | Event debouncing |
| `arc_swap` | 1 | server | Lock-free config |
| `serde_valid` | 0.20 | server | Config validation |

### Utilities

| Crate | Version | Used By | Purpose |
|-------|---------|---------|---------|
| `thiserror` | 1 | utils | Error derive |
| `tracing` | 0.1 | all | Structured logging |
| `directories` | 5 | utils | XDG paths |
| `uuid` | 1 | protocol, server | Identifiers |
| `tui-term` | 0.2 | client | vt100 to ratatui |

## Build Configuration

### Features

```toml
# ccmux-server/Cargo.toml
[features]
default = []
mcp = ["rmcp"]  # Optional MCP server support
```

### Profiles

```toml
# Workspace Cargo.toml
[profile.release]
lto = true
codegen-units = 1
strip = true

[profile.dev]
# Keep debug symbols for easier debugging
debug = true
```

## Testing Strategy

```
ccmux/
├── ccmux-client/
│   └── tests/
│       └── ui_tests.rs       # Widget rendering tests
├── ccmux-server/
│   └── tests/
│       ├── session_tests.rs  # Session management
│       ├── pty_tests.rs      # PTY integration
│       └── claude_tests.rs   # Claude detection
├── ccmux-protocol/
│   └── tests/
│       └── codec_tests.rs    # Serialization roundtrip
└── tests/                    # Workspace-level integration tests
    └── integration/
        ├── client_server.rs  # Full client-server flow
        └── crash_recovery.rs # Persistence tests
```

## Related Documents

- [ARCHITECTURE.md](./ARCHITECTURE.md) - System overview
- [CLAUDE_INTEGRATION.md](./CLAUDE_INTEGRATION.md) - Claude detection details
- [PERSISTENCE.md](./PERSISTENCE.md) - Server persistence design
