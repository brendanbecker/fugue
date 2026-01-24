# fugue Architecture

> High-level system architecture for the Claude Code-aware terminal multiplexer

## Overview

fugue is a terminal multiplexer designed with first-class awareness of Claude Code. Unlike traditional multiplexers (tmux, screen) that treat all processes as opaque byte streams, fugue understands Claude Code's state, can detect activity patterns, and enables intelligent session management including crash recovery.

## Design Philosophy

1. **Claude-first**: Built around Claude Code's specific behaviors and needs
2. **Crash-resilient**: Sessions survive terminal crashes, SSH disconnects, and reboots
3. **Separation of concerns**: Client renders, server manages state
4. **Progressive enhancement**: Basic multiplexer features work without Claude-specific integrations

## System Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              User Terminal                                   │
│  ┌───────────────────────────────────────────────────────────────────────┐  │
│  │                           fugue-client                                 │  │
│  │  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────────┐   │  │
│  │  │   Ratatui UI    │  │  Input Handler  │  │  Protocol Client    │   │  │
│  │  │  (rendering)    │  │  (keys, mouse)  │  │  (IPC messages)     │   │  │
│  │  └────────┬────────┘  └────────┬────────┘  └──────────┬──────────┘   │  │
│  │           │                    │                      │               │  │
│  │           └────────────────────┴──────────────────────┘               │  │
│  │                                │                                       │  │
│  └────────────────────────────────┼───────────────────────────────────────┘  │
└───────────────────────────────────┼──────────────────────────────────────────┘
                                    │
                              Unix Socket
                         (~/.fugue/fugue.sock)
                                    │
┌───────────────────────────────────┼──────────────────────────────────────────┐
│                              fugue-server (daemon)                           │
│  ┌────────────────────────────────┴───────────────────────────────────────┐  │
│  │                         Protocol Server                                 │  │
│  │                      (IPC message handling)                             │  │
│  └────────────────────────────────┬───────────────────────────────────────┘  │
│                                   │                                          │
│  ┌────────────────────────────────┼───────────────────────────────────────┐  │
│  │                        Session Manager                                  │  │
│  │  ┌─────────────────────────────┴─────────────────────────────────────┐ │  │
│  │  │                          Sessions                                  │ │  │
│  │  │   ┌─────────────────────────────────────────────────────────┐     │ │  │
│  │  │   │ Session "main"                                          │     │ │  │
│  │  │   │   ┌─────────────────────────────────────────────────┐   │     │ │  │
│  │  │   │   │ Window 0                                        │   │     │ │  │
│  │  │   │   │   ┌──────────────┐  ┌──────────────┐            │   │     │ │  │
│  │  │   │   │   │   Pane 0     │  │   Pane 1     │            │   │     │ │  │
│  │  │   │   │   │  (Claude)    │  │  (Shell)     │            │   │     │ │  │
│  │  │   │   │   │  ┌────────┐  │  │  ┌────────┐  │            │   │     │ │  │
│  │  │   │   │   │  │  PTY   │  │  │  │  PTY   │  │            │   │     │ │  │
│  │  │   │   │   │  │ Parser │  │  │  │ Parser │  │            │   │     │ │  │
│  │  │   │   │   │  └────────┘  │  │  └────────┘  │            │   │     │ │  │
│  │  │   │   │   └──────────────┘  └──────────────┘            │   │     │ │  │
│  │  │   │   └─────────────────────────────────────────────────┘   │     │ │  │
│  │  │   └─────────────────────────────────────────────────────────┘     │ │  │
│  │  └───────────────────────────────────────────────────────────────────┘ │  │
│  └────────────────────────────────────────────────────────────────────────┘  │
│                                                                              │
│  ┌────────────────────┐  ┌────────────────────┐  ┌──────────────────────┐   │
│  │  Persistence Layer │  │  Config Watcher    │  │  Claude Detector     │   │
│  │  (checkpoint/WAL)  │  │  (hot-reload)      │  │  (state parsing)     │   │
│  └────────────────────┘  └────────────────────┘  └──────────────────────┘   │
└──────────────────────────────────────────────────────────────────────────────┘
```

## Client-Server Model

### Why Separate Client and Server?

| Benefit | Description |
|---------|-------------|
| **Crash isolation** | Client crash doesn't kill running processes |
| **Detach/Reattach** | Disconnect terminal, reconnect later |
| **Multiple clients** | View same session from different terminals |
| **Resource management** | Server can run as systemd service |
| **Clean shutdown** | Graceful handling of terminal closure |

### Communication

- **Transport**: Unix domain socket (`~/.fugue/fugue.sock`)
- **Protocol**: Length-prefixed binary frames with serde serialization
- **Serialization**: bincode for performance, JSON for debugging

## Component Responsibilities

### fugue-client

| Component | Responsibility |
|-----------|----------------|
| **Ratatui UI** | Render pane contents, status bar, borders |
| **Input Handler** | Process keyboard/mouse events, route to server |
| **Protocol Client** | Serialize/deserialize IPC messages |

The client is intentionally thin. It maintains minimal state and can be replaced or restarted without affecting sessions.

### fugue-server

| Component | Responsibility |
|-----------|----------------|
| **Protocol Server** | Accept client connections, dispatch messages |
| **Session Manager** | Create/destroy sessions, windows, panes |
| **PTY Manager** | Spawn processes, manage PTY lifecycle |
| **Terminal Parser** | Parse ANSI escape sequences (vt100 crate) |
| **Claude Detector** | Identify Claude panes, track state |
| **Persistence Layer** | Checkpoint state, maintain WAL |
| **Config Watcher** | Monitor config files, trigger hot-reload |

### fugue-protocol

Defines all IPC messages between client and server:

```rust
// Client → Server
enum ClientMessage {
    Connect { client_id: Uuid },
    CreateSession { name: String },
    CreatePane { session: String, window: usize, direction: SplitDirection },
    Input { pane_id: Uuid, data: Vec<u8> },
    Resize { pane_id: Uuid, cols: u16, rows: u16 },
    Detach,
}

