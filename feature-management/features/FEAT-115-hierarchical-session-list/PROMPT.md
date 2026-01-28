# FEAT-115: Hierarchical Session List View

**Priority**: P2
**Component**: mcp/session
**Effort**: Small
**Status**: new

## Summary

Add an option to `fugue_list_sessions` (and potentially the TUI) to display sessions in a hierarchical tree view based on parent-child relationships, rather than a flat list.

## Problem

Currently `fugue_list_sessions` returns a flat list of all sessions:

```
nexus
orch-perf-takehome
fugue-orch
worker-BUG-069
worker-BUG-071
worker-FEAT-104
__watchdog
```

This makes it hard to understand the orchestration topology. With lineage tags (`child:<parent>`), we have the data to show relationships but no way to visualize them.

## Proposed Solution

### Option 1: Tree Parameter for `fugue_list_sessions`

Add a `tree` or `hierarchical` parameter:

```json
// Flat list (current behavior, default)
fugue_list_sessions({})

// Hierarchical view
fugue_list_sessions({"view": "tree"})
```

**Tree output:**
```
nexus (tags: nexus)
├── fugue-orch (tags: orchestrator, child:nexus)
│   ├── worker-BUG-069 (tags: worker, child:fugue-orch)
│   ├── worker-BUG-071 (tags: worker, child:fugue-orch)
│   └── __watchdog (tags: watchdog, child:fugue-orch)
└── orch-perf-takehome (tags: orchestrator, child:nexus)
    ├── worker-f28 (tags: worker, child:orch-perf-takehome)
    └── worker-f29 (tags: worker, child:orch-perf-takehome)
```

### Option 2: Separate `fugue_list_sessions_tree` Tool

New dedicated tool for hierarchical view:

```json
fugue_list_sessions_tree({
  "root": "nexus",        // Optional: start from specific session
  "max_depth": 3          // Optional: limit depth
})
```

### Option 3: Add `parent` and `children` Fields to Response

Enhance the existing response with relationship data:

```json
{
  "sessions": [
    {
      "name": "fugue-orch",
      "id": "...",
      "tags": ["orchestrator", "child:nexus"],
      "parent": "nexus",
      "children": ["worker-BUG-069", "worker-BUG-071", "__watchdog"]
    }
  ]
}
```

## Implementation Notes

### Deriving Hierarchy from Tags

The `child:<parent>` tag convention already exists. To build the tree:

1. Parse `child:*` tags to extract parent session name
2. Build adjacency map: `parent -> [children]`
3. Find roots (sessions with no `child:*` tag or parent not found)
4. Recursively build tree from roots

```rust
fn build_session_tree(sessions: &[Session]) -> Vec<TreeNode> {
    let mut children_map: HashMap<String, Vec<&Session>> = HashMap::new();

    for session in sessions {
        if let Some(parent) = session.tags.iter()
            .find(|t| t.starts_with("child:"))
            .map(|t| t.strip_prefix("child:").unwrap())
        {
            children_map.entry(parent.to_string())
                .or_default()
                .push(session);
        }
    }

    // Find roots and build tree recursively
    // ...
}
```

### TUI Integration (Future)

The TUI session list could also show hierarchy:
- Tree view with expand/collapse
- Indentation to show depth
- Icons for role (🎯 nexus, 📋 orchestrator, 👷 worker, 🐕 watchdog)

## Acceptance Criteria

- [ ] `fugue_list_sessions` accepts `view: "tree"` parameter
- [ ] Tree view shows parent-child relationships based on `child:*` tags
- [ ] Sessions without parents shown as roots
- [ ] Sessions with missing parents shown as orphaned roots
- [ ] Flat view remains default for backward compatibility
- [ ] Response includes enough data for clients to build their own tree

## Example Use Case

Orchestrator checking on its workers:

```json
// Before: flat list, must manually filter
fugue_list_sessions({})
// Returns 15 sessions, hard to find my workers

// After: see my subtree
fugue_list_sessions({"view": "tree", "root": "fugue-orch"})
// Returns only my children in tree format
```

## Related

- FEAT-104: Watchdog Orchestration Skill (would benefit from this)
- FEAT-106: Session creation tags (provides the lineage tags)
- Lineage tag convention: `child:<parent-session-name>`
