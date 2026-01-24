# FEAT-099: Dynamic orchestration session naming

**Priority**: P2
**Component**: mcp-bridge
**Status**: blocked
**Depends On**: FEAT-100

## Problem

`fugue_run_parallel` and `fugue_run_pipeline` with `layout: "hidden"` currently use a hardcoded session name:

```rust
const ORCHESTRATION_SESSION_NAME: &str = "__orchestration__";
```

This causes issues:
- All hidden work shares one session (no isolation)
- Session gets cluttered if cleanup fails
- Can't easily observe specific parallel runs
- No way to organize or name batch jobs

## Proposed Solution

Allow callers to specify a session name, with sensible defaults:

```json
fugue_run_parallel({
  "session": "build-tests",     // optional: use/create this session
  "commands": [...],
  "cleanup": true
})
```

### Behavior

1. **If `session` provided:**
   - Look for existing session with that name
   - If not found, create it
   - Run commands there

2. **If `session` omitted:**
   - Generate unique name: `__parallel_{uuid}__` or `__parallel_{timestamp}__`
   - Create session
   - If `cleanup: true`, delete entire session after completion

### Benefits

- Watch specific jobs by switching to their named session
- Multiple parallel runs stay isolated
- Easy cleanup of abandoned sessions
- Backwards compatible (omitting `session` works like before, but with better isolation)

## Implementation

**Note**: This feature builds on FEAT-100 (OrchestrationContext abstraction). The context provides the infrastructure; this feature adds dynamic naming.

### Section 1: Update OrchestrationContext

In `fugue-server/src/mcp/bridge/orchestration_context.rs`:

```rust
impl OrchestrationContext {
    pub async fn get_session(&mut self, conn: &mut ConnectionManager) -> Result<Uuid, McpError> {
        let session_name = self.config.session.clone().unwrap_or_else(|| {
            // Generate unique name when not specified
            format!("__parallel_{}__", Uuid::new_v4().as_simple())
        });

        // Look for existing or create new
        // Track whether we created it for cleanup purposes
        self.session_was_created = !session_exists(&session_name);
        // ...
    }
}
```

### Section 2: Update Tool Schemas

In `fugue-server/src/mcp/tools.rs`, add `session` parameter to:
- `RunParallelParams`
- `RunPipelineParams`

### Section 3: Session Cleanup

In `OrchestrationContext::cleanup()`:
- If `session_was_created` and cleanup requested, delete entire session
- Otherwise just close tracked panes

## Acceptance Criteria

- [ ] `session` parameter added to run_parallel and run_pipeline
- [ ] Named sessions are created/reused correctly
- [ ] Omitted session generates unique name
- [ ] Auto-created sessions cleaned up when `cleanup: true`
- [ ] Existing behavior preserved when session omitted

## Related

- FEAT-094: fugue_run_parallel (implements hidden layout)
- FEAT-095: fugue_run_pipeline (implements hidden layout)
- `fugue-server/src/mcp/bridge/orchestration.rs`
