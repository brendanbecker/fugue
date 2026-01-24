# Claude Code Integration

> Detection, communication, and session management for Claude Code panes

## Overview

fugue provides first-class support for Claude Code by detecting its state, managing session identifiers for crash recovery, and enabling structured communication between Claude and the multiplexer.

## State Detection Methods

### Method 1: PTY Output Monitoring (Default)

Parse Claude's terminal output for characteristic patterns.

**Indicators**:
| Pattern | State | Description |
|---------|-------|-------------|
| Spinner + `Thinking` | Thinking | Claude is processing |
| Spinner + `Channelling` | Working | Claude is executing tools |
| `>` prompt | Idle | Waiting for user input |
| Tool output streaming | Active | Tool execution in progress |
| Session end message | Complete | Task finished |

**Implementation**:
```rust
pub enum ClaudeState {
    Unknown,
    Idle,           // Waiting for input
    Thinking,       // Processing request
    ToolExecution,  // Running a tool
    Streaming,      // Outputting response
    Complete,       // Task finished
    Crashed,        // Process died unexpectedly
}

impl ClaudeDetector {
    pub fn analyze_output(&mut self, chunk: &[u8]) -> Option<ClaudeState> {
        let text = String::from_utf8_lossy(chunk);

        // Check for spinner patterns (carriage return + overwrite)
        if text.contains("\r") && text.contains("Thinking") {
            return Some(ClaudeState::Thinking);
        }

        // Check for prompt
        if text.ends_with("> ") || text.ends_with("❯ ") {
            return Some(ClaudeState::Idle);
        }

        // Check for tool execution markers
        if text.contains("Running:") || text.contains("Executing:") {
            return Some(ClaudeState::ToolExecution);
        }

        None // State unchanged
    }
}
```

**Reliability**: Medium - depends on Claude's output format remaining stable

### Method 2: Stream JSON Parsing (Orchestrated Mode)

When launching Claude with `--output-format stream-json`, parse JSON events.

**Usage**:
```bash
claude --output-format stream-json --session-id <id>
```

**Event Types**:
```json
{"type": "start", "session_id": "abc123"}
{"type": "thinking", "content": "..."}
{"type": "tool_use", "tool": "Read", "input": {...}}
{"type": "tool_result", "result": {...}}
{"type": "text", "content": "..."}
{"type": "end", "session_id": "abc123"}
```

**Implementation**:
```rust
pub struct StreamJsonDetector {
    buffer: String,
}

impl StreamJsonDetector {
    pub fn feed(&mut self, chunk: &[u8]) -> Vec<ClaudeEvent> {
        self.buffer.push_str(&String::from_utf8_lossy(chunk));

        let mut events = Vec::new();
        for line in self.buffer.lines() {
            if let Ok(event) = serde_json::from_str::<ClaudeEvent>(line) {
                events.push(event);
            }
        }

        // Keep incomplete last line
        if let Some(idx) = self.buffer.rfind('\n') {
            self.buffer = self.buffer[idx+1..].to_string();
        }

        events
    }
}
```

**Reliability**: High - structured, documented format

### Method 3: Visual Telemetry (Fallback)

Detect Claude-specific UI elements in the terminal buffer.

**Patterns**:
- "Claude Code" in title/header
- Characteristic spinner characters
- Tool output formatting

**Reliability**: Medium - UI may change between versions

## Session Management

### Session ID Discovery

Claude stores session IDs in predictable locations:

```
~/.claude/
├── .claude.json              # Active session state
└── projects/
    └── <encoded-path>/       # Per-project sessions
        ├── <uuid>.jsonl      # Session transcript
        └── ...
```

**Path encoding**: Project paths are base64-encoded to create directory names.

**Implementation**:
```rust
pub fn find_claude_session(cwd: &Path) -> Option<SessionId> {
    let encoded = base64_encode(cwd.to_string_lossy());
    let session_dir = dirs::home_dir()?
        .join(".claude")
        .join("projects")
        .join(encoded);

    // Find most recent session file
    let mut sessions: Vec<_> = std::fs::read_dir(&session_dir)
        .ok()?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension() == Some("jsonl".as_ref()))
        .collect();

    sessions.sort_by_key(|e| e.metadata().ok()?.modified().ok());

    sessions.last()
        .and_then(|e| e.path().file_stem()?.to_str().map(|s| s.to_string()))
}
```

