# FEAT-100: OrchestrationContext abstraction

**Priority**: P2
**Component**: mcp-bridge
**Status**: new
**Dependents**: FEAT-099

## Problem

Orchestration tools have inconsistent session/pane creation logic:

```rust
// run_parallel - inline session selection
let session_id = if is_hidden {
    get_or_create_orchestration_session(connection).await?
} else {
    get_first_session(connection).await?
};

// run_pipeline - no layout support, hardcoded behavior
CreatePaneWithOptions { session_filter: None, ... }
```

This leads to:
- Inconsistent behavior between tools
- Duplicated logic
- Difficulty adding features like dynamic session naming (FEAT-099)
- Each new orchestration tool must reimplement session handling

## Proposed Solution

Create a unified `OrchestrationContext` that handles session and pane management for all orchestration tools.

### Interface Design

```rust
/// Configuration for orchestration work
#[derive(Debug, Clone)]
pub struct OrchestrationConfig {
    /// Named session to use/create (None = auto-generate)
    pub session: Option<String>,
    /// Layout mode
    pub layout: Layout,
    /// Working directory for panes
    pub cwd: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub enum Layout {
    #[default]
    Hidden,  // Use/create orchestration session
    Tiled,   // Use current visible session
}

/// Manages session lifecycle for orchestration operations
pub struct OrchestrationContext {
    config: OrchestrationConfig,
    session_id: Option<Uuid>,
    session_was_created: bool,
    pane_ids: Vec<Uuid>,
}

impl OrchestrationContext {
    pub fn new(config: OrchestrationConfig) -> Self;

    /// Get or create the appropriate session
    pub async fn get_session(
        &mut self,
        conn: &mut ConnectionManager
    ) -> Result<Uuid, McpError>;

    /// Create a pane in the orchestration session
    pub async fn create_pane(
        &mut self,
        conn: &mut ConnectionManager,
        opts: CreatePaneOptions,
    ) -> Result<Uuid, McpError>;

    /// Track a pane for cleanup
    pub fn track_pane(&mut self, pane_id: Uuid);

    /// Cleanup panes (and session if auto-created)
    pub async fn cleanup(
        &mut self,
        conn: &mut ConnectionManager,
        cleanup_panes: bool,
        cleanup_session: bool,
    ) -> Result<(), McpError>;
}
```

### Usage in Tools

```rust
// run_parallel
pub async fn run_parallel(conn: &mut ConnectionManager, request: RunParallelRequest) -> Result<...> {
    let ctx = OrchestrationContext::new(OrchestrationConfig {
        session: request.session,
        layout: request.layout.into(),
        cwd: request.cwd,
    });

    let session_id = ctx.get_session(conn).await?;

    for cmd in &request.commands {
        let pane_id = ctx.create_pane(conn, CreatePaneOptions {
            command: Some(wrapped_command),
            name: cmd.name.clone(),
            ..Default::default()
        }).await?;
    }

    // ... polling logic ...

    if request.cleanup {
        ctx.cleanup(conn, true, true).await?;
    }
}

// run_pipeline - now gets layout support for free
pub async fn run(&mut self, request: RunPipelineRequest) -> Result<...> {
    let ctx = OrchestrationContext::new(OrchestrationConfig {
        session: request.session,
        layout: request.layout.unwrap_or_default().into(),
        cwd: request.cwd,
    });

    let pane_id = ctx.create_pane(conn, CreatePaneOptions {
        name: Some("pipeline-runner".to_string()),
        ..Default::default()
    }).await?;

    // ... step execution logic ...

    if request.cleanup.unwrap_or(false) {
        ctx.cleanup(conn, true, false).await?;
    }
}
```

## Implementation

### Section 1: Create OrchestrationContext

File: `ccmux-server/src/mcp/bridge/orchestration_context.rs`

- [ ] Define `OrchestrationConfig` struct
- [ ] Define `Layout` enum
- [ ] Implement `OrchestrationContext` struct
- [ ] Implement `get_session()` - migrate logic from `get_or_create_orchestration_session()`
- [ ] Implement `create_pane()` - unified pane creation
- [ ] Implement `cleanup()` - pane and session cleanup

### Section 2: Migrate run_parallel

- [ ] Update `RunParallelRequest` to use new config pattern
- [ ] Refactor `run_parallel()` to use `OrchestrationContext`
- [ ] Remove inline session logic
- [ ] Test parallel execution still works

### Section 3: Migrate run_pipeline

- [ ] Add `layout` and `session` parameters to `RunPipelineRequest`
- [ ] Refactor `PipelineRunner` to use `OrchestrationContext`
- [ ] Test pipeline execution still works
- [ ] Verify hidden layout now works for pipelines

### Section 4: Cleanup Old Code

- [ ] Remove `get_or_create_orchestration_session()` function
- [ ] Remove `get_first_session()` function
- [ ] Update module exports

## Acceptance Criteria

- [ ] `OrchestrationContext` provides unified session/pane management
- [ ] `run_parallel` uses `OrchestrationContext`
- [ ] `run_pipeline` uses `OrchestrationContext` and gains `layout` support
- [ ] Both tools have consistent hidden/tiled behavior
- [ ] All existing tests pass
- [ ] New unit tests for `OrchestrationContext`

## Related

- **FEAT-099**: Dynamic session naming (depends on this)
- **FEAT-094**: run_parallel implementation
- **FEAT-095**: run_pipeline implementation
- `ccmux-server/src/mcp/bridge/orchestration.rs`
