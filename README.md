# ccmux

**Claude Code-aware terminal multiplexer**

## Status

Early development, spec-driven. Currently in Stage 5 (Implementation Planning) of the [Context Engineering Methodology](https://github.com/brendanbecker/featmgmt/blob/master/CONTEXT_ENGINEERING_METHODOLOGY.md). Architecture documentation is complete.

## The Problem

tmux doesn't know what's running inside it.

When you run Claude Code in a tmux pane, tmux sees it as just another process spewing bytes. It has no idea whether Claude is:
- Thinking deeply about a complex problem
- Waiting for your input
- Done and ready for the next task
- About to spawn a sub-agent
- Crashed and needs recovery

This blindness means you can't build intelligent workflows. You can't automatically split a pane when Claude spawns a sub-agent. You can't get notifications when Claude finishes a long task. You can't recover a crashed Claude session with `--resume` because tmux doesn't know there's a session to resume.

## The Vision

ccmux is a terminal multiplexer that understands Claude Code.

**Claude-aware session management:**
- Detect Claude Code state (thinking, waiting, complete, crashed)
- Visual indicators for Claude's current activity
- Automatic crash recovery with `claude --resume`
- Session tree visualization for orchestrated agent hierarchies

**Intelligent pane orchestration:**
- Claude can request new panes via structured output (e.g., `<ccmux:spawn>`)
- Parent sessions notified of child completion
- Recursion depth limits to prevent runaway spawns
- Optional: Claude can read from sibling panes

**Crash resilience:**
- Continuous session state persistence
- Survive terminal crashes, SSH disconnects, system reboots
- Resume exactly where you left off

**Modern terminal multiplexer features:**
- Multiple panes with flexible layouts
- Hot-reload configuration
- Keyboard-driven navigation (vim-style)
- Scrollback and copy mode

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                         ccmux                               │
├─────────────────────────────────────────────────────────────┤
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────┐ │
│  │   PTY Manager   │  │  State Parser   │  │   Renderer  │ │
│  │  (portable-pty) │  │ (Claude detect) │  │  (ratatui)  │ │
│  └────────┬────────┘  └────────┬────────┘  └──────┬──────┘ │
│           │                    │                   │        │
│           ▼                    ▼                   ▼        │
│  ┌─────────────────────────────────────────────────────────┐│
│  │              Session State Manager                      ││
│  │  - Pane tree structure                                  ││
│  │  - Claude session metadata                              ││
│  │  - Crash recovery checkpoints                           ││
│  └─────────────────────────────────────────────────────────┘│
│                              │                              │
│                              ▼                              │
│  ┌─────────────────────────────────────────────────────────┐│
│  │              Persistent State (JSON)                    ││
│  │              ~/.ccmux/sessions/                         ││
│  └─────────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────┘
```

## Project Structure

```
ccmux/
├── Cargo.toml
├── README.md
├── docs/
│   ├── research/              # Deep research outputs
│   ├── ARCHITECTURE.md        # Generated after research
│   ├── PROJECT_SUMMARY.md     # Generated after research
│   └── DEEP_RESEARCH_PROMPT.md
├── feature-management/
│   ├── features/
│   └── bugs/
├── src/
│   └── main.rs                # Placeholder
└── .ccmux/                    # Example runtime state
    ├── settings.default.json
    └── sessions/
```

## Development

```bash
# Build
cargo build

# Run (currently just prints status)
cargo run

# Test
cargo test
```

## Prior Art

- **tmux** - The standard, but process-unaware
- **Zellij** - Modern Rust multiplexer with plugins
- **Wezterm** - GPU-accelerated terminal with mux mode
- **Alacritty** - Fast terminal (not a mux, but `alacritty_terminal` crate is useful)

## License

This project is licensed under the MIT License. See [LICENSE.md](LICENSE.md) for details.
