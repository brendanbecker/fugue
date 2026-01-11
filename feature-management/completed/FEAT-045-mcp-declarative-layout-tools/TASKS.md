# Task Breakdown: FEAT-045

**Work Item**: [FEAT-045: MCP Declarative Layout Tools](PROMPT.md)
**Status**: Not Started
**Last Updated**: 2026-01-10

## Prerequisites

- [ ] Read and understand PROMPT.md
- [ ] Review PLAN.md and update if needed
- [ ] Review existing `ccmux-client/src/ui/layout.rs` implementation
- [ ] Review current MCP tools in `ccmux-server/src/mcp/tools.rs`
- [ ] Verify FEAT-029 (MCP base) is implemented
- [ ] Check FEAT-039 (MCP broadcast) status for client sync

## Design Tasks

- [ ] Decide if layout types need to move to shared crate
- [ ] Finalize JSON schema for layout specification
- [ ] Design error handling for partial failures
- [ ] Design response format for all new tools
- [ ] Update PLAN.md with final design decisions

## Implementation Tasks

### Tool Definitions (tools.rs)

- [ ] Add `ccmux_create_layout` tool definition
  - [ ] Define nested layout schema
  - [ ] Define preset option schema
  - [ ] Define pane_commands array schema
- [ ] Add `ccmux_split_pane` tool definition
  - [ ] pane_id parameter
  - [ ] direction parameter
  - [ ] ratio parameter (0.1-0.9)
  - [ ] command/cwd/name parameters
- [ ] Add `ccmux_resize_pane` tool definition
  - [ ] pane_id parameter
  - [ ] delta parameter (-0.5 to 0.5)
- [ ] Update `ccmux_create_pane` tool definition
  - [ ] Add ratio parameter
  - [ ] Add source_pane parameter

### Layout Spec Parsing (handlers.rs or new layout.rs)

- [ ] Define `LayoutSpec` enum (Pane | Split)
- [ ] Define `PaneSpec` struct
- [ ] Define `PaneConfig` struct
- [ ] Define `SplitSpec` struct
- [ ] Define `SplitChild` struct
- [ ] Implement `parse_layout_spec()` function
- [ ] Implement ratio normalization
- [ ] Implement preset name to `LayoutPreset` mapping

### Layout Building

- [ ] Implement `build_layout_from_spec()` function
- [ ] Implement recursive `build_node()` function
- [ ] Handle pane spawning at leaf nodes
- [ ] Collect pane IDs as tree is built
- [ ] Handle direction parsing
- [ ] Handle ratio normalization

### Handler Implementation

- [ ] Implement `handle_create_layout()` handler
  - [ ] Parse session/window parameters
  - [ ] Parse layout or preset
  - [ ] Build layout tree
  - [ ] Spawn panes
  - [ ] Apply layout to window
  - [ ] Return success response
- [ ] Implement `handle_split_pane()` handler
  - [ ] Find target pane
  - [ ] Calculate ratios
  - [ ] Spawn new pane
  - [ ] Update layout tree
- [ ] Implement `handle_resize_pane()` handler
  - [ ] Find pane in layout tree
  - [ ] Apply delta to ratios
  - [ ] Broadcast update

### Bridge/Router Updates

- [ ] Add routing for `ccmux_create_layout` in bridge.rs
- [ ] Add routing for `ccmux_split_pane` in bridge.rs
- [ ] Add routing for `ccmux_resize_pane` in bridge.rs
- [ ] Update `ccmux_create_pane` routing for new params

### Session Manager Updates

- [ ] Add method to replace window's layout tree
- [ ] Add method to get current layout structure
- [ ] Ensure pane creation integrates with layout

### Client Sync (if FEAT-039 available)

- [ ] Ensure layout changes broadcast to TUI clients
- [ ] Test client receives and applies layout updates
- [ ] Handle reconnecting client layout sync

## Testing Tasks

### Unit Tests

- [ ] Test layout spec parsing - simple pane
- [ ] Test layout spec parsing - horizontal split
- [ ] Test layout spec parsing - vertical split
- [ ] Test layout spec parsing - nested splits
- [ ] Test layout spec parsing - 3+ way splits
- [ ] Test ratio normalization - exact 1.0
- [ ] Test ratio normalization - less than 1.0
- [ ] Test ratio normalization - greater than 1.0
- [ ] Test preset mapping - all presets
- [ ] Test preset mapping - invalid preset error
- [ ] Test direction parsing - valid values
- [ ] Test direction parsing - invalid value error

### Integration Tests

- [ ] Test create_layout with simple split
- [ ] Test create_layout with nested layout
- [ ] Test create_layout with preset
- [ ] Test create_layout with preset and commands
- [ ] Test split_pane with default ratio
- [ ] Test split_pane with custom ratio
- [ ] Test resize_pane grow
- [ ] Test resize_pane shrink
- [ ] Test resize_pane at bounds
- [ ] Verify pane commands execute
- [ ] Verify pane names are set
- [ ] Verify TUI client displays layout

### Error Handling Tests

- [ ] Test invalid layout spec
- [ ] Test invalid direction value
- [ ] Test invalid ratio (< 0.1 or > 0.9)
- [ ] Test unknown preset name
- [ ] Test nonexistent pane_id for split/resize
- [ ] Test partial spawn failure cleanup

## Documentation Tasks

- [ ] Document layout spec JSON format in PROMPT.md examples
- [ ] Add usage examples for each new tool
- [ ] Update tool descriptions for clarity
- [ ] Document ratio normalization behavior
- [ ] Document preset pane ordering

## Completion Checklist

- [ ] All implementation tasks complete
- [ ] All tests passing
- [ ] PLAN.md updated with final implementation notes
- [ ] feature_request.json status updated
- [ ] Ready for review/merge

---
*Check off tasks as you complete them. Update status field above.*
