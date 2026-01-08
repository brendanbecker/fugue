# Building ccmux: A Claude Code-Aware Terminal Multiplexer in Rust

The most viable architecture for ccmux combines **portable-pty** for PTY management, **vt100** for terminal state parsing, and **Ratatui** for rendering—all coordinated through a client-server model inspired by Zellij's multi-crate workspace. Claude Code integration requires PTY-based output monitoring (no native IPC exists) combined with **stream-json** output format for structured data, while crash recovery should use a **hybrid checkpoint + WAL strategy** that persists both terminal state and Claude session IDs separately. The critical insight is that Claude Code stores sessions as JSONL files in `~/.claude/projects/`, making session resume via `--resume <session-id>` the primary recovery mechanism after crashes.

---

## Section 1: Terminal emulation requires three layered components

Building terminal emulation in Rust means solving three distinct problems: PTY lifecycle management, escape sequence parsing with state tracking, and efficient rendering to the host terminal. The **portable-pty** crate (from the WezTerm ecosystem) provides the cleanest cross-platform PTY abstraction, while **vt100** offers a purpose-built parser for multiplexer use cases.

### PTY management with portable-pty

The crate handles platform differences automatically—using `/dev/ptmx` on Linux, `posix_openpt` on macOS, and ConPTY on Windows. The key insight is that PTY reads are **blocking operations**, requiring dedicated threads or `spawn_blocking` in async contexts:

```rust
use portable_pty::{CommandBuilder, PtySize, native_pty_system, PtySystem};
use std::sync::mpsc::channel;
use std::thread;

let pty_system = native_pty_system();
let mut pair = pty_system.openpty(PtySize {
    rows: 24, cols: 80, pixel_width: 0, pixel_height: 0,
})?;

let mut cmd = CommandBuilder::new("bash");
cmd.arg("-i");
let _child = pair.slave.spawn_command(cmd)?;
drop(pair.slave);  // Critical: prevents hangs

// Blocking reader in dedicated thread
let (tx, rx) = channel();
let mut reader = pair.master.try_clone_reader()?;
thread::spawn(move || {
    let mut buf = [0u8; 4096];
    while let Ok(n) = reader.read(&mut buf) {
        if n == 0 { break; }
        tx.send(buf[..n].to_vec()).ok();
    }
});

// Resize triggers SIGWINCH in child process
pair.master.resize(PtySize { rows: 30, cols: 120, ..Default::default() })?;
```

The **master end** is your program's handle for reading output and writing input; the **slave end** becomes the child process's controlling terminal. Always drop the slave after spawning to prevent deadlocks.

### Terminal state parsing with vt100

For multiplexer use cases, **vt100** outperforms **alacritty_terminal** due to its `contents_diff()` method that returns minimal escape sequences for efficient screen updates:

```rust
use vt100::Parser;

let mut parser = Parser::new(24, 80, 1000); // rows, cols, scrollback
parser.process(b"Hello \x1b[31mRED\x1b[m World");

let screen = parser.screen();
let cell = screen.cell(0, 6).unwrap();
println!("Char: {}, FG: {:?}", cell.contents(), cell.fgcolor());

// Efficient diffing for rendering
let old_screen = parser.screen().clone();
parser.process(new_data);
let diff = parser.screen().contents_diff(&old_screen);
```

The tradeoff: **alacritty_terminal** provides fuller xterm emulation with damage tracking, kitty keyboard protocol, and hyperlink support, but requires implementing an `EventListener` trait and managing a separate VTE parser. For ccmux, vt100's **simpler API** and built-in diff support make it the recommended choice.

### Rendering at 60fps with Ratatui

The **tui-term** crate provides a `PseudoTerminal` widget that renders vt100 screens directly:

```rust
use ratatui::widgets::{Block, Borders};
use tui_term::widget::PseudoTerminal;

fn render(frame: &mut Frame, parser: &Parser, area: Rect) {
    let term = PseudoTerminal::new(parser.screen())
        .block(Block::default().title("Pane 1").borders(Borders::ALL));
    frame.render_widget(term, area);
}
```

