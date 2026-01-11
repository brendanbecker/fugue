# Implementation Plan: FEAT-045

**Work Item**: [FEAT-045: MCP Declarative Layout Tools](PROMPT.md)
**Component**: ccmux-server (MCP)
**Priority**: P1
**Created**: 2026-01-10

## Overview

Add declarative layout control to MCP tools, allowing LLMs to create complex terminal layouts from natural language descriptions. This exposes the existing `layout.rs` capabilities (nested layouts, custom ratios, presets) through MCP.

## Architecture Decisions

### Approach: Leverage Existing LayoutNode Infrastructure

The `ccmux-client/src/ui/layout.rs` already has a complete layout tree implementation:

- `LayoutNode` enum with `Pane` and `Split` variants
- `split_with_ratios()` for custom sizing
- `LayoutPreset` enum for common patterns
- `LayoutManager` for operations

**Decision**: Parse MCP JSON into `LayoutNode`, use existing methods to build layouts.

**Trade-offs**:
- (+) Reuses proven code
- (+) Layout logic stays in one place
- (-) May need to extract layout types to shared crate for server use

### Approach: Atomic Layout Creation

When creating a complex layout, all panes should be created atomically.

**Decision**: Spawn all PTYs first, then build the `LayoutNode` tree with the resulting pane IDs.

**Trade-offs**:
- (+) Either the whole layout succeeds or fails
- (+) No partial layouts to clean up
- (-) Slightly more complex implementation

### Approach: JSON Schema for Layout Specification

Use a recursive JSON schema that mirrors the `LayoutNode` structure.

**Decision**: Two forms - direct `layout` object or `preset` + `pane_commands` array.

```json
// Form 1: Custom layout
{"layout": {"direction": "horizontal", "splits": [...]}}

// Form 2: Preset with commands
{"preset": "main_left", "pane_commands": [...]}
```

**Trade-offs**:
- (+) Flexible for complex needs
- (+) Simple presets for common cases
- (-) LLMs may prefer simpler form

## Affected Components

| Component | Type of Change | Risk Level |
|-----------|----------------|------------|
| `ccmux-server/src/mcp/tools.rs` | Add tool definitions | Low |
| `ccmux-server/src/mcp/handlers.rs` | Implement handlers | Medium |
| `ccmux-server/src/mcp/bridge.rs` | Route new tools | Low |
| `ccmux-server/src/session/manager.rs` | Layout application | Medium |
| `ccmux-protocol/src/messages.rs` | Layout sync messages | Low |

## Key Implementation Details

### 1. Layout Specification Types

```rust
// In mcp/handlers.rs or new mcp/layout.rs

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum LayoutSpec {
    Pane(PaneSpec),
    Split(SplitSpec),
}

#[derive(Debug, Deserialize)]
pub struct PaneSpec {
    pub pane: PaneConfig,
}

#[derive(Debug, Deserialize)]
pub struct PaneConfig {
    pub command: Option<String>,
    pub cwd: Option<String>,
    pub name: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SplitSpec {
    pub direction: String,  // "horizontal" | "vertical"
    pub splits: Vec<SplitChild>,
}

#[derive(Debug, Deserialize)]
pub struct SplitChild {
    pub ratio: f32,
    #[serde(flatten)]
    pub layout: LayoutSpec,
}
```

### 2. Layout Building Algorithm

```rust
async fn build_layout_from_spec(
    spec: &LayoutSpec,
    session_mgr: &SessionManager,
    session_id: &SessionId,
    window_id: &WindowId,
) -> Result<(LayoutNode, Vec<PaneInfo>), Error> {
    let mut panes = Vec::new();
    let node = build_node(spec, session_mgr, session_id, window_id, &mut panes).await?;
    Ok((node, panes))
}

async fn build_node(
    spec: &LayoutSpec,
    /* ... */
    panes: &mut Vec<PaneInfo>,
) -> Result<LayoutNode, Error> {
    match spec {
        LayoutSpec::Pane(pane_spec) => {
            // Spawn PTY, create pane
            let pane_id = session_mgr.create_pane(
                session_id,
                window_id,
                pane_spec.pane.command.clone(),
                pane_spec.pane.cwd.clone(),
            ).await?;
            panes.push(PaneInfo { id: pane_id, name: pane_spec.pane.name.clone() });
            Ok(LayoutNode::pane(pane_id))
        }
        LayoutSpec::Split(split_spec) => {
            let direction = match split_spec.direction.as_str() {
                "horizontal" => SplitDirection::Horizontal,
                "vertical" => SplitDirection::Vertical,
                _ => return Err(Error::InvalidDirection),
            };

            let mut children = Vec::new();
            for child in &split_spec.splits {
                let node = build_node(&child.layout, /* ... */, panes).await?;
                children.push((node, child.ratio));
            }

            // Normalize ratios if they don't sum to 1.0
            let total: f32 = children.iter().map(|(_, r)| r).sum();
            if (total - 1.0).abs() > 0.001 {
                for (_, ratio) in &mut children {
                    *ratio /= total;
                }
            }

            Ok(LayoutNode::split_with_ratios(direction, children))
        }
    }
}
```

### 3. Preset Handling

```rust
fn preset_from_name(name: &str) -> Result<LayoutPreset, Error> {
    match name {
        "single" => Ok(LayoutPreset::Single),
        "split_horizontal" => Ok(LayoutPreset::SplitHorizontal),
        "split_vertical" => Ok(LayoutPreset::SplitVertical),
        "grid_2x2" => Ok(LayoutPreset::Grid2x2),
        "main_left" => Ok(LayoutPreset::MainLeft),
        "main_top" => Ok(LayoutPreset::MainTop),
        _ => Err(Error::UnknownPreset(name.to_string())),
    }
}
```

### 4. Response Format

```rust
#[derive(Debug, Serialize)]
pub struct CreateLayoutResponse {
    pub success: bool,
    pub session_id: String,
    pub session_name: String,
    pub window_id: String,
    pub window_name: Option<String>,
    pub panes: Vec<PaneInfoResponse>,
    pub layout_applied: String,  // "custom" or preset name
}

#[derive(Debug, Serialize)]
pub struct PaneInfoResponse {
    pub id: String,
    pub name: Option<String>,
    pub command: Option<String>,
}
```

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Layout code in wrong crate | Medium | Medium | Extract layout types to ccmux-protocol or shared crate |
| Ratio normalization edge cases | Low | Low | Add tests for edge cases, round to 0.01 precision |
| Partial failure during spawn | Medium | High | Track spawned panes, clean up on error |
| Client layout sync issues | Medium | Medium | Leverage FEAT-039 broadcast infrastructure |

## Rollback Strategy

If implementation causes issues:
1. Revert commits associated with this work item
2. Verify existing `ccmux_create_pane` still works
3. Document what went wrong in comments.md

## Testing Strategy

1. **Unit tests for layout spec parsing**:
   - Valid nested layout specs
   - Invalid direction values
   - Ratio normalization

2. **Unit tests for preset mapping**:
   - All preset names recognized
   - Unknown preset error handling

3. **Integration tests**:
   - Create complex layout via MCP, verify pane count
   - Verify layout structure matches spec
   - Verify TUI client receives layout update

## Implementation Notes

- Start with `ccmux_create_layout` as the core feature
- `ccmux_split_pane` and `ccmux_resize_pane` can follow
- Consider adding `ccmux_get_layout` to query current structure
- Layout types may need to move to `ccmux-protocol` crate for sharing

---
*This plan should be updated as implementation progresses.*