// Server → Client
enum ServerMessage {
    SessionList { sessions: Vec<SessionInfo> },
    PaneOutput { pane_id: Uuid, data: Vec<u8> },
    PaneState { pane_id: Uuid, state: PaneState },
    ClaudeState { pane_id: Uuid, state: ClaudeState },
    Error { code: ErrorCode, message: String },
}
```

### fugue-utils

Shared utilities used by both client and server:
- Configuration parsing and validation
- Logging infrastructure
- Error types
- Common data structures

## Session Hierarchy

```
fugue-server
└── Session "work"
    ├── Window 0 "main"
    │   ├── Pane 0 (Claude Code, session: abc123)
    │   │   ├── PTY: /dev/pts/4
    │   │   ├── Parser: vt100
    │   │   ├── State: Thinking
    │   │   └── cwd: /home/user/project
    │   └── Pane 1 (bash)
    │       ├── PTY: /dev/pts/5
    │       ├── Parser: vt100
    │       └── cwd: /home/user/project
    └── Window 1 "logs"
        └── Pane 0 (tail -f app.log)
```

## Data Flow

### Input Flow (User → Process)

```
┌────────────┐    ┌────────────┐    ┌────────────┐    ┌────────────┐
│  Terminal  │───►│   Client   │───►│   Server   │───►│    PTY     │
│  (stdin)   │    │ (crossterm)│    │ (dispatch) │    │  (write)   │
└────────────┘    └────────────┘    └────────────┘    └────────────┘
     Key             Serialize        Route to          Send to
   pressed          as Input{}       pane by ID        process
```

### Output Flow (Process → Display)

```
┌────────────┐    ┌────────────┐    ┌────────────┐    ┌────────────┐
│    PTY     │───►│   Parser   │───►│   Server   │───►│   Client   │
│  (read)    │    │  (vt100)   │    │ (broadcast)│    │ (ratatui)  │
└────────────┘    └────────────┘    └────────────┘    └────────────┘
   Process          Parse ANSI       Send diff        Render to
   output          sequences         to clients       terminal
```

### Claude State Detection Flow

```
┌────────────┐    ┌────────────┐    ┌────────────┐    ┌────────────┐
│    PTY     │───►│  Detector  │───►│   Server   │───►│   Client   │
│  (output)  │    │ (patterns) │    │ (update)   │    │ (status)   │
└────────────┘    └────────────┘    └────────────┘    └────────────┘
   Claude          Match state       Store state      Update status
   output          indicators        per-pane         bar indicator
```

## Technology Stack

### Core Dependencies

| Crate | Version | Purpose |
|-------|---------|---------|
| `portable-pty` | 0.9 | Cross-platform PTY management |
| `vt100` | latest | Terminal state parsing |
| `ratatui` | 0.29 | TUI rendering framework |
| `crossterm` | 0.28 | Terminal backend for ratatui |
| `tokio` | 1.x | Async runtime |
| `serde` | latest | Serialization framework |
| `bincode` | latest | Binary serialization |

### Supporting Dependencies

| Crate | Purpose |
|-------|---------|
| `notify` | File system watching for config |
| `arc_swap` | Lock-free config swapping |
| `okaywal` | Write-ahead logging |
| `tui-term` | vt100 to ratatui widget |
| `uuid` | Session and pane identifiers |

See [CRATE_STRUCTURE.md](./CRATE_STRUCTURE.md) for the full dependency graph.

## Key Design Decisions

### Decision 1: vt100 over alacritty_terminal

We chose `vt100` as the terminal parser for its simpler API and `contents_diff()` method that enables efficient incremental updates. See [ADR-001](./ADR/001-terminal-parser.md).

### Decision 2: Dual Communication Protocols

We support both MCP Server and XML Sideband protocols for Claude communication. MCP provides structured orchestration; sideband enables lightweight interaction. See [ADR-002](./ADR/002-claude-communication.md).

### Decision 3: Environment-based Session Isolation

For concurrent Claude instances, we use `CLAUDE_CONFIG_DIR` environment variable rather than HOME isolation. See [ADR-003](./ADR/003-session-isolation.md).

### Decision 4: Hybrid Persistence

We use checkpoints (every 30-60s) combined with a write-ahead log for crash recovery. See [PERSISTENCE.md](./PERSISTENCE.md).

## Security Considerations

1. **Socket permissions**: Unix socket created with user-only permissions
2. **Process isolation**: Each pane runs in its own PTY with no shared state
3. **No network exposure**: Server only listens on local Unix socket
4. **Config validation**: All configuration validated before application

## Performance Targets

| Metric | Target | Rationale |
|--------|--------|-----------|
| Render latency | <16ms | 60 FPS smooth scrolling |
| Input latency | <5ms | Responsive typing |
| Parser timeout | 5s | Prevent hanging on malformed sequences |
| Memory per pane | <50MB | Reasonable scrollback buffer |

## Related Documents

- [CRATE_STRUCTURE.md](./CRATE_STRUCTURE.md) - Workspace organization
- [CLAUDE_INTEGRATION.md](./CLAUDE_INTEGRATION.md) - Claude Code specifics
- [PERSISTENCE.md](./PERSISTENCE.md) - Crash recovery design
- [CONFIGURATION.md](./CONFIGURATION.md) - Hot-reload system
- [ADR/](./ADR/) - Architecture Decision Records