For **10+ panes at 60fps**, three techniques are essential: (1) batch PTY reads before rendering using `try_recv()` loops, (2) throttle renders to 16ms intervals, and (3) leverage Ratatui's automatic double-buffering which only writes changed cells. Zellij and WezTerm both use dedicated screen threads to decouple parsing from rendering.

### Recommended crate dependencies

```toml
[dependencies]
ratatui = "0.29"
crossterm = "0.28"
portable-pty = "0.9"
vt100 = "0.16"
tui-term = "0.2"
tokio = { version = "1", features = ["full"] }
```

---

## Section 2: Claude Code operates as a closed system with workarounds

Claude Code is a **React/Ink-based TUI** with no native IPC socket or API for communicating with running instances. State detection must happen through PTY output monitoring or the `--output-format stream-json` flag for new sessions. Session state persists in `~/.claude/` as JSONL files with UUID-based session IDs.

### Visual state detection patterns

During thinking states, Claude Code displays an **ASCII "flower" spinner animation** with Unicode characters and shimmer effects. The status can be inferred through:

- **Thinking**: Animated spinner with verbs ("Sparkling", "Reticulating")
- **Waiting for input**: Prompt box with `>` or `------` delimiters
- **Permission mode**: Shows `⏵⏵ accept edits on` (purple) or `⏸ plan mode on`

The **AgentAPI project** (github.com/coder/agentapi) demonstrates practical PTY-based state detection, exposing `GET /status` that returns `"stable"` or `"running"` by monitoring output changes.

### Structured output via stream-json

For programmatic interaction, use print mode with streaming JSON:

```bash
claude -p --output-format stream-json "query"
```

This emits NDJSON events for every token, tool call, and result. Chaining is possible:

```bash
claude -p --output-format stream-json "task 1" | \
claude -p --input-format stream-json --output-format stream-json "task 2"
```

The JSON output structure includes a `result.content` array with text blocks, plus `session_id` in system init messages when using the SDK.

### Session storage and resume mechanics

Sessions are stored in a predictable directory structure:

```
~/.claude/
├── projects/
│   └── [encoded-directory-paths]/     # e.g., -home-user-myproject
│       ├── [session-uuid].jsonl       # Full conversation history
│       └── [summary-uuid].jsonl       # Summaries for long sessions
├── history.jsonl                      # Session metadata index
├── settings.json                      # User preferences
└── CLAUDE.md                          # Global memory instructions
```

Resume commands:
- `claude --continue` or `-c`: Most recent session in current directory
- `claude --resume <session-id>` or `-r`: Specific UUID (partial matches work)
- `claude --session-id "uuid"`: Start with explicit ID for later resumption

The **CLAUDE_CONFIG_DIR** environment variable can relocate the entire `.claude` directory, useful for containerized deployments.

### Key environment variables for integration

| Variable | Purpose |
|----------|---------|
| `ANTHROPIC_API_KEY` | API authentication |
| `CLAUDE_CONFIG_DIR` | Relocate ~/.claude directory |
| `CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC` | Disable telemetry |
| `FORCE_COLOR=0` | Disable colored output for parsing |
| `MAX_THINKING_TOKENS` | Control thinking budget |

---

## Section 3: Crash recovery demands hybrid persistence strategies

Terminal state recovery requires saving screen buffers, cursor position, terminal modes, and scrollback—but running processes cannot be truly recovered without CRIU (Checkpoint/Restore In Userspace), which is impractical for typical use. The solution is **persisting enough state to recreate sessions**, following Zellij's approach of serializing commands for re-execution.

### What constitutes terminal state

```rust
#[derive(Serialize, Deserialize)]
pub struct TerminalState {
    pub primary_screen: ScreenBuffer,
    pub alternate_screen: Option<ScreenBuffer>,
    pub scrollback: Vec<Line>,
    pub cursor: CursorState,
    pub modes: TerminalModes,  // bracketed_paste, mouse_tracking, etc.
    pub saved_cursor: Option<CursorState>,  // DECSC/DECRC
}

#[derive(Serialize, Deserialize)]
pub struct CursorState {
    pub x: usize,
    pub y: usize,
    pub visible: bool,
    pub shape: CursorShape,
}
```

