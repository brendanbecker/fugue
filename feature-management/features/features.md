# Feature Tracking

**Last Updated**: 2026-01-08
**Repository**: ccmux

## Summary Statistics

- **Total Features**: 20
- **By Priority**: P0: 0, P1: 13, P2: 7, P3: 0
- **By Status**:
  - New: 14
  - In Progress: 0
  - Completed: 6
  - Deprecated: 0

## Features by Priority

### P0 - Critical (0)

*No P0 features*

### P1 - High Priority (13)

| ID | Title | Component | Status | Link |
|----|-------|-----------|--------|------|
| FEAT-001 | Pane Content Abstraction (Terminal vs Canvas) | session/pane | new | [Link](FEAT-001-pane-content-abstraction/) |
| FEAT-002 | Per-Session-Type Scrollback Configuration | config | new | [Link](FEAT-002-per-session-type-scrollback/) |
| FEAT-005 | Response Channel for Orchestrator-Worker Communication | orchestration | new | [Link](FEAT-005-response-channel-orchestrator-worker/) |
| FEAT-007 | Protocol Layer - IPC Message Types and Codec | ccmux-protocol | completed | [Link](FEAT-007-protocol-layer-ipc-message-types-and-codec/) |
| FEAT-008 | Utilities - Error Types, Logging, and Path Helpers | ccmux-utils | completed | [Link](FEAT-008-utilities-error-types-logging-path-helpers/) |
| FEAT-009 | Client UI - Ratatui Terminal Interface | ccmux-client | new | [Link](FEAT-009-client-ui-ratatui-terminal-interface/) |
| FEAT-010 | Client Input - Keyboard and Mouse Event Handling | ccmux-client | new | [Link](FEAT-010-client-input-keyboard-and-mouse-event-handling/) |
| FEAT-011 | Client Connection - Unix Socket Client | ccmux-client | completed | [Link](FEAT-011-client-connection-unix-socket-client/) |
| FEAT-012 | Session Management - Session/Window/Pane Hierarchy | ccmux-server | completed | [Link](FEAT-012-session-management-hierarchy/) |
| FEAT-013 | PTY Management - Process Spawning and Lifecycle | ccmux-server | completed | [Link](FEAT-013-pty-management-process-spawning-and-lifecycle/) |
| FEAT-014 | Terminal Parsing - ANSI/VT100 State Machine | ccmux-server | new | [Link](FEAT-014-terminal-parsing-ansi-vt100-state-machine/) |
| FEAT-015 | Claude Detection - State Detection from PTY Output | ccmux-server | new | [Link](FEAT-015-claude-detection-state-detection-from-pty-output/) |
| FEAT-020 | Session Isolation - Per-Pane CLAUDE_CONFIG_DIR | ccmux-server | new | [Link](FEAT-020-session-isolation-per-pane-claude-config-dir/) |

### P2 - Medium Priority (7)

| ID | Title | Component | Status | Link |
|----|-------|-----------|--------|------|
| FEAT-003 | Viewport Pinning with New Content Indicator | tui | new | [Link](FEAT-003-viewport-pinning-with-new-content-indicator/) |
| FEAT-004 | Worktree-Aware Orchestration | orchestration | new | [Link](FEAT-004-worktree-aware-orchestration/) |
| FEAT-006 | Per-Session Log Levels and Storage | logging | new | [Link](FEAT-006-per-session-log-levels-and-storage/) |
| FEAT-016 | Persistence - Checkpoint and WAL for Crash Recovery | ccmux-server | new | [Link](FEAT-016-persistence-checkpoint-and-wal-for-crash-recovery/) |
| FEAT-017 | Configuration - TOML Config with Hot Reload | ccmux-server | completed | [Link](FEAT-017-configuration-toml-config-with-hot-reload/) |
| FEAT-018 | MCP Server - Model Context Protocol Integration | ccmux-server | new | [Link](FEAT-018-mcp-server-model-context-protocol-integration/) |
| FEAT-019 | Sideband Protocol - XML Command Parsing from Claude Output | ccmux-server | new | [Link](FEAT-019-sideband-protocol-xml-command-parsing/) |

### P3 - Low Priority (0)

*No P3 features*

## Recent Activity

