# Feature Tracking

**Last Updated**: 2026-01-07
**Repository**: ccmux

## Summary Statistics

- **Total Features**: 0
- **By Priority**: P0: 0, P1: 0, P2: 0, P3: 0
- **By Status**:
  - New: 0
  - In Progress: 0
  - Completed: 0
  - Deprecated: 0

## Features by Priority

### P0 - Critical (0)

*No P0 features*

### P1 - High Priority (0)

*No P1 features*

### P2 - Medium Priority (0)

*No P2 features*

### P3 - Low Priority (0)

*No P3 features*

## Recent Activity

*No features created yet. Project is in early development.*

**Current Stage**: Stage 1 (Ideation) / Stage 2 (Deep Research)

Features will be generated after:
1. Deep research is completed with Gemini, ChatGPT, and Claude
2. Research documents are parsed
3. Architecture is defined (ARCHITECTURE.md)
4. Feature decomposition session is run

See [docs/DEEP_RESEARCH_PROMPT.md](../../docs/DEEP_RESEARCH_PROMPT.md) for the research prompt.

## Planned Feature Areas

Based on the project vision, features will likely include:

### Core Terminal Multiplexer
- PTY spawning and management
- Pane layout and navigation
- Scrollback and copy mode
- Keyboard input handling

### Claude Code Integration
- State detection (thinking, waiting, complete)
- Visual indicators for Claude activity
- Session tracking for `--resume` support
- Structured output parsing (`<ccmux:spawn>`)

### Session Management
- Session persistence and recovery
- Crash recovery with automatic resume
- Session tree visualization

### Orchestration
- Child pane spawning on Claude request
- Recursion depth limits
- Parent notification on child completion

### Configuration
- Hot-reload configuration
- Customizable keybindings
- Theme support
