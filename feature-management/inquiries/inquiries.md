# Inquiries (Deliberation Processes)

**Project**: fugue
**Last Updated**: 2026-01-20

## Summary Statistics
- Total Inquiries: 5
- By Phase:
  - Research: 5
  - Synthesis: 0
  - Debate: 0
  - Consensus: 0
  - Completed: 0

## Active Inquiries

| ID | Title | Priority | Phase | Research Agents | Deadline | Location |
|----|-------|----------|-------|-----------------|----------|----------|
| INQ-001 | Visualization Architecture Review | P1 | research | 3 | - | `INQ-001-visualization-architecture/` |
| INQ-002 | Intelligent Pipe Fabric | P1 | research | 3 | - | `INQ-002-intelligent-pipe-fabric/` |
| INQ-003 | Hierarchical Orchestration Messaging | P1 | research | 3 | - | `INQ-003-hierarchical-orchestration-messaging/` |
| INQ-004 | MCP Response Integrity | P1 | research | 2 | - | `INQ-004-mcp-response-integrity/` |
| INQ-005 | Submit/Enter Behavior in fugue Input Tools | P2 | research | 2 | - | `INQ-005-submit-enter-behavior/` |

## Completed Inquiries

| ID | Title | Decision Points | Spawned Features | Completed Date | Location |
|----|-------|-----------------|------------------|----------------|----------|

## Recent Activity

### 2026-01-20
- INQ-005 created (submit/Enter behavior - observed during orchestration watchdog setup)

### 2026-01-19
- INQ-004 created (MCP response integrity, observed in Session 18)
- INQ-003 created (hierarchical orchestration, mcp-agent-mail research)
- INQ-001 created (converted from FEAT-103)
- INQ-002 created ("replace | with mux" vision)

---

## About Inquiries

Inquiries are structured deliberation processes for reaching consensus on complex decisions. They progress through four mandatory phases:

### Phases

1. **Research**: Independent parallel exploration by multiple agents
2. **Synthesis**: Consolidation of findings into unified understanding
3. **Debate**: Adversarial argumentation to resolve conflicts
4. **Consensus**: Formalization of decisions and spawning of FEAT work items

### When to Create an Inquiry

- Multiple valid approaches exist and best choice is unclear
- Problem requires deep exploration before solution design
- Stakeholders have conflicting perspectives needing resolution
- Architectural decisions have long-term implications
- Trade-offs need formal analysis and documentation

### Directory Structure

```
inquiries/INQ-XXX-descriptive-slug/
├── inquiry_report.json   # Required: Metadata
├── QUESTION.md           # Required: Problem statement
├── research/             # Required: Research phase outputs
│   ├── agent-1.md        # Independent research report
│   └── agent-N.md        # One per configured research agent
├── SYNTHESIS.md          # Required: Consolidated findings (Phase 2)
├── DEBATE.md             # Required: Structured arguments (Phase 3)
├── CONSENSUS.md          # Required: Final decisions (Phase 4)
└── comments.md           # Optional: Process notes
```

### Outcome

Each inquiry produces:
- Documented decision rationale
- One or more FEAT work items
- Clear traceability from research to implementation

See `feature-management/docs/WORK-ITEM-TYPES.md` for complete type definitions.