For serialization, **bincode** offers the best speed/size ratio (~24B overhead, ~0.1μs serialize), while JSON/KDL provide human-readable exports for debugging.

### Hybrid checkpoint + WAL strategy

Pure continuous persistence causes severe I/O overhead (terminals receive thousands of updates/second). Pure checkpointing loses up to `interval` seconds of data. The recommended hybrid approach:

1. **WAL for operations** between checkpoints using **okaywal** crate
2. **Periodic checkpoints** every 30-60 seconds
3. **On recovery**: Load checkpoint, replay WAL entries since

```rust
use okaywal::{WriteAheadLog, LogManager};

#[derive(Serialize, Deserialize)]
pub enum TerminalOp {
    Output { pane_id: u32, data: Vec<u8> },
    Resize { pane_id: u32, width: u16, height: u16 },
    CursorMove { pane_id: u32, x: u16, y: u16 },
}

// Atomic checkpoint writes prevent corruption
async fn checkpoint(state: &SessionState, path: &Path) -> Result<()> {
    let temp = path.with_extension(".tmp");
    tokio::fs::write(&temp, bincode::serialize(state)?).await?;
    tokio::fs::rename(&temp, path).await?;
    tokio::fs::File::open(path.parent().unwrap()).await?.sync_all().await?;
    Ok(())
}
```

### Claude session recovery specifically

Claude Code's own persistence means **session IDs should be stored separately** from terminal state for faster lookup and independent recovery:

```rust
#[derive(Serialize, Deserialize)]
pub struct ClaudeSessionInfo {
    pub session_id: String,
    pub started_at: SystemTime,
    pub exit_status: Option<SessionExitStatus>,
    pub claude_storage_path: PathBuf,  // ~/.claude/projects/...
}

pub enum SessionExitStatus {
    NormalExit { code: i32 },
    Crashed { signal: Option<i32> },  // Detected via signal termination
    Disconnected,
}
```

Detection logic: If `status.code()` returns `None` on Unix, the process was killed by signal (crash). If `status.success()` returns true, it finished normally.

### Zellij's approach as reference

Zellij serializes every **1 second** to `~/.cache/zellij/<version>/session_info/<session>/` in human-readable KDL format. Commands are placed behind a **"Press ENTER to run..."** banner on recovery—a safety feature ccmux should adopt. Known issues include inconsistent tab restoration and first-pane bugs, suggesting ccmux should include comprehensive recovery tests.

---

## Section 4: Prior art reveals architectural patterns worth adopting

Terminal multiplexers share a **client-server architecture** for session persistence, but differ significantly in implementation complexity and feature sets. tmux's ~6MB memory baseline proves minimalism is achievable; Zellij's ~80MB footprint buys WASM plugins and better discoverability.

### tmux's battle-tested foundations

The client-server split uses Unix domain sockets with `imsg` protocol messaging. The data model is hierarchical: **Server → Sessions → Windows → Panes**, stored in red-black trees. Key insights:

- **Grid system** uses memory-efficient encoding: 4-byte `grid_cell_entry` for ASCII, extended entries for wide chars/RGB/hyperlinks
- **Prefix key** works via named "key tables" (`root`, `copy-mode`, `copy-mode-vi`)
- **Configuration hot-reload** via `source-file` command with options in hierarchical RB-trees

Common **pain points**: session persistence requires external plugins (tmux-resurrect), complex keybinding syntax, poor discoverability, performance degradation with large scrollback.

### Zellij's modern Rust architecture

The multi-crate workspace separates concerns cleanly:

| Crate | Purpose |
|-------|---------|
| `zellij-client` | Input handling, UI rendering |
| `zellij-server` | State management, PTY handling |
| `zellij-utils` | IPC (Protocol Buffers), config, errors |
| `zellij-tile` | WASM plugin SDK |

