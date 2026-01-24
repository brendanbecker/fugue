# Implementation Plan: BUG-047

**Work Item**: [BUG-047: Clean up compiler warnings across fugue crates](PROMPT.md)
**Component**: build
**Priority**: P3
**Created**: 2026-01-16

## Overview

Clean up 51+ compiler warnings across fugue-server, fugue-client, and fugue-protocol to improve code quality and maintainability.

## Architecture Decisions

### Key Design Choice: Dead Code Handling

**Decision**: Triage dead code into three categories: remove, allow explicitly, or wire up.

**Rationale**:
- Truly unused code should be removed to reduce maintenance burden
- Scaffolding for future features should be kept with explicit `#[allow(dead_code)]` and comments
- Code that should be used but isn't wired up should be connected properly

**Trade-offs**:
- More conservative approach keeps scaffolding around, slightly larger codebase
- Aggressive removal may require reimplementation if features are revived

### Implementation Approach

**Phase 1: Automated Fixes**
- Run `cargo fix` for unused imports
- Low risk, high reward

**Phase 2: Simple Replacements**
- Replace deprecated `PaneState::Claude` with `PaneState::Agent`
- Straightforward find-replace, low risk

**Phase 3: Manual Review**
- Review dead code warnings one by one
- Categorize and handle appropriately
- Higher effort but prevents accidental removal of intentional scaffolding

**Phase 4: Variable Cleanup**
- Prefix unused variables with underscore
- Simple and low risk

## Affected Components

| Component | Type of Change | Risk Level |
|-----------|----------------|------------|
| fugue-server/src/handlers/ | Remove unused imports | Low |
| fugue-server/src/mcp/ | Remove imports, fix deprecated | Low |
| fugue-server/src/persistence/ | Dead code triage | Medium |
| fugue-server/src/orchestration/ | Dead code triage | Low |
| fugue-server/src/beads.rs | Dead code triage | Low |
| fugue-server/src/observability/ | Remove unused imports/code | Low |
| fugue-server/src/agents/ | Dead code, unused vars | Low |
| fugue-client/src/ui/app.rs | Fix deprecated, unused vars | Low |
| fugue-protocol/src/types.rs | Fix deprecated | Low |
| fugue-protocol/src/codec.rs | Remove unused imports | Low |

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Remove code that's actually needed | Low | Medium | Review each removal carefully, run tests |
| Break API by removing public items | Low | High | Check visibility, only remove private dead code |
| Introduce bugs in deprecated replacement | Low | Low | Straightforward rename, tests catch issues |
| Miss warnings in some build configs | Low | Low | Run cargo check with all features |

## Rollback Strategy

If implementation causes issues:
1. Revert commits associated with this work item
2. Verify build warnings return to previous state
3. Document which changes caused problems in comments.md

## Implementation Notes

### Persistence Module Dead Code

The persistence module has significant dead code (WAL, checkpoint, replay). Before removing, check:
- Is FEAT-XXX for crash recovery still planned?
- Are these scaffolding for future work?

If scaffolding, keep with explicit allows:
```rust
#[allow(dead_code)] // Scaffolding for crash recovery (FEAT-XXX)
```

### Beads Module

The beads module appears to have significant unused code. Determine if this is:
- A feature that was never completed
- Being phased out
- Actively needed but not wired up

### MessageRouter

The entire MessageRouter in orchestration/router.rs is unused. This may be:
- Old architecture that was replaced
- Future feature scaffolding
- Dead code to remove

---
*This plan should be updated as implementation progresses.*
