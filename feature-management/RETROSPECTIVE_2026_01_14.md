# Retrospective Report: 2026-01-14

## Overview
This retrospective analyzes the current backlog of 12 open bugs and 21 features to identify patterns, categorize work, and define parallel execution tracks.

## 1. Analysis of Current State

### Critical Clusters
The backlog shows a high concentration of issues in two areas:
1.  **MCP Bridge Stability**: A cluster of bugs (BUG-033, 034, 035, 036, 039, 040) points to systemic fragility in `fugue-server/src/mcp/bridge.rs`. Symptoms include state drift, wrong response types, and operation timeouts. The file is noted as being too large (33k+ tokens), making it a high-risk area for simple patches.
2.  **PTY/Input Handling**: BUG-041 (Claude Code crash on paste) indicates a specific incompatibility in how `fugue` handles large or rapid input (bracketed paste) compared to native terminals.

### Emerging Themes
- **Human Control**: A strong push for "Human-in-the-loop" safety (FEAT-079, 077, 078) to prevent agents from disrupting user workflows.
- **Isolation & Configuration**: New features (FEAT-080, 081, 071) aim to give agents fine-grained control over the execution environment (sandboxing, config).
- **Remote Connectivity**: A set of features (FEAT-066, 067, 068) lays the groundwork for remote peering.

## 2. Work Categorization & Parallel Tracks

We can split the work into three distinct, largely independent parallel streams.

### Stream A: Core Stability (The "Daemon" Track)
**Goal**: Stabilize the MCP bridge and resolve the cluster of state-related bugs.
**Strategy**: Prioritize Refactoring over Patching.
**Owner**: Backend/Systems Engineer

1.  **FEAT-064**: Refactor MCP bridge into modular components. (High Leverage)
    *   *Rationale*: Attempting to fix BUG-033/035/040 individually in the current monolithic bridge is risky. Refactoring first provides a clean slate.
2.  **Bug Fixes**: Once modularized, address:
    *   BUG-035 (Wrong response types)
    *   BUG-034 & BUG-040 (Window creation/persistence)
    *   BUG-039 (MCP hangs)
    *   BUG-033 (Layout validation)

### Stream B: User Experience (The "Client" Track)
**Goal**: Improve TUI reliability and Human-Control safety.
**Strategy**: Focus on Client/Input handling and TUI feedback.
**Owner**: Frontend/TUI Engineer

1.  **BUG-041**: Fix Claude Code crash on paste. (Critical Usability)
2.  **FEAT-079**: Implement Human-Control Arbitration logic.
3.  **FEAT-077**: Add Human-Control UX indicators.
4.  **FEAT-078**: Implement per-client focus state.
5.  **BUG-036**: Fix Selection tools not switching TUI view (Client-side fix).

### Stream C: Advanced Features (The "Capabilities" Track)
**Goal**: Enable new agent workflows without blocking Core or UX.
**Strategy**: Additive changes that don't destabilize existing paths.
**Owner**: Feature Engineer

1.  **FEAT-080**: Per-Pane/Session Configuration via Sideband.
2.  **FEAT-081**: Landlock Integration (dependent on FEAT-080).
3.  **FEAT-071**: Per-pane Claude configuration.
4.  **Remote Peering**: FEAT-066 -> FEAT-067 -> FEAT-068.

## 3. Recommendations

1.  **Halt Feature Work on Stream A**: Do not add new features to `fugue-server` (like Remote Peering) until FEAT-064 (Refactor) is complete to avoid merge conflicts and compounding complexity.
2.  **Swarm BUG-041**: This is a standalone critical bug that degrades the primary use case (using Claude Code inside fugue). It should be tackled immediately by Stream B.
3.  **Defer Stream C Networking**: The Remote Peering work (FEAT-066+) involves the server. It should probably wait until the MCP Bridge refactor is stable, or proceed *very* carefully to avoid touching the bridge.

## 4. Reprioritized Backlog (Top 5)

1.  **BUG-041**: Claude Code crashes on paste (P0 - Usability)
2.  **FEAT-064**: Refactor MCP Bridge (P1 - Strategic Enabler)
3.  **FEAT-079**: Human Control Arbitration (P2 - Core Value Prop)
4.  **FEAT-080**: Sideband Config (P2 - Agent Autonomy)
5.  **BUG-036**: Selection Tools Fix (P0 - Core Functionality)

