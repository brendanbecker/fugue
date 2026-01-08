# Gemini Research Document - Section Map

> Source: `/home/becker/projects/tools/ccmux/docs/research/gemini_research.md`
> Total: ~8,500 tokens | 528 lines | 10 major sections

## Navigation Tree

```
ccmux Architectural Blueprint (~8,500 tokens)
│
├── 1. Terminal Emulation in Rust (1,450 tokens) [L3-119]
│   ├── 1.1 PTY Spawning and Management (700 tokens) [L7-63]
│   │   ├── 1.1.1 State of the Art: portable-pty (180 tokens)
│   │   ├── 1.1.2 Bidirectional I/O and Resizing (180 tokens)
│   │   └── 1.1.3 Implementation Strategy for ccmux (340 tokens) [CODE]
│   │
│   ├── 1.2 Terminal State Parsing (420 tokens) [L64-90]
│   │   ├── 1.2.1 alacritty_terminal vs. vt100 (280 tokens) [TABLE]
│   │   └── 1.2.2 Data Structures and Storage (140 tokens)
│   │
│   ├── 1.3 Rendering Through Ratatui (280 tokens) [L91-111]
│   │   └── 1.3.1 Adapter Pattern Implementation (220 tokens)
│   │
│   └── 1.4 Terminal Capabilities (120 tokens) [L112-119]
│
├── 2. Claude Code Internals (850 tokens) [L120-176]
│   ├── 2.1 State Detection via Visual Telemetry (320 tokens)
│   ├── 2.2 Output Structure (200 tokens)
│   ├── 2.3 Session Resume (140 tokens)
│   └── 2.4 Programmatic Interaction (100 tokens)
│
├── 3. Crash Recovery for Terminal Multiplexers (680 tokens) [L177-218]
│   ├── 3.1 Architecture: Daemon-Client Separation (180 tokens)
│   ├── 3.2 Persistence Strategies (280 tokens)
│   │   ├── 3.2.1 State That Must Be Persisted (80 tokens)
│   │   └── 3.2.2 Persistence Mechanisms (200 tokens)
│   └── 3.3 Adopting Orphaned PTYs (180 tokens)
│
├── 4. Prior Art in Terminal Multiplexers (520 tokens) [L219-258]
│   ├── 4.1 tmux (80 tokens)
│   ├── 4.2 Zellij (100 tokens)
│   ├── 4.3 WezTerm Mux (100 tokens)
│   ├── 4.4 shpool (80 tokens)
│   └── 4.5 Comparison Matrix (160 tokens) [TABLE]
│
├── 5. Hot-Reload Configuration Patterns (480 tokens) [L259-299]
│   ├── 5.1 File Watching (100 tokens)
│   ├── 5.2 Atomic State Swapping (260 tokens) [CODE]
│   └── 5.3 State Migration (120 tokens)
│
├── 6. Claude Code Skills for Structured Output (520 tokens) [L300-339]
│   ├── 6.1 Teaching Structured Output via MCP (260 tokens)
│   │   └── 6.1.1 ccmux as an MCP Server (200 tokens)
│   ├── 6.2 Fallback Protocol: The Sideband (160 tokens) [CODE]
│   └── 6.3 Reliability (80 tokens)
│
├── 7. Recursion and Orchestration Patterns (400 tokens) [L340-368]
│   ├── 7.1 Supervision Trees (Actor Model) (260 tokens)
│   └── 7.2 Resource Management (120 tokens)
│
├── Deliverables (280 tokens) [L369-388]
│   ├── Phase 1: Foundation (80 tokens)
│   ├── Phase 2: Claude Awareness (80 tokens)
│   └── Phase 3: Robustness (80 tokens)
│
├── Code Examples (900 tokens) [L389-474]
│   ├── Alacritty to Ratatui Adapter (520 tokens) [CODE]
│   └── Hot-Reload Config with ArcSwap (380 tokens) [CODE]
│
├── Conclusion (180 tokens) [L475-478]
│
└── Works Cited (1,200 tokens) [L479-528] [48 references]
```

## Quick Reference by Topic

| Topic | Primary Section | Key Artifacts |
|-------|-----------------|---------------|
| PTY Management | 1.1 | `portable-pty`, `PtyActor` pattern |
| Terminal Parsing | 1.2 | `alacritty_terminal` vs `vt100` comparison |
| UI Rendering | 1.3 | Ratatui adapter pattern |
| Claude Detection | 2.1 | Visual telemetry states |
| Output Parsing | 2.2 | `--output-format stream-json` |
| Session Recovery | 2.3 | `--resume <session_id>` |
| Architecture | 3.1 | Daemon/Client separation |
| Persistence | 3.2 | WAL vs Snapshotting hybrid |
| PTY Recovery | 3.3 | SCM_RIGHTS, shpool pattern |
| Config Reload | 5.2 | `ArcSwap` lock-free swapping |
| MCP Integration | 6.1 | ccmux as MCP server |
| Supervision | 7.1 | Actor model with depth limits |

## Markers

- `[CODE]` = Contains code example
- `[TABLE]` = Contains comparison table
- `[L##-##]` = Line number range
