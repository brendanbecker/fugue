# fugue Research Synthesis

> Cross-source unified analysis from Claude, Gemini, and ChatGPT research documents
> Generated: 2026-01-07 | Stage 3 Output

---

## Executive Summary

Three independent AI research sessions converged on a **portable-pty + terminal parser + ratatui** core stack with a **client-server architecture** for session persistence. Claude Code integration requires PTY-based state detection or `stream-json` output parsing, with session resume via `claude --resume <session_id>`. Key architectural decisions remain around terminal parser choice (vt100 vs alacritty_terminal) and Claude communication protocol (MCP server vs XML sideband).

---

## 1. Core Technology Stack

### 1.1 Consensus Crates

| Crate | Version | Purpose | Agreement Level |
|-------|---------|---------|-----------------|
| `portable-pty` | 0.9 | PTY management | **Universal** (3/3) |
| `ratatui` | 0.29 | TUI rendering | **Universal** (3/3) |
| `crossterm` | 0.28 | Terminal backend | **Strong** (2/3 explicit) |
| `tokio` | 1.x | Async runtime | **Universal** (3/3) |
| `notify` | latest | File watching | **Universal** (3/3) |
| `serde` | latest | Serialization | **Universal** (3/3) |

### 1.2 Contested: Terminal Parser

| Crate | Advocates | Strengths | Weaknesses |
|-------|-----------|-----------|------------|
| `vt100` | Claude, ChatGPT | Simple API, `contents_diff()` for efficient updates, lighter | Weaker alternate screen buffer handling |
| `alacritty_terminal` | Gemini | Better edge case handling, TrueColor, alternate screen | Heavier, requires adapter pattern |

**Resolution**: Start with `vt100` for simplicity; benchmark with Claude's actual output. Fall back to `alacritty_terminal` if vt100 mishandles escape sequences.

### 1.3 Supporting Crates

| Crate | Purpose | Source |
|-------|---------|--------|
| `tui-term` | vt100→Ratatui widget | Claude, ChatGPT |
| `bincode` | Fast state serialization | Claude |
| `okaywal` | Write-ahead log | Claude |
| `arc_swap` | Lock-free config swapping | Gemini |
| `notify-debouncer-full` | Event debouncing | Gemini |
| `serde_valid` | Config validation | Claude |
| `mcp-rust-sdk` (rmcp) | MCP server | Gemini |
| `ractor` | Actor supervision | Gemini |
| `cgroups-rs` | Resource limits | Claude |

---

## 2. Architecture Decisions

### 2.1 Client-Server Model (Unanimous)

All three sources recommend tmux-style daemon separation:

```
┌─────────────────────────────────────────────────┐
│                  fugue-server                    │
│  ┌─────────┐  ┌─────────┐  ┌─────────┐         │
│  │  PTY 1  │  │  PTY 2  │  │  PTY N  │  ...    │
│  │ (Claude)│  │ (Shell) │  │         │         │
│  └────┬────┘  └────┬────┘  └────┬────┘         │
│       │            │            │               │
│       └────────────┴────────────┘               │
│                    │                            │
│              Unix Socket                        │
└────────────────────┼────────────────────────────┘
                     │
┌────────────────────┼────────────────────────────┐
│                fugue-client                      │
│            ┌───────┴───────┐                    │
│            │   Ratatui UI  │                    │
│            └───────────────┘                    │
└─────────────────────────────────────────────────┘
```

**Benefits**:
- UI crash doesn't kill sessions
- Detach/reattach capability
- Multiple clients possible
- Server can run as systemd service

### 2.2 Crate Structure

Recommended workspace layout (from all sources):

```
fugue/
├── fugue-client/     # UI and user interaction
├── fugue-server/     # PTY management, session state
├── fugue-protocol/   # IPC message definitions
└── fugue-utils/      # Shared utilities
```

### 2.3 Session Hierarchy

```
Session
└── Window
    └── Pane (terminal instance)
        ├── type: Claude | Shell
        ├── PTY handle
        ├── vt100 parser
        └── metadata (cwd, session_id, etc.)
```

