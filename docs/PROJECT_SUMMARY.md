# fugue Project Summary

> Synthesized findings from deep research and architectural design

## Project Status

**Stage**: 5 of 6 (Implementation Planning)
**Architecture**: Complete
**Implementation**: Not started

## Problem Statement

tmux doesn't know what's running inside it. When Claude Code runs in a tmux pane, tmux sees opaque byte streams. It cannot:
- Detect Claude's state (thinking, waiting, complete)
- Recover crashed Claude sessions with `--resume`
- Orchestrate sub-agent spawning
- Provide Claude-aware notifications

## Solution

fugue is a terminal multiplexer with first-class Claude Code awareness:
- **State detection**: Parse PTY output for Claude activity patterns
- **Session management**: Capture and store Claude session IDs
- **Crash recovery**: Auto-resume Claude sessions after crashes
- **Orchestration**: MCP and sideband protocols for Claude-to-fugue communication

## Technology Stack

| Component | Technology | Version |
|-----------|------------|---------|
| PTY Management | portable-pty | 0.9 |
| Terminal Parser | vt100 | 0.15 |
| TUI Framework | ratatui | 0.29 |
| Terminal Backend | crossterm | 0.28 |
| Async Runtime | tokio | 1.x |
| Serialization | serde + bincode | latest |
| File Watching | notify | 6 |
| Config Swap | arc_swap | 1 |
| WAL | okaywal | 0.3 |

## Architecture Overview

```
┌──────────────┐         Unix Socket         ┌──────────────┐
│  fugue-client │◄───────────────────────────►│ fugue-server  │
│  (Ratatui UI) │                             │   (Daemon)    │
└──────────────┘                             └──────┬───────┘
                                                    │
                              ┌─────────────────────┼─────────────────────┐
                              │                     │                     │
                         ┌────▼────┐          ┌────▼────┐          ┌────▼────┐
                         │  PTY 0  │          │  PTY 1  │          │  PTY N  │
                         │ (Claude)│          │ (Shell) │          │  (...)  │
                         └─────────┘          └─────────┘          └─────────┘
```

## Key Decisions

1. **Client-Server**: UI crash doesn't kill sessions; enables detach/attach
2. **vt100 Parser**: Simpler API, migrate to alacritty_terminal if needed
3. **Dual Communication**: MCP for orchestration, XML sideband for lightweight
4. **CLAUDE_CONFIG_DIR**: Per-instance isolation preserving shell environment
5. **Hybrid Persistence**: Checkpoints (30-60s) + WAL for crash recovery

## Implementation Phases

| Phase | Features | Status |
|-------|----------|--------|
| 1. Foundation | Workspace, IPC, PTY, parsing | Planned |
| 2. Multiplexing | Splits, sessions, detach/attach | Planned |
| 3. Claude | Detection, resume, sideband | Planned |
| 4. Robustness | Persistence, hot-reload, MCP | Planned |

## Documentation

- **Research**: `docs/research/SYNTHESIS.md`
- **Architecture**: `docs/architecture/`
- **Features**: `feature-management/features/` (Stage 5)
- **Handoff**: `docs/HANDOFF.md`

## Risks and Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| vt100 limitations | Medium | Abstraction layer, alacritty_terminal fallback |
| CLAUDE_CONFIG_DIR changes | Low | Monitor Claude releases, fallback to HOME isolation |
| Sideband parsing failures | Low | MCP as primary, sideband as enhancement |

## Open Questions (Deferred)

- Scrollback buffer management strategy
- Mouse support requirements
- Multi-window vs single-window default

## References

- [Context Engineering Methodology](./CONTEXT_ENGINEERING_METHODOLOGY.md)
- [Deep Research Prompt](./DEEP_RESEARCH_PROMPT.md)
- [Research Synthesis](./research/SYNTHESIS.md)
- [Architecture Documentation](./architecture/)
