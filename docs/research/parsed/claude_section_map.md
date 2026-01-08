# ccmux Research Document - Section Map (Claude)

> Source: `/home/becker/projects/tools/ccmux/docs/research/claude_research.md`
> Total: ~4500 tokens | 633 lines | 7 major sections

## Navigation Tree

```
ccmux Research Document (~4500 tokens)
│
├── Executive Summary (180 tokens) [L1-5]
│   └── Key insight: portable-pty + vt100 + Ratatui architecture
│
├── 1. Terminal Emulation Components (750 tokens) [L7-97]
│   ├── 1.1 PTY management with portable-pty (320 tokens) [L11-45]
│   ├── 1.2 Terminal state parsing with vt100 (200 tokens) [L47-67]
│   ├── 1.3 Rendering at 60fps with Ratatui (130 tokens) [L69-84]
│   └── 1.4 Recommended crate dependencies (100 tokens) [L86-97]
│
├── 2. Claude Code Integration (580 tokens) [L100-162]
│   ├── 2.1 Visual state detection patterns (100 tokens) [L104-112]
│   ├── 2.2 Structured output via stream-json (130 tokens) [L114-129]
│   ├── 2.3 Session storage and resume mechanics (200 tokens) [L131-151]
│   └── 2.4 Key environment variables (100 tokens) [L153-162]
│
├── 3. Crash Recovery Strategies (680 tokens) [L165-246]
│   ├── 3.1 What constitutes terminal state (180 tokens) [L169-191]
│   ├── 3.2 Hybrid checkpoint + WAL strategy (250 tokens) [L193-219]
│   ├── 3.3 Claude session recovery specifically (180 tokens) [L221-242]
│   └── 3.4 Zellij's approach as reference (70 tokens) [L244-246]
│
├── 4. Prior Art & Architecture (520 tokens) [L249-300]
│   ├── 4.1 tmux's battle-tested foundations (130 tokens) [L253-261]
│   ├── 4.2 Zellij's modern Rust architecture (170 tokens) [L263-277]
│   ├── 4.3 Comparison matrix (100 tokens) [L279-288]
│   └── 4.4 Recommended ccmux structure (120 tokens) [L290-300]
│
├── 5. Configuration Hot-Reload (620 tokens) [L303-407]
│   ├── 5.1 File watching implementation (320 tokens) [L307-346]
│   ├── 5.2 Categorizing changes for selective application (180 tokens) [L348-377]
│   └── 5.3 Validation with serde_valid (120 tokens) [L379-407]
│
├── 6. Claude Skills Protocol (600 tokens) [L410-506]
│   ├── 6.1 Protocol design using namespaced XML (180 tokens) [L414-433]
│   ├── 6.2 SKILL.md definition for ccmux (170 tokens) [L435-458]
│   ├── 6.3 Streaming parser with recovery (200 tokens) [L460-491]
│   └── 6.4 Nesting recommendation (50 tokens) [L493-506]
│
├── 7. Recursion Control & Supervision (750 tokens) [L509-624]
│   ├── 7.1 Depth enforcement across processes (200 tokens) [L513-538]
│   ├── 7.2 Session tree with supervision (180 tokens) [L540-564]
│   ├── 7.3 Erlang OTP supervision strategies (80 tokens) [L566-573]
│   ├── 7.4 Resource limits via cgroups (170 tokens) [L575-597]
│   ├── 7.5 Fan-out orchestration pattern (100 tokens) [L599-613]
│   └── 7.6 Recommended defaults (80 tokens) [L615-624]
│
└── Conclusion (220 tokens) [L627-633]
    └── Critical path: PTY mgmt → XML protocol → hybrid persistence
```

## Quick Reference by Topic

| Topic | Primary Section | Supporting Sections |
|-------|-----------------|---------------------|
| Crate selection | 1.4 | 1.1, 1.2, 1.3 |
| Claude detection | 2.1 | 2.2 |
| Session resume | 2.3 | 3.3 |
| Crash recovery | 3.2 | 3.1, 3.3, 3.4 |
| Architecture | 4.4 | 4.1, 4.2, 4.3 |
| Config hot-reload | 5.1 | 5.2, 5.3 |
| XML protocol | 6.1 | 6.2, 6.3 |
| Session tree | 7.2 | 7.1, 7.3 |
| Resource limits | 7.4 | 7.5, 7.6 |
