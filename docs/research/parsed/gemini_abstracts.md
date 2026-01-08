# Gemini Research Document - Section Abstracts

> Source: `/home/becker/projects/tools/ccmux/docs/research/gemini_research.md`
> Each abstract: 100-200 tokens summarizing section content

---

## 1. Terminal Emulation in Rust
**Lines 3-119 | ~1,450 tokens**

Comprehensive guide to building terminal emulation in Rust for ccmux. Covers PTY spawning via `portable-pty` crate from WezTerm, which provides cross-platform abstraction over Unix/Windows PTY interfaces. Details the four-step PTY dance (open master, grant/unlock, spawn child, handshake) and async I/O integration with Tokio. Recommends `alacritty_terminal` over `vt100` for state parsing due to superior handling of alternate screen buffers and edge cases. Explains the adapter pattern needed to bridge Alacritty's Cell type to Ratatui's Buffer, including color mapping and dirty-rect optimization for 60fps with 10+ panes.

---

## 2. Claude Code Internals
**Lines 120-176 | ~850 tokens**

Documents Claude Code's observable behaviors for building "Claude Awareness" in ccmux. Identifies visual telemetry states (Thinking/Channelling, Synthesizing, Discombobulating, Waiting) detectable via carriage return patterns in the byte stream. Recommends `--output-format stream-json` for spawning managed Claude panes, enabling real-time parsing of token usage and cognitive state without screen clutter. Details session resume mechanism via `claude --resume <session_id>` with state stored in `~/.claude/`. Notes absence of formal IPC socket - interaction limited to STDIN injection, filesystem watchers, or future MCP.

---

## 3. Crash Recovery for Terminal Multiplexers
**Lines 177-218 | ~680 tokens**

Establishes the Daemon-Client architecture as mandatory for session persistence. The Daemon owns PTY handles and terminal state with no UI dependencies, while the Client handles rendering and connects via Unix Domain Socket. For daemon crashes, recommends hybrid persistence: topology/metadata to SQLite/JSON on change (low frequency), but skip full grid persistence - instead rely on Claude's `--resume` to restore agentic context. Advanced recovery via SCM_RIGHTS file descriptor passing or shpool-style holder process that keeps PTYs alive across daemon restarts.

---

## 4. Prior Art in Terminal Multiplexers
**Lines 219-258 | ~520 tokens**

Analyzes four terminal multiplexers for ccmux design lessons. **tmux**: Validates client/server separation as essential. **Zellij**: Rust-based with WASM plugins; adopt KDL layout format. **WezTerm**: Proves Rust viability for high-performance terminal emulation; source of `portable-pty`. **shpool**: Demonstrates session persistence as distinct problem; potential dependency for "keep alive" functionality. Comparison matrix shows ccmux targets deep Claude semantic awareness vs limited/none in existing tools, with MCP+Rust extensibility vs WASM or custom DSL.

---

## 5. Hot-Reload Configuration Patterns
**Lines 259-299 | ~480 tokens**

Patterns for modifying keybindings and layouts without restart. Use `notify` crate with `notify-debouncer-full` to handle atomic file writes that generate event flurries. For configuration access during rendering, use `ArcSwap` for lock-free atomic swapping - readers get Arc snapshot without blocking writers, critical for 60fps. State migration requires reconciliation logic: when layout changes remove panes, use "Adopt or Kill" strategy - orphaned processes move to detached list rather than immediate termination to preserve work from config errors.

---

## 6. Claude Code Skills for Structured Output
**Lines 300-339 | ~520 tokens**

Strategies for formal Claude-ccmux communication beyond regex scraping. Primary recommendation: ccmux as MCP Server using `mcp-rust-sdk` (rmcp), exposing tools like `create_pane()`, `read_pane_output()`, `list_panes()`. Claude sees these tools and can call them for deterministic layout changes. Fallback: XML-like sideband protocol defined in CLAUDE.md (e.g., `<ccmux-action type="spawn">`), parsed from stream and hidden from display. Notes that stream-json preferred over JSON mode for incremental text display with structured control signals.

---

## 7. Recursion and Orchestration Patterns
**Lines 340-368 | ~400 tokens**

Prevents infinite recursion when Claude can spawn terminals (including itself). Implements supervision tree via Tokio + `ractor` or `JoinSet`: Root Supervisor -> Session Supervisor -> Pane Actors. Session Supervisor tracks pane tree depth with hard limit (e.g., Depth 3). Crash policy: restart with `--resume` up to 3 times with exponential backoff. Resource management via token quotas (budget per session, pause on exceed) and process limits via `tokio::sync::Semaphore` for concurrent Claude processes.

---

## Deliverables
**Lines 369-388 | ~280 tokens**

Three-phase roadmap. **Phase 1 (Foundation)**: Daemon/Client with Tokio + portable-pty, Ratatui rendering alacritty_terminal, basic UDS reconnection. **Phase 2 (Claude Awareness)**: State detection via stream regex, stream-json parsing for clean UI, basic CLAUDE.md sideband parsing. **Phase 3 (Robustness)**: Full MCP server, session ID serialization with auto-resume, strict actor hierarchy with recursion limits.

---

## Code Examples
**Lines 389-474 | ~900 tokens**

Two production-ready code samples. **Alacritty-Ratatui Adapter**: Function `render_grid_to_buffer()` using Alacritty's `display_iter()` for optimized sparse iteration, mapping Cell attributes (fg, bg, flags) to Ratatui Style including TrueColor RGB preservation. **Hot-Reload Config**: `ArcSwap<AppConfig>` static with `notify-debouncer-full` watching ccmux.toml, 500ms debounce, atomic store on change.

---

## Conclusion
**Lines 475-478 | ~180 tokens**

Positions ccmux as "smart containers" vs "dumb pipes" - a bidirectional control surface for developer-AI collaboration. Core stack: portable-pty + alacritty_terminal + Actor model + MCP integration. Daemon/Client split with shpool-inspired persistence ensures daily-driver reliability.

---

## Works Cited
**Lines 479-528 | ~1,200 tokens**

48 references covering crate documentation, GitHub repositories, and technical articles on terminal emulation, multiplexer architecture, and Claude Code internals.