### Session Resume

Resume a Claude session after crash:

```bash
# Resume specific session
claude --resume <session_id>

# Resume most recent in current directory
claude -c

# Fork from existing session
claude --fork-session <session_id>
```

**fugue integration**:
```rust
impl Pane {
    pub fn spawn_claude(&mut self, resume_id: Option<&str>) -> Result<()> {
        let mut cmd = std::process::Command::new("claude");

        if let Some(id) = resume_id {
            cmd.arg("--resume").arg(id);
        }

        // Set isolation environment
        let config_dir = self.config_dir();
        cmd.env("CLAUDE_CONFIG_DIR", &config_dir);

        self.pty.spawn(cmd)?;
        Ok(())
    }
}
```

## Concurrent Session Isolation

### The Problem

Claude writes to `~/.claude.json` at approximately 1.5 writes/second during active use. Multiple Claude instances cause conflicts:

```
Instance A writes: { "current_session": "abc" }
Instance B writes: { "current_session": "xyz" }  # Overwrites A
Instance A reads:  { "current_session": "xyz" }  # Wrong session!
```

### Solution: CLAUDE_CONFIG_DIR

Each Claude pane gets its own config directory:

```
~/.fugue/claude-configs/
├── pane-<uuid1>/
│   └── .claude.json
├── pane-<uuid2>/
│   └── .claude.json
└── ...
```

**Implementation**:
```rust
impl Pane {
    pub fn config_dir(&self) -> PathBuf {
        let state_dir = fugue_utils::state_dir();
        state_dir
            .join("claude-configs")
            .join(format!("pane-{}", self.id))
    }

    pub fn spawn_with_isolation(&mut self) -> Result<()> {
        let config_dir = self.config_dir();
        std::fs::create_dir_all(&config_dir)?;

        let mut cmd = self.build_command();
        cmd.env("CLAUDE_CONFIG_DIR", &config_dir);

        self.pty.spawn(cmd)?;
        Ok(())
    }
}
```

See [ADR-003](./ADR/003-session-isolation.md) for the full decision rationale.

## Communication Protocols

### Protocol A: MCP Server

fugue exposes an MCP (Model Context Protocol) server via the `mcp-bridge` subcommand. This allows Claude to control fugue sessions programmatically.

**MCP Configuration** (add to `~/.claude/mcp.json`):
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

**Tools provided** (30 total):

| Category | Tool | Description |
|----------|------|-------------|
| **Sessions** | `fugue_list_sessions` | List all sessions with metadata |
| | `fugue_create_session` | Create a new session |
| | `fugue_rename_session` | Rename a session for easier identification |
| | `fugue_select_session` | Switch to a different session |
| | `fugue_kill_session` | Destroy a session |
| **Windows** | `fugue_list_windows` | List windows in a session |
| | `fugue_create_window` | Create a new window |
| | `fugue_select_window` | Switch to a different window |
| | `fugue_rename_window` | Rename a window |
| **Panes** | `fugue_list_panes` | List all panes with metadata |
| | `fugue_create_pane` | Create a new pane (split) |
| | `fugue_close_pane` | Close a pane |
| | `fugue_focus_pane` | Focus a specific pane |
| | `fugue_rename_pane` | Rename a pane |
| **I/O** | `fugue_read_pane` | Read output buffer from pane |
| | `fugue_send_input` | Send keystrokes to pane (use `\n` for Enter) |
| | `fugue_get_status` | Get pane state (shell, Claude, etc.) |
| **Layouts** | `fugue_create_layout` | Create complex layouts declaratively |
| | `fugue_split_pane` | Split a pane with custom ratio |
| | `fugue_resize_pane` | Resize a pane dynamically |
| **Environment** | `fugue_set_environment` | Set session environment variable |
| | `fugue_get_environment` | Get session environment variables |
| **Metadata** | `fugue_set_metadata` | Set session metadata |
| | `fugue_get_metadata` | Get session metadata |
| **Orchestration** | `fugue_send_orchestration` | Send orchestration message with tag-based routing |
| | `fugue_set_tags` | Add/remove tags on a session |
| | `fugue_get_tags` | Query session tags |
| | `fugue_report_status` | Report status to orchestrator sessions |
| | `fugue_request_help` | Request help from orchestrator |
| | `fugue_broadcast` | Broadcast message to all sessions |

