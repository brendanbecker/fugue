# Stage 5 Handoff: Implementation Planning

> **LIVING DOCUMENT**: This handoff file is the interface between sessions. Update it constantly as you work—mark completed items, add discoveries, note blockers, revise plans. The next session (or a resumed session) relies on this being current. When you finish, update the title to the next stage and rewrite for the incoming session.

## Context

You are continuing work on **ccmux**, a Claude Code-aware terminal multiplexer in Rust. We are following the [Context Engineering Methodology](./CONTEXT_ENGINEERING_METHODOLOGY.md).

**Current Stage**: Stage 5 (Implementation Planning / Feature Generation)
**Previous Stage**: Stage 4 (Architecture Generation) - COMPLETE

## What Was Done

### Stage 1 (Ideation)
Produced `docs/DEEP_RESEARCH_PROMPT.md` covering 7 research areas.

### Stage 2 (Deep Research)
Gathered research from three LLMs:
- `docs/research/claude_research.md` (24KB, ~4,500 tokens)
- `docs/research/gemini_research.md` (36KB, ~8,500 tokens)
- `docs/research/chatgpt_research.pdf` (351KB, ~12,000 tokens)

### Stage 3 (Document Parsing) - COMPLETE
Parsed all research into structured, navigable chunks:
- 12 parsed output files in `docs/research/parsed/`
- Key deliverable: `docs/research/SYNTHESIS.md`

### Stage 4 (Architecture Generation) - COMPLETE
Generated formal architecture documentation:

**Architecture Documents (8 files)**:
```
docs/architecture/
├── ARCHITECTURE.md         # System overview, client-server model, data flow
├── CRATE_STRUCTURE.md      # Workspace layout, dependency graph
├── CLAUDE_INTEGRATION.md   # Detection, session management, communication
├── PERSISTENCE.md          # Checkpoint + WAL, recovery flow
├── CONFIGURATION.md        # Hot-reload with ArcSwap, validation
└── ADR/
    ├── 001-terminal-parser.md      # vt100 first, migrate if needed
    ├── 002-claude-communication.md # Both MCP and sideband protocols
    └── 003-session-isolation.md    # CLAUDE_CONFIG_DIR approach
```

**Key Architectural Decisions**:
- Client-server with Unix socket IPC
- Four-crate workspace: client, server, protocol, utils
- vt100 for terminal parsing (with alacritty_terminal fallback path)
- Dual communication: MCP for orchestration, XML sideband for lightweight
- CLAUDE_CONFIG_DIR for concurrent Claude isolation
- Hybrid checkpoint (30-60s) + WAL for persistence
- ArcSwap for lock-free hot-reload config

## Your Task: Implementation Planning

Generate implementable feature specifications using the featmgmt pattern.

### Key Inputs

1. **Architecture Documents** - PRIMARY INPUTS
   - `docs/architecture/ARCHITECTURE.md` - Start here for system overview
   - `docs/architecture/CRATE_STRUCTURE.md` - Workspace structure
   - `docs/architecture/CLAUDE_INTEGRATION.md` - Claude specifics
   - `docs/architecture/PERSISTENCE.md` - Crash recovery
   - `docs/architecture/CONFIGURATION.md` - Hot-reload system
   - `docs/architecture/ADR/` - Decision rationales

2. **Research Synthesis** (for deep dives)
   - `docs/research/SYNTHESIS.md` - Technology decisions, crate versions

### Expected Outputs

Create in `feature-management/features/`:

**Phase 1: Foundation**
```
feature-001-workspace-setup/PROMPT.md      # Cargo workspace, crate scaffolding
feature-002-protocol-types/PROMPT.md       # IPC message definitions
feature-003-server-skeleton/PROMPT.md      # Basic tokio server with socket
feature-004-client-skeleton/PROMPT.md      # Basic ratatui client
feature-005-pty-management/PROMPT.md       # portable-pty integration
feature-006-terminal-parsing/PROMPT.md     # vt100 integration
```

**Phase 2: Core Multiplexing**
```
feature-007-pane-splits/PROMPT.md          # Horizontal/vertical splits
feature-008-session-management/PROMPT.md   # Session/window/pane hierarchy
feature-009-detach-attach/PROMPT.md        # Detach/reattach functionality
feature-010-input-routing/PROMPT.md        # Keyboard input to correct pane
```

