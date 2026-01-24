# Implementation Plan: FEAT-062

**Work Item**: [FEAT-062: Mirror Pane (Picture-in-Picture View)](PROMPT.md)
**Component**: fugue-server, fugue-client
**Priority**: P3
**Created**: 2026-01-11

## Overview

Add mirror pane capability that renders another pane's output in real-time, enabling visibility into other sessions without context switching. This is a new pane type that subscribes to a source pane's output stream.

## Architecture Decisions

### Pane Type Model

**Decision**: Add a `PaneType` enum to distinguish regular panes from mirrors.

```rust
pub enum PaneType {
    Terminal { pty: Pty, parser: TerminalParser },
    Mirror { source_id: PaneId, buffer: TerminalBuffer },
}
```

**Rationale**:
- Clean separation of concerns - terminal panes own PTYs, mirrors do not
- Mirror has its own buffer for independent scrollback
- Explicit type makes rendering and input handling straightforward

**Alternative considered**: Boolean flag `is_mirror` on Pane - rejected as less type-safe.

### Buffer Strategy

**Decision**: Mirrors maintain their own terminal buffer.

**Approach**:
- When source emits output, it's broadcast to all mirrors
- Each mirror parses the output into its own buffer
- This allows independent scroll position per mirror

**Trade-off**: Memory usage increases with each mirror (duplicated buffer content).

**Future optimization**: Share underlying buffer, only track view offset per-mirror. Not needed for v1.

### Output Broadcast Mechanism

**Decision**: Use existing broadcast channel pattern, extend to include mirrors.

**Approach**:
1. Server maintains `HashMap<PaneId, Vec<PaneId>>` mapping sources to their mirrors
2. When source pane produces output:
   - Send to clients viewing that pane (existing behavior)
   - Also forward to mirror pane buffers
   - Clients viewing mirrors receive updates via existing mechanism

### Mirror Lifecycle

**Decision**: Mirrors are tied to their source but handle closure gracefully.

**Rules**:
- When source closes: Mirror becomes a "dead" pane showing "[Source closed]"
- When mirror closes: Remove from mirror registry, no effect on source
- Mirrors do not persist across server restart (stateless)

## Affected Components

| Component | Type of Change | Risk Level |
|-----------|----------------|------------|
| fugue-server/src/session/pane.rs | Add PaneType, mirror logic | Medium |
| fugue-server/src/session/mod.rs | Mirror registry | Low |
| fugue-server/src/mcp/tools.rs | New MCP tool | Low |
| fugue-protocol/src/message.rs | New message types | Low |
| fugue-client/src/ui/pane.rs | Mirror rendering | Low |
| fugue-client/src/input/handler.rs | Mirror input handling | Low |
| fugue/src/cli/commands.rs | CLI command | Low |

## Dependencies

None - uses existing infrastructure.

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Memory growth with many mirrors | Medium | Medium | Document as known limitation; add mirror limit |
| Output race conditions | Low | Medium | Use existing synchronization patterns |
| Dead source handling bugs | Medium | Low | Thorough testing of closure scenarios |
| Performance with high-output sources | Low | Medium | Same perf characteristics as regular panes |

## Implementation Phases

### Phase 1: Server Mirror Infrastructure (2-3 hours)

1. **Add PaneType enum** to pane.rs
   - Terminal variant with PTY and parser
   - Mirror variant with source_id and buffer

2. **Create mirror registry** in session manager
   - HashMap tracking source -> mirrors relationships
   - Add/remove operations
   - Lookup for output broadcasting

3. **Implement mirror creation**
   - Create pane without PTY
   - Register in mirror registry
   - Initialize buffer to match source's current content

4. **Implement output forwarding**
   - When pane output occurs, check for mirrors
   - Forward output to mirror buffers
   - Trigger client updates for mirror viewers

### Phase 2: Protocol Messages (30 min)

1. **Add CreateMirror request message**
   - source_pane_id: PaneId
   - target_pane_id: Option<PaneId>

2. **Add MirrorCreated response message**
   - mirror_pane_id: PaneId
   - source_pane_id: PaneId

3. **Add MirrorSourceClosed notification**
   - mirror_pane_id: PaneId