**Example: Create a pane**:
```json
{
  "tool": "fugue_create_pane",
  "input": {
    "direction": "horizontal",
    "command": "npm test"
  }
}
```

**Example: Declarative layout** (via `fugue_create_layout`):
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
          {"ratio": 0.5, "layout": {"pane": {"command": "cargo watch -x check"}}}
        ]
      }}
    ]
  }
}
```

This creates a 60/40 horizontal split with vim on the left, and a vertical split on the right with Claude on top and cargo watch on bottom.

**Example: Send input with Enter**:
```json
{
  "tool": "fugue_send_input",
  "input": {
    "pane_id": "abc-123",
    "input": "ls -la\n"
  }
}
```

**Example: Send orchestration message** (via `fugue_send_orchestration`):
```json
{
  "tool": "fugue_send_orchestration",
  "input": {
    "target": {"tag": "orchestrator"},
    "msg_type": "task.complete",
    "payload": {"feature": "FEAT-048", "status": "done"}
  }
}
```

Target variants:
- `{"tag": "orchestrator"}` - Send to sessions with specific tag
- `{"session": "uuid"}` - Send to specific session by ID
- `{"broadcast": true}` - Send to all sessions
- `{"worktree": "/path"}` - Send to sessions in specific worktree

**Example: Tag a session as orchestrator**:
```json
{
  "tool": "fugue_set_tags",
  "input": {
    "add": ["orchestrator", "primary"]
  }
}
```

**Example: Report status to orchestrator** (convenience tool):
```json
{
  "tool": "fugue_report_status",
  "input": {
    "status": "working",
    "message": "Implementing FEAT-048"
  }
}
```

### Protocol B: XML Sideband

Claude can emit structured commands in its output that fugue parses:

**Command format**:
```xml
<fugue:spawn direction="vertical" command="cargo build" />
<fugue:input pane="1">ls -la</fugue:input>
<fugue:control action="focus" pane="0" />
```

**Implementation**:
```rust
pub struct SidebandParser {
    buffer: String,
}

impl SidebandParser {
    pub fn parse(&mut self, output: &str) -> (String, Vec<CcmuxCommand>) {
        let mut commands = Vec::new();
        let mut display = String::new();

        let re = regex::Regex::new(r"<fugue:(\w+)([^>]*)(?:>(.*?)</fugue:\1>|/>)").unwrap();

        let mut last_end = 0;
        for cap in re.captures_iter(output) {
            // Append text before this match to display
            display.push_str(&output[last_end..cap.get(0).unwrap().start()]);
            last_end = cap.get(0).unwrap().end();

            // Parse command
            let cmd_type = &cap[1];
            let attrs = &cap[2];
            let content = cap.get(3).map(|m| m.as_str()).unwrap_or("");

            if let Some(cmd) = parse_command(cmd_type, attrs, content) {
                commands.push(cmd);
            }
        }

        display.push_str(&output[last_end..]);
        (display, commands)
    }
}
```

**Sideband commands are hidden from display** - only the non-fugue portions are rendered.

See [ADR-002](./ADR/002-claude-communication.md) for protocol selection rationale.

## Pane Metadata

### Claude Pane Structure

```rust
pub struct ClaudePaneMetadata {
    /// Claude session UUID (for --resume)
    pub session_id: Option<String>,

    /// Current detected state
    pub state: ClaudeState,

    /// Working directory when Claude was launched
    pub cwd: PathBuf,

