# fugue

**Claude Code-aware terminal multiplexer**

A terminal multiplexer built in Rust that understands Claude Code. Unlike tmux, fugue knows when Claude is thinking, waiting, or ready for input—and can be controlled by Claude itself via MCP.

## Quick Links

| Resource | Description |
|----------|-------------|
| [Quickstart Guide](docs/QUICKSTART.md) | Get up and running in 5 minutes |
| [Architecture](docs/architecture/ARCHITECTURE.md) | System design and crate structure |
| [Claude Integration](docs/architecture/CLAUDE_INTEGRATION.md) | MCP tools, sideband protocol, state detection |
| [Configuration](docs/architecture/CONFIGURATION.md) | Config file options and hot-reload |
| [Remote Access](docs/REMOTE_ACCESS.md) | Secure remote peering via SSH tunneling |
| [Troubleshooting](docs/TROUBLESHOOTING.md) | Common issues and solutions |

## Why fugue?

**The Problem**: tmux doesn't know what's running inside it. When Claude Code runs in a tmux pane, tmux sees opaque byte streams. It cannot:

- Detect Claude's state (thinking, waiting, complete)
- Recover crashed Claude sessions with `--resume`
- Let Claude orchestrate sub-agent spawning
- Provide Claude-aware notifications

**The Solution**: fugue is a Claude Code-aware terminal multiplexer. It understands Claude's state, can detect activity patterns, and enables intelligent session management including crash recovery.

## Features

### What Makes fugue Unique

| Feature | Description |
|---------|-------------|
| **Claude State Detection** | Automatically detects Claude's state: Idle, Thinking, ToolUse, Streaming, Complete |
| **Session Isolation** | Each Claude pane gets its own `CLAUDE_CONFIG_DIR`, preventing conflicts between concurrent instances |
| **Auto-Resume** | Claude conversations persist across server restarts and auto-resume |
| **MCP Integration** | Claude can control fugue via 30 MCP tools—create panes, run commands, orchestrate agents |
| **Sideband Protocol** | Claude can emit `<fugue:spawn>` commands in output to control the multiplexer |

### tmux-Compatible Experience

- **Auto-start**: Just run `fugue`, server starts automatically
- **Familiar keybinds**: `Ctrl+b` prefix, `c` creates window, `%/"` splits, `d` detaches
- **Sessions/Windows/Panes**: Same hierarchy you know from tmux
- **Mouse scroll**: Scroll through scrollback with mouse wheel
- **Configurable**: Hot-reload config, customizable keybinds

### Persistence & Recovery

- **WAL-based crash recovery**: Sessions, windows, panes, and scrollback survive server crashes
- **Session persistence**: Detach and reattach anytime—processes keep running
- **Claude auto-resume**: Claude conversations resume automatically after recovery

## Installation

```bash
# Build
cargo build --release

# Run (auto-starts server)
./target/release/fugue

# Run with custom command
./target/release/fugue bash
./target/release/fugue claude --resume
```

## Quick Start

1. **Run fugue**: `./target/release/fugue`
2. **Create session**: Press `n` at the session picker
3. **Split pane**: `Ctrl+b %` (vertical) or `Ctrl+b "` (horizontal)
4. **Navigate panes**: `Ctrl+b h/j/k/l` or arrow keys
5. **Detach**: `Ctrl+b d` (session keeps running)
6. **Reattach**: Run `fugue` again

## Configuration

Config file: `~/.config/fugue/config.toml`

```toml
[general]
# Auto-launch Claude in every new session
default_command = "claude"
```

See [Configuration Guide](docs/architecture/CONFIGURATION.md) for all options.

## Keybinds

### Session Selection (on startup)

| Key | Action |
|-----|--------|
| `n` | Create new session |
| `Ctrl+D` | Delete selected session |
| `Enter` | Attach to session |
| `j/k` or arrows | Navigate |
| `q` | Quit |

### Prefix Mode (`Ctrl+b`, then...)

| Key | Action |
|-----|--------|
| **Windows** ||
| `c` | Create window |
| `&` | Close window |
| `n/p` | Next/prev window |
| `0-9` | Select window by number |
| **Panes** ||
| `%` | Split vertical |
| `"` | Split horizontal |
| `x` | Close pane |
| `o` | Next pane (cycle) |
| `h/j/k/l` | Vim-style navigation |
| `z` | Zoom pane (fullscreen) |
| **Session** ||
| `s` | Session picker |
| `d` | Detach |