**WASM plugins** run sandboxed with explicit permissions (filesystem, command execution, network). Built-in plugins (tab bar, status bar) use the same API as custom plugins, proving the system's capability.

**Better than tmux**: Intuitive UI with keybinding hints, built-in session restoration, floating/stacked panes, web client for collaboration. **Worse than tmux**: 13x memory usage, 40x binary size, less mature ecosystem.

### Comparison matrix for architecture decisions

| Feature | tmux | Zellij | Wezterm Mux |
|---------|------|--------|-------------|
| Memory (idle) | **~6MB** | ~80MB | Varies |
| Binary size | **~900KB** | ~38MB | ~50MB+ |
| Plugin system | Scripts | **WASM** | Lua events |
| Session persist | Via plugin | **Built-in** | With config |
| Config format | tmux.conf | **KDL** | Lua |
| Learning curve | Steep | **Gentle** | Moderate |

### Recommended ccmux structure

```
ccmux/
├── ccmux-client/      # Input, Ratatui rendering
├── ccmux-server/      # PTY management, state
├── ccmux-utils/       # IPC, config, shared types
└── ccmux-protocol/    # Claude protocol definitions
```

Adopt tmux's hierarchical data model (simpler than Zellij's flat pane approach) with Zellij's crate separation for maintainability.

---

## Section 5: Hot-reload configuration requires careful debouncing

File watching using the **notify** crate with proper debouncing handles the complexities of atomic writes (temp file + rename) and rapid editor saves. Alacritty's implementation provides an excellent reference pattern using 10ms debounce delays and parent directory watching.

### File watching implementation

```rust
use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
use std::time::{Duration, Instant};

const DEBOUNCE_DELAY: Duration = Duration::from_millis(50);

fn watch_config(config_path: &Path) -> notify::Result<()> {
    let (tx, rx) = std::sync::mpsc::channel();
    let mut watcher = RecommendedWatcher::new(tx, Config::default())?;
    
    // Watch parent directory to catch atomic saves (temp → rename)
    let parent = config_path.parent().unwrap();
    watcher.watch(parent, RecursiveMode::NonRecursive)?;
    
    let mut debounce_deadline: Option<Instant> = None;
    let mut pending_events = Vec::new();
    
    loop {
        let timeout = debounce_deadline
            .map(|d| d.saturating_duration_since(Instant::now()))
            .unwrap_or(Duration::MAX);
        
        match rx.recv_timeout(timeout) {
            Ok(Ok(event)) => {
                pending_events.push(event);
                debounce_deadline.get_or_insert(Instant::now() + DEBOUNCE_DELAY);
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                debounce_deadline = None;
                if pending_events.drain(..).any(|e| e.paths.contains(&config_path)) {
                    trigger_reload();
                }
            }
            _ => continue,
        }
    }
}
```

Platform backends: **inotify** on Linux, **FSEvents/kqueue** on macOS, **ReadDirectoryChangesW** on Windows.

### Categorizing changes for selective application

```rust
#[derive(Deserialize, PartialEq)]
pub struct CcmuxConfig {
    // Hot-reloadable
    pub colors: ColorConfig,
    pub keybindings: Vec<Keybinding>,
    pub status_bar: StatusBarConfig,
    
    // Requires restart
    pub terminal: TerminalConfig,
    pub default_shell: String,
}

impl CcmuxConfig {
    pub fn diff_changes(&self, new: &CcmuxConfig) -> ConfigChanges {
        ConfigChanges {
            hot_reload: HotReloadChanges {
                colors: self.colors != new.colors,
                keybindings: self.keybindings != new.keybindings,
            },
            requires_restart: self.terminal != new.terminal 
                || self.default_shell != new.default_shell,
        }
    }
}
```

### Validation with serde_valid

