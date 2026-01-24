# ChatGPT Research Document - Section Abstracts

> Source: `/home/becker/projects/tools/fugue/docs/research/chatgpt_research.pdf`
> Each abstract: 100-200 tokens summarizing section content

---

## 1. Terminal Emulation in Rust
**Pages 1-12 | ~3,200 tokens**

Comprehensive coverage of PTY spawning via `portable-pty` (WezTerm ecosystem) with `native_pty_system().openpty()`, slave.spawn_command(), and master.try_clone_reader() patterns. Compares terminal state parsing options: recommends **vt100** for simplicity and `contents_diff()` optimization, though notes **alacritty_terminal** handles edge cases better (alternate screen buffer). Details Ratatui widget implementation iterating screen cells with color mapping. Examines tmux (C, grid model), Zellij (Rust, WASM plugins, sixel), and WezTerm (full GUI emulator). References **cockpit** and **tui-term** crates as integration examples. Open questions: vt100 vs alacritty for Claude's actual output, scrollback management, mouse support, Unicode/emoji width handling.

---

## 2. Claude Code Internals
**Pages 12-24 | ~3,000 tokens**

Documents Claude Code as Node/NPM agentic CLI with session state in `~/.claude.json` (updated ~1.5x/sec via atomic temp+rename). State detection: interactive mode shows spinner (`|/-\` via carriage return), permission prompts as text; JSON mode provides explicit status events. Recommends `--output-format stream-json` with `--include-partial-messages` for structured parsing. Session resume via `--resume <session_id>` or `-c` for most recent. Session IDs are UUIDs visible in debug logs. No IPC/socket API - stdio only. Concurrent sessions risk `.claude.json` conflicts - recommends separate HOME directories per instance. Key flags: `-p` (print mode), `--fork-session`, `--dangerously-skip-permissions`. Environment: `ANTHROPIC_API_KEY`, `CLAUDE_CODE_ACTION`, `CLAUDE_BASH_MAINTAIN_PROJECT_WORKING_DIR`.

---

## 3. Crash Recovery for Terminal Multiplexers
**Pages 24-32 | ~2,200 tokens**

Establishes that true process recovery requires OS-level CRIU (impractical). State to persist: screen contents with attributes, scrollback history, cursor position, mode flags, pane layout. For Claude: save session ID for `--resume`; for shells: cannot recover - start fresh. Three persistence strategies: (1) Continuous WAL with I/O overhead, (2) Periodic snapshots losing inter-snapshot data, (3) **Hybrid recommended**: WAL + periodic compressed snapshots with log position markers. Recommends **client-server architecture**: fork at startup with parent as server (holds PTYs) and child as UI client via Unix socket. Periodic checkpointing to `~/.fugue/last_session.json` with atomic writes. On recovery: restore Claude via `--resume`, show shell screens as "ghost image". References abduco/dtach for PTY-holding patterns and tmux-resurrect for layout restoration.

---

## 4. Prior Art in Terminal Multiplexers
**Pages 32-40 | ~2,000 tokens**

Analyzes **tmux**: client-server model, detachable sessions, multiple windows/panes, config via tmux.conf, prefix key (Ctrl-B), lightweight (~5MB). **Zellij**: modern Rust with WASM plugins, built-in status bar, YAML layouts, higher memory (~63MB). **WezTerm mux**: integrated with GUI terminal, SSH domain support, Lua config. Comparison matrix: persistence (all detach/attach, none crash-recover), extensibility (tmux: scripts, Zellij: WASM, WezTerm: Lua). Pain points: tmux complex config and TERM/color issues; all lack built-in layout persistence. Zellij uses `portable-pty` and vte/vt100 fork, MIT licensed. Recommended approach: client-server model, Session>Window>Pane hierarchy, configurable prefix key, common shortcuts (%, ", arrows, x, c, [), file watch config reload, status bar with Claude state indicator.

---

## 5. Hot-Reload Configuration Patterns
**Pages 40-49 | ~1,600 tokens**

File watching via **notify** crate with `RecommendedWatcher` + debounce mode. Watch directory (not just file) to catch atomic temp→rename patterns (shows as Create event). 50-100ms debounce to coalesce rapid editor saves. Categorize changes: live-adjustable (colors, keybinds, status bar) apply immediately; fundamental settings document as restart-required. Invalid config: fail entire load atomically, keep previous config, log error. Validation via Serde with `#[serde(default)]` for missing fields; validate ranges/conflicts before applying. Examples: Neovim `:source` re-executes config live; Alacritty `live_config_reload` applies changes automatically with error tolerance. Open questions: error notification (status bar flash?), rollback strategy for unbinding prefix key, handling file-not-found scenarios.
