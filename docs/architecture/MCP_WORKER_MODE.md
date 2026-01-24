# fugue MCP Worker Mode

Spec for reducing MCP overhead in non-orchestrator panes (workers, polecats, subagents) to save tokens and improve startup/performance.

## Goals
- Eliminate unnecessary MCP tool schema loading in worker panes (~5k–50k+ tokens savings per session).
- Keep full MCP available in orchestrator/Mayor pane.
- Maintain session isolation without breaking existing MCP usage.
- Allow future dynamic tool granting (stretch).

## Current Problem
- Claude Code loads **all active MCP servers + tools** into context at session start, even if unused.
- In multi-agent swarms (gastown polecats), this bloats every worker's context window.
- Result: higher token burn, slower init, more hallucinated tool use.

## Approach
1. **Per-pane MCP mode**  
   - Configurable: `mcp_mode = "full" | "minimal" | "none"`  
     - full: default for orchestrator (all tools)  
     - minimal: core safe tools only (e.g., fs read/write, basic bash if allowed)  
     - none: no MCP servers loaded

2. **Implementation levers**  
   - Isolated `CLAUDE_CONFIG_DIR` per pane (already in fugue).  
   - On spawn: create stripped config dir for workers (copy original, remove MCP sections or set `mcp_servers = []`).  
   - Or env override: `CLAUDE_MCP_ENABLED=false` or `CLAUDE_MCP_SERVERS=""` (check Claude Code env support).  
   - Future: MCP call from orchestrator to grant tools dynamically (`<fugue:enable-tool name="git">`).

3. **Flows**

### Orchestrator (full MCP)
```
Mayor pane → normal CLAUDE_CONFIG_DIR with all MCP servers
Claude sees full tool list → can use GitHub, Playwright, etc.
```

### Worker (none / minimal)
```
fugue-server → spawn worker → create stripped config dir
Set CLAUDE_CONFIG_DIR=/tmp/fugue-worker-123/.claude
Claude starts with near-zero MCP overhead
```

### Dynamic Grant (stretch)
```
Worker needs tool → errors or escalates
Orchestrator sends <fugue:enable-tool>
Worker reloads config with added tool
```

## Tradeoffs
| Aspect              | Pro (minimal/none on workers)               | Con                                      |
|---------------------|---------------------------------------------|------------------------------------------|
| Token burn          | Huge savings (5k–50k+ per worker)           | Workers can't use advanced tools         |
| Startup speed       | Faster init for subagents                   | Need fallback/escalation logic           |
| Security            | Sandboxed workers (no dangerous APIs)       | Less flexible without dynamic grant      |
| Complexity          | Simple config/env change                    | Managing per-pane config dirs            |

## Implementation Notes
- Start with "none" mode (easiest): env injection + isolated dir without MCP.
- Test: `/context` in worker pane → confirm MCP tokens near zero.
- Acceptance: Worker starts fast, no unnecessary tools; orchestrator still full power.

Next: Integrate with gastown spawn presets.
