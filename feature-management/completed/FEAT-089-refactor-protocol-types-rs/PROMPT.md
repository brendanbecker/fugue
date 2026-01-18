# FEAT-089: Refactor ccmux-protocol/src/types.rs

**Priority**: P3
**Component**: ccmux-protocol
**Type**: refactor
**Estimated Effort**: medium
**Current Size**: 20.5k tokens (2755 lines)
**Target Size**: <10k tokens per module

## Overview

The protocol types file has grown to 20.5k tokens with the addition of the generic widget system (FEAT-083) and abstract agent state (FEAT-084). Split into logical type groups.

## Current Structure Analysis

The file likely contains:
- Session types (SessionId, SessionInfo, SessionSnapshot)
- Window types (WindowId, WindowInfo)
- Pane types (PaneId, PaneInfo, PaneState)
- Agent types (AgentState, AgentActivity) - new from FEAT-084
- Widget types (Widget, WidgetUpdate, WidgetStore) - new from FEAT-083
- Layout types (Layout, LayoutNode, Direction)
- Metadata types (JsonValue wrapper)
- Beads types (BeadsTask, BeadsStatus) - may be deprecated

## Proposed Module Structure

```
ccmux-protocol/src/types/
├── mod.rs              # Re-exports all types (<2k)
├── session.rs          # Session-related types
├── window.rs           # Window-related types
├── pane.rs             # Pane-related types
├── agent.rs            # AgentState, AgentActivity
├── widget.rs           # Widget, WidgetUpdate, WidgetStore
├── layout.rs           # Layout tree types
└── common.rs           # JsonValue, shared utilities
```

## Refactoring Steps

1. **Categorize types** - List all structs/enums by domain
2. **Check dependencies** - Some types reference others
3. **Extract leaf types first** - Types with no internal dependencies
4. **Work up the dependency tree**
5. **Update all imports across workspace**

## Acceptance Criteria

- [ ] `types.rs` replaced with `types/` module directory
- [ ] Each module <10k tokens
- [ ] All protocol tests pass
- [ ] All crates compile without changes to their code
- [ ] Serialization/deserialization unchanged

## Testing

- Protocol unit tests
- Integration tests across all crates
- Verify bincode serialization still works

## Notes

- Protocol types are used across the entire workspace
- May require `pub use` re-exports for compatibility
- Consider adding type documentation during refactor