    /// Isolated config directory
    pub config_dir: PathBuf,

    /// Last state change timestamp
    pub state_changed_at: SystemTime,

    /// Whether this pane was resumed after crash
    pub is_resumed: bool,

    /// Parent Claude pane ID (if spawned by another Claude)
    pub parent_pane: Option<Uuid>,
}
```

### State Persistence

Claude metadata is included in checkpoints:

```rust
pub struct PaneCheckpoint {
    pub id: Uuid,
    pub pane_type: PaneType,
    pub command: String,
    pub cwd: PathBuf,

    // Claude-specific (only if pane_type == Claude)
    pub claude_metadata: Option<ClaudePaneMetadata>,
}
```

## Recursion Control

### Depth Tracking

Prevent infinite Claude spawning:

```rust
const MAX_DEPTH: u32 = 5;

impl SessionManager {
    pub fn spawn_claude_pane(&mut self, parent: Option<Uuid>) -> Result<Pane> {
        // Calculate depth
        let depth = match parent {
            Some(id) => self.get_pane(id)?.depth + 1,
            None => 0,
        };

        if depth >= MAX_DEPTH {
            return Err(CcmuxError::MaxDepthExceeded);
        }

        let mut pane = Pane::new_claude(depth);
        pane.spawn_with_isolation()?;

        Ok(pane)
    }
}
```

### Environment Propagation

Child Claude instances know their depth:

```rust
cmd.env("FUGUE_SESSION_DEPTH", depth.to_string());
cmd.env("FUGUE_PARENT_PANE", parent_id.to_string());
```

## Status Bar Integration

### Claude State Indicators

```
┌─────────────────────────────────────────────────────────────────────┐
│ Pane 0 [Claude: Thinking]  │  Pane 1 [bash]                        │
│                            │                                        │
│ ...                        │ ...                                    │
├────────────────────────────┴────────────────────────────────────────┤
│ Session: work | Claude: 🧠 Thinking | Panes: 2 | 14:32:05          │
└─────────────────────────────────────────────────────────────────────┘
```

**State icons**:
| State | Icon | Description |
|-------|------|-------------|
| Idle | `⏳` | Waiting for input |
| Thinking | `🧠` | Processing |
| Tool Execution | `⚙️` | Running tool |
| Streaming | `📝` | Generating output |
| Complete | `✅` | Task done |
| Crashed | `❌` | Needs recovery |

## Claude Lifecycle Events

### Events Emitted

```rust
pub enum ClaudeEvent {
    /// Claude detected in pane
    Detected { pane_id: Uuid, session_id: String },

    /// State changed
    StateChanged { pane_id: Uuid, old: ClaudeState, new: ClaudeState },

    /// Session ID captured (from output or discovery)
    SessionCaptured { pane_id: Uuid, session_id: String },

    /// Claude process exited
    Exited { pane_id: Uuid, exit_code: i32 },

    /// Claude crashed (non-zero exit)
    Crashed { pane_id: Uuid, session_id: Option<String> },

    /// Child Claude spawned
    ChildSpawned { parent_id: Uuid, child_id: Uuid },
}
```

### Event Handlers

```rust
impl Server {
    async fn handle_claude_event(&mut self, event: ClaudeEvent) {
        match event {
            ClaudeEvent::Crashed { pane_id, session_id } => {
                // Notify clients to show crash indicator
                self.broadcast(ServerMessage::ClaudeState {
                    pane_id,
                    state: ClaudeState::Crashed,
                }).await;

                // Store session_id for potential resume
                if let Some(id) = session_id {
                    self.store_crash_recovery(pane_id, id).await;
                }
            }
            // ... other handlers
        }
    }
}
```

## Related Documents

- [ARCHITECTURE.md](./ARCHITECTURE.md) - System overview
- [PERSISTENCE.md](./PERSISTENCE.md) - Crash recovery details
- [ADR/002-claude-communication.md](./ADR/002-claude-communication.md) - Protocol decision
- [ADR/003-session-isolation.md](./ADR/003-session-isolation.md) - Isolation strategy