```rust
use serde_valid::Validate;

#[derive(Deserialize, Validate)]
pub struct Config {
    #[validate(minimum = 1, maximum = 65535)]
    pub port: u16,
    
    #[validate(custom = validate_keybindings)]
    pub keybindings: Vec<Keybinding>,
}

fn validate_keybindings(bindings: &[Keybinding]) -> Result<(), ValidationError> {
    let mut seen = HashSet::new();
    for binding in bindings {
        if !seen.insert(&binding.key) {
            return Err(ValidationError::Custom(
                format!("Duplicate keybinding: {}", binding.key)
            ));
        }
    }
    Ok(())
}
```

Always validate before applying and **keep previous config for rollback** on errors.

---

## Section 6: Teaching Claude structured output via skills system

Claude Code's **SKILL.md** and **CLAUDE.md** files provide the ideal mechanism for defining ccmux's output protocol. XML-like namespaced tags (`<ccmux:spawn>`) are preferred over JSON because they parse naturally during streaming, tolerate partial output, and integrate with prose.

### Protocol design using namespaced XML

```xml
<!-- Spawn new pane -->
<ccmux:spawn layout="vertical" focus="new" pane-id="worker-1">
  <command>claude "Implement auth module"</command>
  <cwd>/project/src</cwd>
  <env>CONTEXT_FILE=./context.md</env>
</ccmux:spawn>

<!-- Send input to existing pane -->
<ccmux:input to="worker-1">
yes
</ccmux:input>

<!-- Control pane -->
<ccmux:control pane="worker-1" action="close" />
```

**Why XML over JSON**: Claude's training data heavily features XML/HTML, achieving **~95-98% compliance** with well-prompted XML versus ~60-90% for unprompted JSON. XML also streams naturally—you can parse tags incrementally as tokens arrive.

### SKILL.md definition for ccmux

```markdown
---
name: ccmux-control
description: Control ccmux terminal multiplexer. Use when spawning panes, managing layouts, or coordinating parallel tasks.
allowed-tools: Bash, Read, Write
---

# ccmux Control Protocol

When spawning sessions or controlling the multiplexer, emit XML tags:

<ccmux:spawn layout="vertical|horizontal" focus="new|current">
  <command>command to run</command>
</ccmux:spawn>

Rules:
- Always close tags properly
- Use ccmux: namespace prefix
- Commands execute on tag completion
```

Store in `.claude/skills/ccmux/SKILL.md` for project scope or `~/.claude/skills/ccmux/SKILL.md` for global.

### Streaming parser with recovery

```rust
use htmlparser::Parser;

fn create_ccmux_parser<F: FnMut(CcmuxCommand)>(mut on_command: F) -> Parser {
    let mut current: Option<CcmuxCommand> = None;
    let mut timeout = None;
    
    Parser::new(move |event| {
        match event {
            Event::Start(tag) if tag.name.starts_with("ccmux:") => {
                current = Some(CcmuxCommand::from_tag(tag));
                timeout = Some(Instant::now() + Duration::from_secs(5));
            }
            Event::End(tag) if tag.name.starts_with("ccmux:") => {
                if let Some(cmd) = current.take() {
                    on_command(cmd);
                }
                timeout = None;
            }
            _ => {}
        }
        
        // Timeout recovery for truncated tags
        if timeout.map(|t| Instant::now() > t).unwrap_or(false) {
            current = None;
            timeout = None;
        }
    })
}
```

### Nesting recommendation

**Disallow direct nesting**; use flat structure with references instead. This is more robust to parsing failures:

```xml
<ccmux:spawn pane-id="parent-1">
  <command>claude "Coordinate implementation"</command>
</ccmux:spawn>

<ccmux:spawn parent="parent-1" pane-id="child-1">
  <command>claude "Write tests"</command>
</ccmux:spawn>
```

---

## Section 7: Recursion control requires Erlang-style supervision

Preventing runaway spawns in nested Claude sessions requires **environment variable depth tracking**, **cgroups resource limits**, and **OTP-style supervision strategies**. The recommended maximum depth is **3-5 levels**, with explicit errors (not silent ignores) when limits are hit.

### Depth enforcement across processes