### Quick Navigation (no prefix)

| Key | Action |
|-----|--------|
| `Ctrl+PageUp/Down` | Switch windows |
| `Ctrl+Shift+PageUp/Down` | Switch panes |

## MCP Integration

Claude Code can control fugue sessions via MCP. Add to `~/.claude/mcp.json`:

```json
{
  "mcpServers": {
    "fugue": {
      "command": "/path/to/fugue-server",
      "args": ["mcp-bridge"]
    }
  }
}
```

### Available MCP Tools (30 total)

| Category | Tools |
|----------|-------|
| **Sessions** | `fugue_list_sessions`, `fugue_create_session`, `fugue_rename_session`, `fugue_select_session`, `fugue_kill_session` |
| **Windows** | `fugue_list_windows`, `fugue_create_window`, `fugue_select_window`, `fugue_rename_window` |
| **Panes** | `fugue_list_panes`, `fugue_create_pane`, `fugue_close_pane`, `fugue_focus_pane`, `fugue_rename_pane` |
| **I/O** | `fugue_read_pane`, `fugue_send_input`, `fugue_get_status` |
| **Layouts** | `fugue_create_layout`, `fugue_split_pane`, `fugue_resize_pane` |
| **Environment** | `fugue_set_environment`, `fugue_get_environment` |
| **Metadata** | `fugue_set_metadata`, `fugue_get_metadata` |
| **Orchestration** | `fugue_send_orchestration`, `fugue_set_tags`, `fugue_get_tags`, `fugue_report_status`, `fugue_request_help`, `fugue_broadcast` |

### Declarative Layouts

Create complex layouts with a single MCP call:

```json
{
  "layout": {
    "direction": "horizontal",
    "splits": [
      {"ratio": 0.6, "layout": {"pane": {"command": "vim"}}},
      {"ratio": 0.4, "layout": {
        "direction": "vertical",
        "splits": [
          {"ratio": 0.5, "layout": {"pane": {"command": "claude"}}},
          {"ratio": 0.5, "layout": {"pane": {"command": "cargo watch"}}}
        ]
      }}
    ]
  }
}
```

See [Claude Integration](docs/architecture/CLAUDE_INTEGRATION.md) for full MCP and sideband documentation.

## Architecture

```
fugue/
├── fugue-client/      # TUI client (ratatui + crossterm)
├── fugue-server/      # Daemon with PTY management + MCP bridge
├── fugue-protocol/    # Message types and codec (bincode)
├── fugue-session/     # Session/window/pane hierarchy
├── fugue-utils/       # Shared utilities
└── fugue-persistence/ # WAL-based crash recovery
```

**Communication**:
- Client <-> Server: Unix socket (`~/.fugue/fugue.sock`) with bincode-serialized messages
- MCP Bridge: Connects to same daemon, translates JSON-RPC to internal commands

See [Architecture Guide](docs/architecture/ARCHITECTURE.md) for details.

## Development

```bash
# Build
cargo build --release

# Run tests (1,400+ tests)
cargo test --workspace

# Run server manually
./target/release/fugue-server

# Run client
./target/release/fugue
```

## Known Issues

- **`kill -9` corrupts terminal**: SIGKILL cannot be caught. Run `reset` to fix.
- **Text selection**: Use `Shift+click` for native terminal selection (regular mouse is captured for scrollback)
- **Scrollback limit**: 1000 lines by default to maintain responsiveness
- **Legacy state**: Clear `~/.local/share/.fugue/state/` if upgrading from earlier versions with stale sessions

See [Troubleshooting](docs/TROUBLESHOOTING.md) for more help.

## Prior Art

| Project | Description |
|---------|-------------|
| **tmux** | The standard, but process-unaware |
| **Zellij** | Modern Rust multiplexer with plugins |
| **Wezterm** | GPU-accelerated terminal with mux mode |

## Project Status

- **Stage**: MVP Functional (Stability Release 0.1.1)
- **Tests**: 1,600+ passing
- **Bugs**: 41 tracked, 34 resolved or deprecated
- **Features**: 44 complete, 2 in backlog

## License

MIT License. See [LICENSE.md](LICENSE.md) for details.
