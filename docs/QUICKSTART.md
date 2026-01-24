# fugue Quickstart Guide

Get up and running with fugue in 5 minutes.

## Prerequisites

- Rust toolchain (for building)
- Claude Code CLI (optional, for Claude integration)

## Build

```bash
cd /path/to/fugue
cargo build --release
```

This produces two binaries:
- `target/release/fugue` - The client (what you run)
- `target/release/fugue-server` - The daemon (starts automatically)

## First Run

```bash
./target/release/fugue
```

You'll see the **session picker**:

```
┌─────────────────────────────────────┐
│         fugue Sessions              │
├─────────────────────────────────────┤
│   (no sessions)                     │
│                                     │
│   Press 'n' to create a session     │
│   Press 'q' to quit                 │
└─────────────────────────────────────┘
```

Press `n` to create your first session.

## Basic Navigation

### Session Picker Keys

| Key | Action |
|-----|--------|
| `n` | Create new session |
| `Enter` | Attach to selected session |
| `Ctrl+D` | Delete selected session |
| `j/k` or arrows | Navigate list |
| `q` | Quit |

### Inside a Session

All commands use the **prefix key** `Ctrl+b` (same as tmux).

| Keys | Action |
|------|--------|
| `Ctrl+b c` | Create new window |
| `Ctrl+b %` | Split pane vertically |
| `Ctrl+b "` | Split pane horizontally |
| `Ctrl+b h/j/k/l` | Navigate panes (vim-style) |
| `Ctrl+b n/p` | Next/previous window |
| `Ctrl+b d` | Detach (session keeps running) |
| `Ctrl+b s` | Return to session picker |

## Example Workflow

1. **Start fugue**: `./target/release/fugue`
2. **Create session**: Press `n`
3. **Split for editor + terminal**:
   - `Ctrl+b %` (vertical split)
   - Left pane: `vim myfile.rs`
   - `Ctrl+b l` (move to right pane)
   - Right pane: `cargo watch -x check`
4. **Detach**: `Ctrl+b d`
5. **Later, reattach**: `./target/release/fugue` and select your session

## Running Claude

### Option 1: Launch Claude in a session

```bash
# Start fugue with Claude as the default command
./target/release/fugue claude
```

### Option 2: Configure default command

Create `~/.config/fugue/config.toml`:

```toml
[general]
default_command = "claude"
```

Now every new session starts Claude automatically.

### Option 3: Resume a Claude session

```bash
./target/release/fugue claude --resume
```

This resumes your most recent Claude conversation.

## MCP Integration (Claude controls fugue)

To let Claude create panes, run commands, and read output:

1. Add to `~/.claude/mcp.json`:

```json
{
  "mcpServers": {
    "fugue": {
      "command": "/absolute/path/to/fugue-server",
      "args": ["mcp-bridge"]
    }
  }
}
```

2. Restart Claude Code

3. Claude now has access to 18 MCP tools for controlling fugue

Example prompt: "Create a split layout with vim on the left and run the tests on the right"

## Tips

### Quick window/pane switching (no prefix)

| Keys | Action |
|------|--------|
| `Ctrl+PageUp/Down` | Switch windows |
| `Ctrl+Shift+PageUp/Down` | Switch panes |

### Mouse scrollback

Scroll your mouse wheel to browse history. Use `Shift+click` for native text selection.

### Session persistence

Sessions survive:
- Detaching (`Ctrl+b d`)
- Closing the terminal window
- Server restarts
- System reboots (if server is a systemd service)

### Running multiple Claudes

fugue automatically isolates each Claude instance with its own config directory. No conflicts when running multiple Claude panes.

## Next Steps

- [Full Keybind Reference](../README.md#keybinds)
- [Configuration Options](architecture/CONFIGURATION.md)
- [Claude Integration Details](architecture/CLAUDE_INTEGRATION.md)
- [Troubleshooting](TROUBLESHOOTING.md)