```rust
const CCMUX_DEPTH_VAR: &str = "CCMUX_SESSION_DEPTH";
const MAX_DEPTH: u32 = 5;

fn spawn_child_session(task: &str) -> Result<Child, Error> {
    let current_depth: u32 = std::env::var(CCMUX_DEPTH_VAR)
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    
    if current_depth >= MAX_DEPTH {
        return Err(Error::DepthLimitExceeded { 
            current: current_depth, 
            max: MAX_DEPTH 
        });
    }
    
    Command::new("claude")
        .arg(task)
        .env(CCMUX_DEPTH_VAR, (current_depth + 1).to_string())
        .env("CCMUX_PARENT_ID", &session_id)
        .spawn()
}
```

### Session tree with supervision

```rust
pub struct SessionTree {
    nodes: HashMap<String, Arc<RwLock<SessionNode>>>,
    max_depth: u32,
    max_concurrent: usize,
    semaphore: Arc<Semaphore>,
}

pub struct SessionNode {
    pub id: String,
    pub parent_id: Option<String>,
    pub children: Vec<String>,
    pub status: SessionStatus,
    pub depth: u32,
}

pub enum SessionStatus {
    Running,
    Completed { exit_code: i32, output: String },
    Crashed { error: String },
    Timeout,
}
```

### Erlang OTP supervision strategies for ccmux

| Strategy | Behavior | Use Case |
|----------|----------|----------|
| **one_for_one** | Only restart failed child | Independent parallel tasks |
| **simple_one_for_one** | Dynamic children, same spec | Worker pool of Claude sessions |

**Restart intensity**: Allow 3 restarts per 60 seconds before giving up (Erlang's default).

### Resource limits via cgroups

```rust
use cgroups_rs::{CgroupBuilder, hierarchies::V2, MaxValue};

fn apply_limits(session_id: &str, pid: u64) -> Result<()> {
    let cg = CgroupBuilder::new(session_id)
        .memory()
        .memory_hard_limit(2 * 1024 * 1024 * 1024)  // 2GB
        .done()
        .cpu()
        .cfs_quota_us(50_000)  // 50% of one core
        .cfs_period_us(100_000)
        .done()
        .pid()
        .maximum_number_of_processes(MaxValue::Value(100))
        .done()
        .build(Box::new(V2::new()));
    
    cg.add_task(CgroupPid::from(pid))?;
    Ok(())
}
```

### Fan-out orchestration pattern

```rust
async fn fan_out(tasks: Vec<String>, max_parallel: usize) -> Vec<SessionResult> {
    let semaphore = Arc::new(Semaphore::new(max_parallel));
    
    futures::future::join_all(tasks.into_iter().map(|task| {
        let sem = semaphore.clone();
        async move {
            let _permit = sem.acquire().await.unwrap();
            spawn_claude_session(&task).await
        }
    })).await
}
```

### Recommended defaults

| Parameter | Value | Rationale |
|-----------|-------|-----------|
| Max depth | 5 | Balance flexibility vs complexity |
| Max concurrent | 8-16 | API rate limits |
| Session timeout | 300s | Prevent hung sessions |
| Restart intensity | 3 per 60s | OTP default |
| Memory per session | 2GB | Claude context needs |

---

## Conclusion

Building ccmux requires integrating mature Rust crates (**portable-pty**, **vt100**, **ratatui**) with Claude Code's file-based session system and a robust supervision architecture. The **critical path** involves: (1) implementing PTY management with proper blocking I/O handling, (2) designing the XML-based Claude protocol via SKILL.md files, and (3) building hybrid checkpoint+WAL persistence that stores Claude session IDs separately for independent recovery.

**Key open questions** remain around Claude Code's internal spinner detection (exact Unicode characters undocumented), bidirectional communication patterns (how child output flows back to parents without context explosion), and cross-machine session distribution. Prototype the core PTY+vt100+Ratatui rendering loop first—that architectural foundation will inform solutions to the remaining challenges.

The **recommended next step** is building a minimal prototype that spawns a single Claude session, detects completion via PTY monitoring, and persists the session ID for resume testing. This validates the integration assumptions before investing in the full multiplexer architecture.