**Phase 3: Claude Awareness**
```
feature-011-claude-detection/PROMPT.md     # PTY-based state detection
feature-012-session-capture/PROMPT.md      # Claude session ID discovery
feature-013-crash-recovery/PROMPT.md       # --resume on crash
feature-014-sideband-parsing/PROMPT.md     # XML command parsing
```

**Phase 4: Robustness**
```
feature-015-checkpointing/PROMPT.md        # Periodic state snapshots
feature-016-wal-implementation/PROMPT.md   # Write-ahead log
feature-017-hot-reload/PROMPT.md           # Config file watching
feature-018-mcp-server/PROMPT.md           # MCP tool exposure
```

### Feature PROMPT.md Template

Each feature should follow the featmgmt pattern:

```markdown
# Feature: [Name]

## Overview
[Brief description of what this feature does]

## Dependencies
- Requires: [list feature IDs]
- Blocked by: [any blockers]

## Architecture References
- See: [relevant architecture doc sections]

## Implementation Details

### Files to Create/Modify
- `path/to/file.rs` - [purpose]

### Key Types/Structs
[Outline main types to implement]

### Tests Required
- [ ] Unit test: [description]
- [ ] Integration test: [description]

## Acceptance Criteria
- [ ] [Specific, testable criterion]
- [ ] [Another criterion]

## Notes
[Any implementation notes, gotchas, or alternatives considered]
```

### Phased Implementation

From SYNTHESIS.md Section 9:

**Phase 1: Foundation**
1. Client-server with Unix socket IPC
2. `portable-pty` + `vt100` + `ratatui` stack
3. Basic pane splits (horizontal/vertical)
4. Detach/attach functionality

**Phase 2: Claude Awareness**
1. Claude state detection (PTY or stream-json)
2. Session ID capture and storage
3. `--resume` on crash recovery
4. Basic SKILL.md sideband parsing

**Phase 3: Robustness**
1. Hybrid checkpoint + WAL persistence
2. Full supervision tree
3. MCP server integration
4. Hot-reload configuration
5. cgroups resource limits (optional)

### Recommended Approach

1. Read `ARCHITECTURE.md` for system overview
2. Read `CRATE_STRUCTURE.md` for workspace layout
3. Create Phase 1 features first (foundation)
4. Ensure each feature has clear dependencies
5. Include specific file paths from architecture docs
6. Reference ADRs for contested decisions

## Files to Read First

1. **`docs/architecture/ARCHITECTURE.md`** - System overview
2. **`docs/architecture/CRATE_STRUCTURE.md`** - Workspace layout
3. `docs/research/SYNTHESIS.md` - Technology stack details
4. `feature-management/README.md` - featmgmt pattern reference

## Success Criteria

- [ ] Phase 1 features created (6 features)
- [ ] Phase 2 features created (4 features)
- [ ] Phase 3 features created (4 features)
- [ ] Phase 4 features created (4 features)
- [ ] Each feature has clear dependencies
- [ ] Each feature references architecture docs
- [ ] Ready for Stage 6 (Implementation)

---

## Session Log

*Update this section as you work. Track what's done, what's blocked, what changed.*

### 2026-01-07 (Stage 4 Completion)
- Created `docs/architecture/` directory structure
- Wrote ARCHITECTURE.md with:
  - Client-server model diagrams
  - Component responsibilities
  - Data flow diagrams
  - Technology stack table
- Wrote CRATE_STRUCTURE.md with:
  - Four-crate workspace layout
  - Dependency graph
  - External dependency summary
- Wrote CLAUDE_INTEGRATION.md with:
  - Three detection methods (PTY, stream-json, visual)
  - Session resume commands
  - CLAUDE_CONFIG_DIR isolation
  - MCP and sideband protocols
- Wrote PERSISTENCE.md with:
  - Hybrid checkpoint + WAL strategy
  - Recovery flow diagram
  - Atomic write pattern
  - Ghost image concept
- Wrote CONFIGURATION.md with:
  - Full config schema
  - Change categories (hot-reload vs restart)
  - ArcSwap lock-free pattern
  - Validation approach
- Created 3 ADRs:
  - ADR-001: vt100 first, migrate to alacritty_terminal if needed
  - ADR-002: Both MCP and XML sideband protocols
  - ADR-003: CLAUDE_CONFIG_DIR for session isolation
- Updated HANDOFF.md for Stage 5