---

## 3. Claude Code Integration

### 3.1 State Detection Methods

| Method | Approach | Reliability | Source |
|--------|----------|-------------|--------|
| PTY monitoring | Parse carriage-return + spinner patterns | Medium | All |
| stream-json | Parse JSON events for status | High | All |
| Visual telemetry | Detect "Channelling", "Synthesizing" text | Medium | Gemini |

**Recommended**: Use `--output-format stream-json` for managed Claude panes when orchestrating; fall back to PTY monitoring for interactive sessions.

### 3.2 Session Resume

All sources confirm:
```bash
claude --resume <session_id>   # Resume specific session
claude -c                      # Resume most recent in directory
claude --fork-session          # Branch from existing
```

Session IDs are UUIDs stored in `~/.claude/projects/[encoded-path]/[uuid].jsonl`.

### 3.3 Concurrent Session Isolation

**Problem**: Claude writes to `~/.claude.json` at ~1.5 writes/sec, causing conflicts with concurrent instances.

**Solutions** (from ChatGPT):
1. Set separate `HOME` directory per Claude instance
2. Use `--session-id` flag to specify unique IDs
3. Use `CLAUDE_CONFIG_DIR` environment variable

### 3.4 Communication Protocol

**Option A: MCP Server** (Gemini recommendation)
- fugue exposes tools via `mcp-rust-sdk`
- Claude calls `create_pane()`, `read_pane_output()`, `list_panes()`
- Deterministic, structured interaction

**Option B: XML Sideband** (Claude recommendation)
- Define protocol in SKILL.md/CLAUDE.md
- Parse `<fugue:spawn>`, `<fugue:input>`, `<fugue:control>` from stream
- Hides control messages from display
- 95-98% compliance rate

**Resolution**: Implement both. MCP for formal orchestration, sideband for lightweight interaction.

---

## 4. Crash Recovery Strategy

### 4.1 Hybrid Persistence (Unanimous)

```
┌────────────────────────────────────────────────┐
│              Persistence Layer                  │
├────────────────────────────────────────────────┤
│  Checkpoints (every 30-60s)                    │
│  ├── Layout topology                           │
│  ├── Pane metadata (type, command, cwd)        │
│  ├── Claude session IDs                        │
│  └── Screen snapshots (optional)               │
├────────────────────────────────────────────────┤
│  Write-Ahead Log (continuous)                  │
│  ├── Output chunks since last checkpoint       │
│  └── Trimmed after checkpoint                  │
└────────────────────────────────────────────────┘
```

### 4.2 Recovery Flow

1. On startup, check for crash state file (`~/.fugue/last_session.json`)
2. For Claude panes: `claude --resume <saved_session_id>`
3. For shell panes: Start fresh, display saved screen as "ghost image"
4. Replay WAL tail after checkpoint

### 4.3 Atomic Writes

All sources emphasize: Write to temp file, rename atomically.

```rust
// Recommended pattern
let temp_path = state_path.with_extension("tmp");
std::fs::write(&temp_path, serialized)?;
std::fs::rename(&temp_path, &state_path)?;
```

---

## 5. Configuration Hot-Reload

### 5.1 Implementation Pattern

```rust
// File watching (all sources)
use notify::{RecommendedWatcher, RecursiveMode, Watcher};

// Watch directory, not file (handles atomic renames)
watcher.watch(config_dir, RecursiveMode::NonRecursive)?;

// Debounce: 50-100ms
// Event types: Create (atomic rename), Modify
```

### 5.2 Change Categories

| Category | Examples | Action |
|----------|----------|--------|
| Hot-reloadable | Colors, keybindings, status bar | Apply immediately |
| Restart-required | Terminal config, shell | Document, warn user |
| Invalid | Parse errors, conflicts | Reject, keep previous |

### 5.3 Lock-Free Access (Gemini)

