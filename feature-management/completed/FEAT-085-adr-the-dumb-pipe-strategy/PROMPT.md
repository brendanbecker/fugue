# FEAT-085: ADR: The Dumb Pipe Strategy

**Priority**: P1
**Component**: docs
**Type**: enhancement
**Estimated Effort**: medium
**Business Value**: medium
**Status**: COMPLETED (2026-01-16)

## Overview

Draft an Architectural Decision Record (ADR) defining the 'Dumb Pipe' philosophy for fugue. Explicitly forbid future application-specific logic in the core server/protocol. Define the standard for Sideband usage as the primary integration point.

## Deliverables

- [x] `docs/adr/ADR-001-dumb-pipe-strategy.md` - The ADR document
- [x] Updated `docs/HANDOFF.md` - Reference the ADR
- [x] Updated feature-management files - Marked complete

## What Was Done

### ADR Document Created

Created `docs/adr/ADR-001-dumb-pipe-strategy.md` with:

1. **Full Context**: Documented all current agent-specific code:
   - `ClaudeState` and `ClaudeActivity` in protocol types
   - `BeadsTask` and `BeadsStatus` in protocol types
   - Beads-specific MCP tools in handlers.rs
   - Problems with tight coupling (fragility, maintenance burden, limited extensibility)

2. **Decision**: fugue evolves toward a "dumb pipe" - a minimal, reliable multiplexer that:
   - Multiplexes PTY streams reliably
   - Provides generic metadata storage
   - Offers simple message passing
   - Exposes capabilities via sideband protocol

3. **Consequences** (positive and negative):
   - Positive: Increased reliability, agent-agnostic, easier maintenance
   - Negative: Loss of "magic" features, agent-side burden
   - Mitigations documented

4. **Implementation Path**:
   - Phase 1: Generic Widget System (FEAT-083)
   - Phase 2: Abstract Agent State (FEAT-084)
   - Phase 3: Deprecate agent-specific sideband commands
   - Phase 4: Document agent-side patterns

5. **Before/After Examples** for:
   - Displaying agent activity
   - Task tracking widgets
   - Agent coordination

## Acceptance Criteria

- [x] ADR document created with full context, decision, and consequences
- [x] Clear implementation path outlined (FEAT-083, FEAT-084)
- [x] Examples of "before/after" for key features
- [x] Team alignment on direction (implicit via PR review)

## Next Steps

After committing this ADR, proceed to:
- FEAT-083 (Generic Widget System) - Replace BeadsTask
- FEAT-084 (Abstract Agent State) - Generalize ClaudeState
