# ChatGPT Research Document - Section Map

> Source: `/home/becker/projects/tools/fugue/docs/research/chatgpt_research.pdf`
> Total: ~12,000 tokens | 49 pages | 5 major sections

## Navigation Tree

```
fugue Deep Research (~12,000 tokens)
│
├── 1. Terminal Emulation in Rust (3,200 tokens) [P1-12]
│   ├── 1.1 Executive Summary (300 tokens) [P1-2]
│   ├── 1.2 Detailed Analysis (2,600 tokens) [P2-11]
│   │   ├── 1.2.1 PTY Spawning (500 tokens) [CODE]
│   │   ├── 1.2.2 Terminal State Parsing (600 tokens) [COMPARISON]
│   │   ├── 1.2.3 Rendering with Ratatui (400 tokens) [CODE]
│   │   ├── 1.2.4 Tmux Implementation Details (300 tokens)
│   │   ├── 1.2.5 Zellij Implementation (300 tokens)
│   │   ├── 1.2.6 WezTerm Implementation (300 tokens)
│   │   └── 1.2.7 Integration with Ratatui (200 tokens) [CODE]
│   └── 1.3 Recommended Approach (300 tokens)
│   └── 1.6 Open Questions (included)
│
├── 2. Claude Code Internals (3,000 tokens) [P12-24]
│   ├── 2.1 Executive Summary (350 tokens)
│   ├── 2.2 Detailed Analysis (2,400 tokens) [P13-22]
│   │   ├── 2.2.1 State Detection (400 tokens) [DETECTION]
│   │   ├── 2.2.2 Output Structure & Hidden Data (300 tokens)
│   │   ├── 2.2.3 Session Resume Implementation (350 tokens)
│   │   ├── 2.2.4 Session State Storage Details (350 tokens)
│   │   ├── 2.2.5 Interacting Programmatically (300 tokens)
│   │   ├── 2.2.6 CLI Flags of Interest (250 tokens) [FLAGS]
│   │   ├── 2.2.7 Directory Structure (200 tokens)
│   │   └── 2.2.10 Environment Variables (250 tokens) [ENV]
│   ├── 2.3 Recommended Approach (250 tokens)
│   └── 2.6 Open Questions (included)
│
├── 3. Crash Recovery for Terminal Multiplexers (2,200 tokens) [P24-32]
│   ├── 3.1 Executive Summary (300 tokens)
│   ├── 3.2 Detailed Analysis (1,500 tokens) [P25-30]
│   │   ├── 3.2.1 State That Must Be Persisted (300 tokens)
│   │   ├── 3.2.2 Persistence Strategies (350 tokens) [STRATEGY]
│   │   ├── 3.2.3 Adopting Orphaned PTYs (300 tokens)
│   │   ├── 3.2.4 Tmux's Approach (250 tokens)
│   │   └── 3.2.6 CRIU Overview (300 tokens)
│   ├── 3.3 Recommended Approach (400 tokens) [CODE]
│   └── 3.6 Open Questions (included)
│
├── 4. Prior Art in Terminal Multiplexers (2,000 tokens) [P32-40]
│   ├── 4.1 Executive Summary (250 tokens)
│   ├── 4.2 Comparison Matrix (300 tokens) [TABLE]
│   ├── 4.3 Pain Points from Users (300 tokens)
│   ├── 4.4 Crate Structure of Zellij (250 tokens)
│   ├── 4.9 Recommended Approach (400 tokens)
│   └── 4.12 Open Questions (500 tokens)
│
└── 5. Hot-Reload Configuration Patterns (1,600 tokens) [P40-49]
    ├── 5.1 Executive Summary (250 tokens)
    ├── 5.2 File Watching Implementation (300 tokens) [CODE]
    ├── 5.3 Applying Config Changes (250 tokens)
    ├── 5.4 Debouncing Rapid Changes (200 tokens)
    ├── 5.6 Validation and Defaults (250 tokens)
    ├── 5.7 Examples in Other Projects (350 tokens)
    └── 5.10 Open Questions (included)
```

## Quick Reference by Topic

| Topic | Primary Section | Page Range | Key Artifacts |
|-------|-----------------|------------|---------------|
| PTY Management | 1.2.1 | P2-4 | `portable-pty`, spawn patterns |
| Terminal Parsing | 1.2.2 | P4-6 | `vt100` vs `alacritty_terminal` |
| Ratatui Rendering | 1.2.3, 1.2.7 | P6-7, P10-11 | Widget implementation |
| Claude State Detection | 2.2.1 | P13-15 | Spinner patterns, JSON modes |
| Session Resume | 2.2.3, 2.2.4 | P16-18 | `--resume`, `~/.claude.json` |
| CLI Flags | 2.2.6 | P19-20 | `-p`, `--output-format` |
| Persistence | 3.2.2 | P26-27 | WAL, snapshot, hybrid |
| Orphan PTYs | 3.2.3 | P27-28 | abduco/dtach, SCM_RIGHTS |
| Client-Server | 3.3 | P30-32 | Background session manager |
| Prior Art | 4.1-4.4 | P32-36 | tmux, Zellij, WezTerm |
| File Watching | 5.2 | P41-43 | `notify` crate |
| Debouncing | 5.4 | P44-45 | Atomic writes handling |

## Markers

- `[CODE]` = Contains code example
- `[TABLE]` = Contains comparison table
- `[FLAGS]` = CLI flags reference
- `[ENV]` = Environment variables
- `[DETECTION]` = State detection patterns
- `[STRATEGY]` = Architectural strategy
- `[COMPARISON]` = Crate comparison
- `[P##-##]` = Page range
