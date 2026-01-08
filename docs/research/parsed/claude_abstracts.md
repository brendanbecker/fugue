# ccmux Research Document - Section Abstracts (Claude)

> Source: `/home/becker/projects/tools/ccmux/docs/research/claude_research.md`
> Each abstract: 100-200 tokens summarizing section content

---

## Executive Summary (L1-5)

The recommended architecture combines **portable-pty** for PTY management, **vt100** for terminal parsing, and **Ratatui** for rendering via a client-server model. Claude Code has no native IPC; detection requires PTY monitoring or `stream-json` output. Crash recovery uses hybrid checkpoint + WAL, storing Claude session IDs separately. Sessions persist as JSONL in `~/.claude/projects/` and can be resumed via `--resume <session-id>`.

---

## Section 1: Terminal Emulation Components (L7-97)

Three layered problems: PTY lifecycle, escape sequence parsing, and rendering. **portable-pty** (WezTerm ecosystem) handles cross-platform PTY with automatic platform detection. PTY reads are blocking, requiring dedicated threads. **vt100** outperforms alacritty_terminal for multiplexers due to `contents_diff()` for efficient screen updates. **Ratatui** with **tui-term** renders vt100 screens directly. For 10+ panes at 60fps: batch reads, throttle to 16ms, leverage double-buffering. Recommended deps: ratatui 0.29, crossterm 0.28, portable-pty 0.9, vt100 0.16, tui-term 0.2, tokio 1.

---

## Section 2: Claude Code Integration (L100-162)

Claude Code is a React/Ink TUI with no native IPC. State detection via PTY monitoring: thinking shows ASCII flower spinner, waiting shows prompt with `>`, permission shows mode indicators. **stream-json** provides structured output: `claude -p --output-format stream-json`. Sessions stored in `~/.claude/projects/[encoded-path]/[uuid].jsonl`. Resume via `--continue` (most recent) or `--resume <id>` (specific). Key env vars: `ANTHROPIC_API_KEY`, `CLAUDE_CONFIG_DIR`, `FORCE_COLOR=0` for parsing.

---

## Section 3: Crash Recovery Strategies (L165-246)

Terminal state includes primary/alternate screens, scrollback, cursor, modes. Running processes cannot be recovered without CRIU. Solution: persist enough state to recreate. **bincode** for speed (~24B overhead), JSON/KDL for debugging. Hybrid approach: WAL (**okaywal** crate) for operations between 30-60s checkpoints. Atomic checkpoint writes prevent corruption. Store Claude session IDs separately for independent recovery. Detect crashes via signal termination (status.code() returns None). Zellij serializes every 1s to KDL, shows "Press ENTER" banner on recovery.

---

## Section 4: Prior Art & Architecture (L249-300)

Terminal multiplexers use client-server for persistence. **tmux**: 6MB baseline, Server→Sessions→Windows→Panes hierarchy, Unix sockets with imsg, grid uses 4-byte cells. Pain points: external persistence plugins, complex syntax. **Zellij**: 80MB footprint, multi-crate workspace (client/server/utils/tile), WASM plugins with explicit permissions. Better UI/restore/floating panes, but 13x memory. Recommended ccmux structure: ccmux-client, ccmux-server, ccmux-utils, ccmux-protocol. Adopt tmux hierarchy with Zellij crate separation.

---

## Section 5: Configuration Hot-Reload (L303-407)

Use **notify** crate with debouncing for atomic writes and rapid saves. Watch parent directory to catch temp→rename patterns. 50ms debounce delay, collect events then filter for config path. Platform backends: inotify (Linux), FSEvents/kqueue (macOS), ReadDirectoryChangesW (Windows). Categorize changes: hot-reloadable (colors, keybindings, status bar) vs restart-required (terminal config, shell). Use `diff_changes()` to determine scope. Validate with **serde_valid** before applying; keep previous config for rollback.

---

## Section 6: Claude Skills Protocol (L410-506)

SKILL.md and CLAUDE.md define ccmux output protocol. XML-like namespaced tags (`<ccmux:spawn>`) preferred over JSON: streams naturally, tolerates partial output, integrates with prose, ~95-98% compliance vs 60-90% for JSON. Protocol: `<ccmux:spawn layout="vertical" pane-id="worker-1">`, `<ccmux:input to="worker-1">`, `<ccmux:control action="close">`. Store SKILL.md in `.claude/skills/ccmux/`. Streaming parser with 5s timeout recovery for truncated tags. Disallow direct nesting; use flat structure with parent references.

---

## Section 7: Recursion Control & Supervision (L509-624)

Prevent runaway spawns via environment variable depth tracking (`CCMUX_SESSION_DEPTH`), cgroups limits, and OTP supervision. Max depth: 3-5 levels with explicit errors. Session tree tracks nodes with parent/children, status (Running/Completed/Crashed/Timeout). Erlang strategies: one_for_one for independent tasks, simple_one_for_one for worker pools. Restart intensity: 3 per 60s. cgroups via **cgroups-rs**: 2GB memory, 50% CPU, 100 processes per session. Fan-out with semaphore for max parallelism. Defaults: depth 5, concurrent 8-16, timeout 300s.

---

## Conclusion (L627-633)

Critical path: (1) PTY management with blocking I/O handling, (2) XML protocol via SKILL.md, (3) hybrid persistence with separate Claude session IDs. Open questions: spinner Unicode chars undocumented, bidirectional communication patterns, cross-machine distribution. Recommended next step: minimal prototype spawning single Claude session, detecting completion via PTY, persisting session ID for resume.
