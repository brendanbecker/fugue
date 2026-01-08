# Feature Management Backfill Handoff

> This is a **parallel task** separate from the main implementation effort.
> Main implementation continues via `docs/HANDOFF.md`.

## Objective

Backfill the `feature-management/` system with formal feature definitions for ccmux.
Features FEAT-001 through FEAT-006 are already taken, so we start at **FEAT-007**.

## Current State

- `feature-management/features/features.md` shows **0 features**
- 14 features exist informally in `docs/HANDOFF.md`
- 6 features already implemented (Waves 1-2 complete)
- 8 features pending implementation

## Feature Mapping

Map the 14 informal features to formal FEAT-XXX IDs:

| Old # | Feature | FEAT ID | Status | Priority |
|-------|---------|---------|--------|----------|
| 1 | Protocol Layer | FEAT-007 | completed | P1 |
| 2 | Utilities | FEAT-008 | completed | P1 |
| 3 | Client UI | FEAT-009 | new | P1 |
| 4 | Client Input | FEAT-010 | new | P1 |
| 5 | Client Connection | FEAT-011 | completed | P1 |
| 6 | Session Management | FEAT-012 | completed | P1 |
| 7 | PTY Management | FEAT-013 | completed | P1 |
| 8 | Terminal Parsing | FEAT-014 | new | P1 |
| 9 | Claude Detection | FEAT-015 | new | P1 |
| 10 | Persistence | FEAT-016 | new | P2 |
| 11 | Configuration | FEAT-017 | completed | P2 |
| 12 | MCP Server | FEAT-018 | new | P2 |
| 13 | Sideband Protocol | FEAT-019 | new | P2 |
| 14 | Session Isolation | FEAT-020 | new | P1 |

## Task: Create Feature Files

For each feature, create a file in `feature-management/features/FEAT-XXX/`:

### Directory Structure (per feature)
```
feature-management/features/FEAT-007/
├── feature_request.json    # Metadata (schema-compliant)
└── PROMPT.md               # Implementation guidance
```

### Schema Requirements

Each `feature_request.json` must include (see `feature-management/schemas/feature-request.schema.json`):

```json
{
  "feature_id": "FEAT-007",
  "title": "Protocol Layer - IPC Message Types and Codec",
  "component": "ccmux-protocol",
  "priority": "P1",
  "status": "completed",
  "type": "new_feature",
  "created_date": "2026-01-07",
  "updated_date": "2026-01-08",
  "description": "...",
  "estimated_effort": "medium",
  "business_value": "high",
  "dependencies": [],
  "tags": ["protocol", "ipc", "foundation"]
}
```

### Feature Details

Use these descriptions for each feature:

**FEAT-007: Protocol Layer**
- Component: `ccmux-protocol`
- Description: Client/server IPC message types (ClientMessage, ServerMessage), shared data types (SessionInfo, PaneInfo, ClaudeState), and tokio codec for length-prefixed bincode framing.
- Effort: medium, Value: high

**FEAT-008: Utilities**
- Component: `ccmux-utils`
- Description: Common utilities including CcmuxError enum, logging infrastructure with tracing, and XDG-compliant path utilities for config/state/runtime directories.
- Effort: small, Value: high

**FEAT-009: Client UI**
- Component: `ccmux-client`
- Description: Ratatui-based terminal UI with pane rendering using tui-term, status bar, borders, and Claude state indicators.
- Effort: large, Value: high
- Dependencies: FEAT-007, FEAT-011

**FEAT-010: Client Input**
- Component: `ccmux-client`
- Description: Keyboard and mouse event handling via crossterm, prefix key support (tmux-style), and input routing to active pane.
- Effort: medium, Value: high
- Dependencies: FEAT-009

**FEAT-011: Client Connection**
- Component: `ccmux-client`
- Description: Unix socket client connecting to ccmux-server, async message framing, connection state management, and reconnection logic.
- Effort: medium, Value: high
- Dependencies: FEAT-007

**FEAT-012: Session Management**
- Component: `ccmux-server`
- Description: Session/Window/Pane hierarchy data model, CRUD operations, active selection tracking, and protocol type conversion.
- Effort: medium, Value: high
- Dependencies: FEAT-007

**FEAT-013: PTY Management**
- Component: `ccmux-server`
- Description: PTY spawning via portable-pty, process lifecycle management (spawn/kill/wait), resize support, and async I/O.
- Effort: medium, Value: high

**FEAT-014: Terminal Parsing**
- Component: `ccmux-server`
- Description: ANSI/VT100 terminal state parsing using vt100 crate, screen buffer management, and escape sequence handling.
- Effort: medium, Value: high
- Dependencies: FEAT-013

**FEAT-015: Claude Detection**
- Component: `ccmux-server`
- Description: Detect Claude Code state from PTY output (thinking, idle, tool use), capture session IDs, and emit state change events.
- Effort: large, Value: high
- Dependencies: FEAT-014

**FEAT-016: Persistence**
- Component: `ccmux-server`
- Description: Checkpoint + WAL persistence using okaywal and bincode for crash recovery, session state serialization.
- Effort: large, Value: medium
- Dependencies: FEAT-012

**FEAT-017: Configuration**
- Component: `ccmux-server`
- Description: TOML configuration with hot-reload via notify, lock-free access using ArcSwap, and validation.
- Effort: medium, Value: medium
- Dependencies: FEAT-008

**FEAT-018: MCP Server**
- Component: `ccmux-server`
- Description: Model Context Protocol server exposing tools for Claude to interact with ccmux (list panes, send input, create panes).
- Effort: large, Value: medium
- Dependencies: FEAT-012, FEAT-015

**FEAT-019: Sideband Protocol**
- Component: `ccmux-server`
- Description: Parse XML-style commands from Claude output (`<ccmux:spawn>`, `<ccmux:input>`) for lightweight Claude-ccmux communication.
- Effort: medium, Value: medium
- Dependencies: FEAT-014

**FEAT-020: Session Isolation**
- Component: `ccmux-server`
- Description: CLAUDE_CONFIG_DIR per pane for concurrent Claude instances, preventing config file conflicts.
- Effort: small, Value: high
- Dependencies: FEAT-013, FEAT-015

## Execution

Use `work-item-creation-agent` for each feature, or create files manually following the schema.

After creating all features, update `feature-management/features/features.md` summary.

## Validation

After completion:
1. All 14 `FEAT-XXX/` directories exist in `feature-management/features/`
2. Each has valid `feature_request.json` passing schema validation
3. `features.md` shows correct counts (6 completed, 8 new)
4. Dependencies form a valid DAG (no cycles)

## Session Prompt

```
Read docs/FEATURE_HANDOFF.md and create the 14 feature definitions
in feature-management/features/ following the schema. Start at FEAT-007.
Use work-item-creation-agent subagents to create features in batches.
```
