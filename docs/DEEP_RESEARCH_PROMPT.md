# Deep Research Prompt: fugue

## Context

I'm building **fugue**, a Claude Code-aware terminal multiplexer in Rust. The core insight is that traditional terminal multiplexers (tmux, screen) are blind to what's running inside them—they see only raw byte streams. fugue will understand Claude Code's state (thinking, waiting, complete, crashed) and enable intelligent orchestration, crash recovery, and session management.

## Research Objectives

Please provide comprehensive research on each of the following areas. For each topic, I need:
- Current state of the art and available tools/libraries
- Architectural patterns and design tradeoffs
- Concrete implementation considerations
- Potential pitfalls and how to avoid them
- Code examples or pseudocode where helpful

---

## 1. Terminal Emulation in Rust

### Core Questions

**PTY Spawning and Management:**
- How does `portable-pty` work? What are its strengths and limitations?
- How do you spawn a shell/process in a PTY and handle bidirectional I/O?
- What's the difference between master/slave PTY ends and how do you manage them?
- How do you handle PTY resizing (SIGWINCH equivalent)?

**Terminal State Parsing:**
- How does `alacritty_terminal` parse ANSI escape sequences and maintain terminal state?
- What data structures represent a terminal's grid, cursor position, scrollback?
- How do you handle alternate screen buffer (used by vim, less, etc.)?
- What about `vt100` crate vs `alacritty_terminal`—tradeoffs?

**Rendering Through Ratatui:**
- How do you take a parsed terminal state and render it in a Ratatui widget?
- What's the performance model? Can you render at 60fps with multiple panes?
- How do you handle terminal-in-terminal rendering (escape sequence passthrough vs interpretation)?

**Terminal Capabilities:**
- How do you query/handle TERM, terminfo, and capability negotiation?
- What about true color, mouse input, bracketed paste, kitty keyboard protocol?

### Specific Research Requests