- 2026-01-08: Created FEAT-018 - MCP Server - Model Context Protocol Integration
- 2026-01-08: Created FEAT-016 - Persistence - Checkpoint and WAL for Crash Recovery
- 2026-01-08: Created FEAT-015 - Claude Detection - State Detection from PTY Output
- 2026-01-08: Created FEAT-019 - Sideband Protocol - XML Command Parsing from Claude Output
- 2026-01-08: Created FEAT-017 - Configuration - TOML Config with Hot Reload (completed)
- 2026-01-08: Created FEAT-020 - Session Isolation - Per-Pane CLAUDE_CONFIG_DIR
- 2026-01-08: Created FEAT-008 - Utilities - Error Types, Logging, and Path Helpers (completed)
- 2026-01-08: Created FEAT-010 - Client Input - Keyboard and Mouse Event Handling
- 2026-01-08: Created FEAT-009 - Client UI - Ratatui Terminal Interface
- 2026-01-08: Created FEAT-011 - Client Connection - Unix Socket Client (completed)
- 2026-01-08: Created FEAT-012 - Session Management - Session/Window/Pane Hierarchy (completed)
- 2026-01-08: Created FEAT-007 - Protocol Layer - IPC Message Types and Codec (completed)
- 2026-01-08: Created FEAT-013 - PTY Management - Process Spawning and Lifecycle (completed)
- 2026-01-08: Created FEAT-002 - Per-Session-Type Scrollback Configuration
- 2026-01-08: Created FEAT-001 - Pane Content Abstraction (Terminal vs Canvas)
- 2026-01-08: Created FEAT-004 - Worktree-Aware Orchestration
- 2026-01-08: Created FEAT-003 - Viewport Pinning with New Content Indicator
- 2026-01-08: Created FEAT-006 - Per-Session Log Levels and Storage
- 2026-01-08: Created FEAT-005 - Response Channel for Orchestrator-Worker Communication

## Parallel Development Waves

See [WAVES.md](/WAVES.md) for the complete parallel development plan with dependency analysis.

**Quick Summary**:
- **Wave 0 (Complete)**: 6 foundation features (Protocol, Utilities, Connection, Session, PTY, Config)
- **Wave 1 (Ready)**: 9 features can start in parallel (UI, Terminal Parsing, Persistence, Architecture)
- **Wave 2 (Blocked)**: 3 features waiting on Wave 1 (Input, Claude Detection, Sideband)
- **Wave 3 (Blocked)**: 2 features waiting on Wave 2 (MCP Server, Session Isolation)

## Planned Feature Areas

Based on the project vision, features will likely include:

### Core Terminal Multiplexer
- PTY spawning and management
- Pane layout and navigation
- Scrollback and copy mode
- **Keyboard input handling (FEAT-010)**
- **Terminal parsing - ANSI/VT100 state machine (FEAT-014)**
- **Pane content abstraction for Terminal vs Canvas (FEAT-001)**

### Claude Code Integration
- State detection (thinking, waiting, complete)
- Visual indicators for Claude activity
- Session tracking for `--resume` support
- Structured output parsing (`<ccmux:spawn>`)
- **State detection from PTY output (FEAT-015)**
- **Per-pane CLAUDE_CONFIG_DIR isolation (FEAT-020)**
- **Sideband protocol for XML command parsing (FEAT-019)**
- **MCP Server for Claude interaction (FEAT-018)**

### Session Management
- Session persistence and recovery
- Crash recovery with automatic resume
- Session tree visualization
- **Session/Window/Pane hierarchy (FEAT-012)** - Completed
- **Checkpoint + WAL persistence for crash recovery (FEAT-016)**

### Orchestration
- Child pane spawning on Claude request
- Recursion depth limits
- Parent notification on child completion
- **Response channel for orchestrator-worker communication (FEAT-005)**
- **Worktree-aware orchestration for parallel development (FEAT-004)**

### Configuration
- **TOML config with hot-reload (FEAT-017)** - Completed
- Customizable keybindings
- Theme support
- **Per-session-type scrollback configuration (FEAT-002)**

### Client Connection
- **Unix socket client with async message framing (FEAT-011)** - Completed

### Client UI
- **Ratatui-based terminal interface (FEAT-009)**
- **Keyboard and mouse event handling (FEAT-010)**
- Pane rendering with tui-term
- Status bar and borders
- Claude state indicators

### Utilities
- **Error types, logging, and path helpers (FEAT-008)** - Completed