4. **Extend PaneInfo with mirror metadata**
   - is_mirror: bool
   - mirror_source: Option<PaneId>

### Phase 3: MCP Tool (1 hour)

1. **Add fugue_mirror_pane tool**
   - Parse source_pane_id parameter
   - Optional target_pane_id parameter
   - Call server's mirror creation logic
   - Return mirror pane info

2. **Update pane info responses**
   - Include mirror metadata in pane listings

### Phase 4: Client Rendering (1-2 hours)

1. **Update pane widget**
   - Check if pane is mirror
   - Use different border color (cyan/dim)
   - Show "[MIRROR: {source_id}]" in title

2. **Handle mirror input**
   - Allow scroll keys
   - Block all other input
   - q/Escape to close

3. **Status bar updates**
   - Indicate mirror status when focused

### Phase 5: CLI Command (30 min)

1. **Add mirror subcommand**
   - Argument: source_pane_id
   - Option: --target for existing pane conversion
   - Option: --direction for split direction

### Phase 6: Testing (1-2 hours)

1. **Unit tests**
   - Mirror registry operations
   - PaneType behavior
   - Output forwarding logic

2. **Integration tests**
   - End-to-end mirror creation
   - Output synchronization
   - Source closure handling

## Code Snippets

### PaneType Enum

```rust
pub enum PaneType {
    Terminal {
        pty: PtyHandle,
        parser: VtParser,
    },
    Mirror {
        source_id: PaneId,
    },
}

impl Pane {
    pub fn is_mirror(&self) -> bool {
        matches!(self.pane_type, PaneType::Mirror { .. })
    }

    pub fn mirror_source(&self) -> Option<&PaneId> {
        match &self.pane_type {
            PaneType::Mirror { source_id } => Some(source_id),
            _ => None,
        }
    }
}
```

### Mirror Registry

```rust
pub struct MirrorRegistry {
    /// Map from source pane to its mirrors
    source_to_mirrors: HashMap<PaneId, Vec<PaneId>>,
    /// Reverse map: mirror pane to its source
    mirror_to_source: HashMap<PaneId, PaneId>,
}

impl MirrorRegistry {
    pub fn register(&mut self, source: PaneId, mirror: PaneId) {
        self.source_to_mirrors
            .entry(source.clone())
            .or_default()
            .push(mirror.clone());
        self.mirror_to_source.insert(mirror, source);
    }

    pub fn get_mirrors(&self, source: &PaneId) -> &[PaneId] {
        self.source_to_mirrors.get(source).map(Vec::as_slice).unwrap_or(&[])
    }

    pub fn remove_mirror(&mut self, mirror: &PaneId) -> Option<PaneId> {
        if let Some(source) = self.mirror_to_source.remove(mirror) {
            if let Some(mirrors) = self.source_to_mirrors.get_mut(&source) {
                mirrors.retain(|m| m != mirror);
            }
            Some(source)
        } else {
            None
        }
    }
}
```

### MCP Tool

```rust
async fn fugue_mirror_pane(
    server: &Server,
    source_pane_id: String,
    target_pane_id: Option<String>,
) -> McpResult<Value> {
    let source = PaneId::from(source_pane_id);

    // Verify source exists
    let source_pane = server.get_pane(&source)?;
    if source_pane.is_mirror() {
        return Err(McpError::invalid_params("Cannot mirror a mirror pane"));
    }

    let mirror = if let Some(target) = target_pane_id {
        // Convert existing pane to mirror
        server.convert_to_mirror(PaneId::from(target), source)?
    } else {
        // Create new split as mirror
        server.create_mirror_split(source)?
    };

    Ok(json!({
        "mirror_pane_id": mirror.id.to_string(),
        "source_pane_id": source.to_string(),
    }))
}
```

## Rollback Strategy

If implementation causes issues:
1. Remove MCP tool registration
2. Remove CLI command
3. Remove protocol message types
4. Revert pane.rs to single type
5. Remove mirror registry

Changes are isolated in new code paths and easily reversible.

## Implementation Notes

<!-- Add notes during implementation -->

---
*This plan should be updated as implementation progresses.*