1. Analyze Zellij's architecture for terminal emulation—what crates do they use and how?
2. Analyze Wezterm's architecture—they have a custom terminal parser, what's their approach?
3. Find examples of Ratatui widgets that embed full terminal emulators
4. Research the `termwiz` crate (Wezterm's terminal library)

---

## 2. Claude Code Internals

### Core Questions

**State Detection:**
- What visual patterns does Claude Code emit during different states?
- Is there a spinner or animation during "thinking" that could be detected?
- Are there ANSI sequences, Unicode characters, or specific patterns that indicate state?
- Does Claude Code use any terminal capabilities (like title setting) to indicate state?

**Output Structure:**
- Does Claude Code emit any structured data alongside human-readable output?
- Are there any hidden control sequences or machine-readable markers?
- How does the `--output-format` flag work? What formats are available?

**Session Resume:**
- How does `claude --resume` work under the hood?
- What session ID or state file does it use?
- Where is session state stored? Is it in `~/.claude/` somewhere?
- Can you programmatically access the session ID of a running Claude instance?

**Programmatic Interaction:**
- Is there an API, IPC mechanism, or socket for communicating with running Claude?
- Could we use `claude --print` or similar to query session state?
- What environment variables does Claude Code respect?

### Specific Research Requests

1. Run Claude Code with various flags and observe terminal output with `script` or `ttyrec`
2. Examine `~/.claude/` directory structure during active sessions
3. Research any existing tools that wrap or interact with Claude Code
4. Look for any internal documentation or source code hints about IPC

---

## 3. Crash Recovery for Terminal Multiplexers

### Core Questions

**State That Must Be Persisted:**
- What constitutes "terminal state" that needs saving? (screen buffer, scrollback, cursor, attributes)
- How do you serialize terminal state efficiently?
- What about running process state—can you truly recover a process after crash?

**Persistence Strategies:**
- **Continuous:** Write every update to disk. What's the I/O overhead?
- **Checkpoint:** Periodic snapshots. What interval is appropriate? How do you handle crash between checkpoints?
- **WAL (Write-Ahead Log):** Log operations, replay on recovery. How do you compact?

**Session Recovery:**
- How does tmux handle server crash vs client disconnect?
- What's Zellij's approach to session persistence?
- Can you "adopt" orphaned PTYs after crash?

**Claude-Specific Recovery:**
- If Claude Code crashes, what state do we need to call `--resume`?
- How do we detect "Claude crashed" vs "Claude finished normally"?
- Should we persist Claude's session ID separately from terminal state?

### Specific Research Requests

1. Deep dive into tmux's session persistence code
2. Analyze Zellij's session serialization format
3. Research CRIU (Checkpoint/Restore In Userspace) for process recovery
4. Look into abduco/dtach for minimal session persistence

---

## 4. Prior Art in Terminal Multiplexers

### Core Questions

**tmux Architecture:**
- Client/server model: How does this enable persistence?
- Window/pane management: Data structures and algorithms
- Input handling: How does the prefix key work?
- Configuration: How does tmux.conf hot-reload work?
- What are the pain points users commonly complain about?

**Zellij Architecture:**
- Plugin system: How does WASM plugin architecture work?
- Layout system: How are layouts defined and applied?
- Session management: How does Zellij handle session persistence?
- What does Zellij do better than tmux? What's worse?

**Wezterm Mux Mode:**
- How does Wezterm's multiplexer differ from being a terminal emulator?
- What's the client/server model?
- How does it handle remote sessions?

**Comparison Matrix:**
- Feature comparison: tmux vs Zellij vs Wezterm mux
- Performance characteristics
- Configuration complexity
- Extensibility

### Specific Research Requests

1. Analyze the crate structure of Zellij—what would we steal?
2. Look at Wezterm's mux protocol for inspiration
3. Research less common multiplexers: dvtm, abduco, mtm
4. Find user surveys/complaints about tmux to understand pain points

---

## 5. Hot-Reload Configuration Patterns

### Core Questions

**File Watching:**
- How does the `notify` crate work? What backends does it use (inotify, kqueue, etc.)?
- How do you debounce rapid file changes?
- What about atomic writes (write to temp file, then rename)?

**Configuration Application:**
- What config changes can be applied without disruption?
- What changes require session restart?
- How do you validate config before applying?
- How do you handle config parse errors gracefully?

**State Migration:**
- If config changes window layouts, how do you migrate?
- How do you handle keybinding conflicts?
- Should you preserve the old config for rollback?

### Specific Research Requests

1. Analyze how Neovim handles `:source` for config reload
2. Look at how Alacritty handles live config reload
3. Research serde's approach to config validation and defaults
4. Find patterns for "diff and apply" config changes

---

## 6. Claude Code Skills for Structured Output

### Core Questions

**Teaching Structured Output:**
- How do you reliably get Claude to emit machine-parseable markers in output?
- What format is most robust? XML-like tags? JSON blocks? Special Unicode?
- How do you handle when Claude doesn't follow the format?
- Can you use Claude Code's skills/CLAUDE.md to define output protocols?

**Protocol Design:**
- What should a `<fugue:spawn>` block look like?
- Should it be inline in output or use a sideband channel?
- How do you handle nesting (spawn within spawn)?
- Error handling for malformed blocks

**Example Protocol:**
```
<fugue:spawn layout="vertical" focus="new">
  <command>claude "Implement the auth module"</command>
  <env>CONTEXT_FILE=./context.md</env>
</fugue:spawn>
```

**Reliability:**
- What's the failure rate of structured output in practice?
- How do you recover from partial/truncated structured blocks?
- Should you require confirmation before acting on spawn commands?

### Specific Research Requests

1. Research Claude Code's skill system (CLAUDE.md, ~/.claude/settings.json)
2. Look at existing structured output patterns in Claude (XML, JSON mode)
3. Find examples of LLMs controlling external systems via structured output
4. Research Model Context Protocol (MCP) for inspiration on tool protocols

---

## 7. Recursion and Orchestration Patterns

### Core Questions

**Preventing Runaway Spawns:**
- What depth limit is reasonable for nested Claude sessions?
- How do you enforce depth limits across process boundaries?
- What happens when you hit the limit—error? Warning? Silent ignore?

**Session Trees:**
- How do you represent parent-child relationships between sessions?
- Should parents be notified when children complete?
- How does a parent know if a child crashed vs finished?
- Can siblings communicate?

**Resource Management:**
- How do you limit total concurrent Claude sessions?
- Memory and CPU quotas per session?
- Priority/scheduling for nested sessions?

**Completion Notification:**
- How should a child session signal completion to parent?
- Should the parent's Claude see the child's output?
- How do you handle child timeout?

**Orchestration Patterns:**
- Fan-out: Parent spawns multiple workers
- Pipeline: Chain of Claude sessions
- Supervisor: Parent monitors and restarts failed children

### Specific Research Requests

1. Research process supervision patterns (systemd, supervisord, erlang OTP)
2. Look at Kubernetes pod management for multi-container orchestration
3. Study GNU Parallel and similar tools for job management
4. Research existing LLM orchestration frameworks (AutoGPT, BabyAGI patterns)

---

## Deliverables

For each section, please provide:

1. **Executive Summary** (2-3 sentences on the key findings)
2. **Detailed Analysis** (comprehensive coverage of the questions)
3. **Recommended Approach** (what you'd suggest for fugue specifically)
4. **Code Examples** (Rust preferred, but pseudocode acceptable)
5. **References** (links to crates, repos, documentation)
6. **Open Questions** (things that need further investigation or prototyping)

---

## Constraints and Context

- **Language:** Rust (stable, 2021 edition or later)
- **Target platforms:** Linux first, macOS second, Windows not a priority
- **Performance:** Must handle 10+ concurrent panes at 60fps rendering
- **Reliability:** Crash recovery is a first-class feature, not an afterthought
- **Complexity:** Prefer simple, auditable code over clever abstractions
- **Dependencies:** Prefer well-maintained crates with good documentation

---

## About This Research

This prompt is part of a [Context Engineering Methodology](https://github.com/brendanbecker/featmgmt/blob/master/CONTEXT_ENGINEERING_METHODOLOGY.md). The research outputs will be used to:

1. Generate an architecture document (`ARCHITECTURE.md`)
2. Define features for implementation (`feature-management/features/`)
3. Identify risks and unknowns early
4. Provide reference material during implementation

Please be thorough. The quality of this research directly impacts the quality of the final implementation.