```rust
use arc_swap::ArcSwap;

static CONFIG: Lazy<ArcSwap<AppConfig>> = Lazy::new(|| {
    ArcSwap::from_pointee(load_config())
});

// Reading (lock-free, critical for 60fps)
let cfg = CONFIG.load();

// Writing (atomic swap)
CONFIG.store(Arc::new(new_config));
```

---

## 6. Recursion Control

### 6.1 Depth Limiting

Environment variable tracking:
```rust
const MAX_DEPTH: u32 = 5;

fn spawn_child() {
    let current = std::env::var("FUGUE_SESSION_DEPTH")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);

    if current >= MAX_DEPTH {
        return Err("Maximum session depth exceeded");
    }

    cmd.env("FUGUE_SESSION_DEPTH", (current + 1).to_string());
}
```

### 6.2 Supervision Tree

```
Root Supervisor
├── Session Supervisor (per session)
│   ├── Pane Actor (Claude) [--resume on crash, max 3 restarts]
│   ├── Pane Actor (Shell)  [restart fresh]
│   └── Pane Actor (...)
└── Config Watcher
```

**Crash policies**:
- `one_for_one`: Restart only failed pane
- Restart intensity: 3 per 60 seconds
- Exponential backoff to prevent CPU thrashing

### 6.3 Resource Limits (Claude)

Via `cgroups-rs`:
- Memory: 2GB per session
- CPU: 50% limit
- Processes: 100 per session

---

## 7. Recommended Defaults

| Setting | Value | Source |
|---------|-------|--------|
| Max depth | 5 | Claude |
| Concurrent sessions | 8-16 | Claude |
| Session timeout | 300s | Claude |
| Restart intensity | 3 per 60s | Claude, Gemini |
| Memory per session | 2GB | Claude |
| Checkpoint interval | 30-60s | Claude |
| Debounce delay | 50ms | Claude |
| Render throttle | 16ms (60fps) | Claude |
| Parser timeout | 5s | Claude |

---

## 8. Open Questions for Architecture Phase

### High Priority
1. **Terminal parser**: Final decision after benchmarking vt100 vs alacritty_terminal with Claude output
2. **MCP vs Sideband**: Which is primary communication channel?
3. **Concurrent session handling**: HOME directory isolation vs shared state?

### Medium Priority
4. **Scrollback management**: Implement copy-mode like tmux? Cap lines per pane?
5. **Mouse support**: Needed for clickable links or text editors?
6. **Multi-window**: Support multiple named sessions like tmux?

### Lower Priority
7. **Unicode width**: Verify CJK/emoji alignment handling
8. **Status bar content**: What Claude state indicators to show?
9. **Default startup**: Auto-launch Claude pane?

---

## 9. Implementation Priority

### Phase 1: Foundation
1. Client-server with Unix socket IPC
2. `portable-pty` + `vt100` + `ratatui` stack
3. Basic pane splits (horizontal/vertical)
4. Detach/attach functionality

### Phase 2: Claude Awareness
1. Claude state detection (PTY or stream-json)
2. Session ID capture and storage
3. `--resume` on crash recovery
4. Basic SKILL.md sideband parsing

### Phase 3: Robustness
1. Hybrid checkpoint + WAL persistence
2. Full supervision tree
3. MCP server integration
4. Hot-reload configuration
5. cgroups resource limits

---

## 10. Key File References

For detailed information, consult:

| Topic | File | Section |
|-------|------|---------|
| PTY patterns | `claude_metadata.json` | `code_blocks[0-1]` |
| Terminal parsing comparison | `gemini_metadata.json` | `tables.parser_comparison` |
| Claude CLI flags | `chatgpt_metadata.json` | `claude_code_detection.cli_flags` |
| Session storage | `chatgpt_abstracts.md` | Section 2 |
| Crash recovery | `claude_abstracts.md` | Section 3 |
| ArcSwap pattern | `gemini_metadata.json` | `code_blocks.arcswap_config` |
| Supervision strategies | `claude_section_map.md` | Section 7 |

---

*This synthesis document consolidates ~25,000 tokens of research into actionable architectural guidance for Stage 4.*
