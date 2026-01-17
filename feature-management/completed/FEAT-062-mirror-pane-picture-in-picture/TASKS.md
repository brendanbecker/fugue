# Task Breakdown: FEAT-062

**Work Item**: [FEAT-062: Mirror Pane (Picture-in-Picture View)](PROMPT.md)
**Status**: Not Started
**Last Updated**: 2026-01-11

## Prerequisites

- [ ] Read and understand PROMPT.md
- [ ] Review PLAN.md and update if needed
- [ ] Understand existing pane creation flow
- [ ] Understand existing output broadcast mechanism
- [ ] Review MCP tool implementation patterns

## Phase 1: Server Mirror Infrastructure

### Pane Type Extension

- [ ] Add `PaneType` enum to `ccmux-server/src/session/pane.rs`
  - [ ] `Terminal` variant with PTY and parser
  - [ ] `Mirror` variant with source_id
- [ ] Add helper methods: `is_mirror()`, `mirror_source()`
- [ ] Update pane creation to set pane type
- [ ] Ensure existing pane tests still pass

### Mirror Registry

- [ ] Create `MirrorRegistry` struct in session module
  - [ ] `source_to_mirrors: HashMap<PaneId, Vec<PaneId>>`
  - [ ] `mirror_to_source: HashMap<PaneId, PaneId>`
- [ ] Implement `register(source, mirror)`
- [ ] Implement `get_mirrors(source) -> &[PaneId]`
- [ ] Implement `remove_mirror(mirror) -> Option<source>`
- [ ] Implement `remove_source(source)` for cleanup
- [ ] Add registry to server state

### Mirror Creation

- [ ] Implement `create_mirror_pane(source_id)` in session manager
  - [ ] Verify source exists and is not a mirror
  - [ ] Create new pane with Mirror type
  - [ ] Initialize buffer (copy from source or start empty)
  - [ ] Register in mirror registry
  - [ ] Return new pane info
- [ ] Implement `convert_to_mirror(target_id, source_id)`
  - [ ] Verify target exists
  - [ ] Close target's PTY if any
  - [ ] Convert pane type to Mirror
  - [ ] Register in mirror registry

### Output Forwarding

- [ ] Modify pane output handler to check for mirrors
- [ ] When source produces output:
  - [ ] Get mirrors from registry
  - [ ] Forward output bytes to each mirror's buffer
  - [ ] Trigger client update for mirror viewers
- [ ] Handle source closure:
  - [ ] Get all mirrors of closing source
  - [ ] Mark mirrors as "source closed" (or remove them)
  - [ ] Notify clients

## Phase 2: Protocol Messages

### New Message Types

- [ ] Add `CreateMirror` request message
  - [ ] `source_pane_id: PaneId`
  - [ ] `target_pane_id: Option<PaneId>`
- [ ] Add `MirrorCreated` response message
  - [ ] `mirror_pane_id: PaneId`
  - [ ] `source_pane_id: PaneId`
- [ ] Add `MirrorSourceClosed` notification
  - [ ] `mirror_pane_id: PaneId`

### Pane Info Extension

- [ ] Add `is_mirror: bool` to PaneInfo struct
- [ ] Add `mirror_source: Option<PaneId>` to PaneInfo
- [ ] Update pane info serialization
- [ ] Update client deserialization

## Phase 3: MCP Tool

### Tool Implementation

- [ ] Add `ccmux_mirror_pane` tool definition
  - [ ] Define input schema (source_pane_id, target_pane_id?)
  - [ ] Define output schema
- [ ] Implement tool handler
  - [ ] Parse and validate parameters
  - [ ] Call server's mirror creation
  - [ ] Return mirror pane info
- [ ] Register tool in MCP server

### Error Handling

- [ ] Invalid source_pane_id error
- [ ] Source is already a mirror error
- [ ] Target pane not found error
- [ ] Cannot mirror self error

## Phase 4: Client Rendering

### Mirror Pane Widget

- [ ] Detect if pane is mirror (from PaneInfo)
- [ ] Use different border style for mirrors
  - [ ] Cyan or dim border color
  - [ ] Consider dashed border
- [ ] Show "[MIRROR]" indicator in title
- [ ] Show source pane ID in title or status

### Input Handling

- [ ] Check if focused pane is mirror
- [ ] If mirror:
  - [ ] Allow: arrow keys, page up/down (scroll)
  - [ ] Allow: q, Escape (close mirror)
  - [ ] Allow: mouse scroll
  - [ ] Block: all other input (don't forward to server)
- [ ] Update focus handling for mirrors

### Status Bar

- [ ] Show mirror indicator when mirror is focused
- [ ] Consider showing source pane status

## Phase 5: CLI Command

### Add Mirror Subcommand

- [ ] Add `mirror` subcommand to CLI
- [ ] Required argument: source_pane_id
- [ ] Optional: `--target <pane_id>`
- [ ] Optional: `--direction <vertical|horizontal>`
- [ ] Implement command handler
- [ ] Add to help text

## Testing Tasks

### Unit Tests

- [ ] Test `MirrorRegistry` operations
  - [ ] Register and lookup
  - [ ] Remove mirror
  - [ ] Remove source (cleanup all mirrors)
- [ ] Test `PaneType` enum and helpers
- [ ] Test mirror creation validation

### Integration Tests

- [ ] Test mirror creation via protocol message
- [ ] Test output forwarding to mirrors
- [ ] Test source closure handling
- [ ] Test MCP tool invocation
- [ ] Test multiple mirrors of same source

### Manual Testing

- [ ] Create pane with continuous output
- [ ] Create mirror of that pane
- [ ] Verify:
  - [ ] Output appears in both panes
  - [ ] Mirror has visual indicator
  - [ ] Scrolling mirror doesn't affect source
  - [ ] Input is blocked on mirror (except scroll/quit)
- [ ] Close source pane
  - [ ] Mirror shows closure message
  - [ ] Mirror can be closed
- [ ] Test with Claude session
  - [ ] Create worker pane
  - [ ] Create mirror
  - [ ] Verify conversation visible in both

## Documentation Tasks

- [ ] Add MCP tool to tools documentation
- [ ] Add CLI command to help
- [ ] Update README with mirror feature
- [ ] Add to CHANGELOG

## Verification Tasks

- [ ] All acceptance criteria from PROMPT.md met:
  - [ ] `ccmux_mirror_pane` MCP tool creates mirror pane
  - [ ] CLI `ccmux mirror <pane_id>` creates mirror pane
  - [ ] Mirror displays source pane output in real-time
  - [ ] Mirror has independent scrollback from source
  - [ ] Mirror shows visual indicator (border color, title)
  - [ ] `q` or `Escape` closes mirror pane
  - [ ] Arrow keys scroll mirror view without affecting source
  - [ ] Source pane closure is handled gracefully
  - [ ] Multiple mirrors of different sources work simultaneously
  - [ ] Server tracks mirror relationships for cleanup
- [ ] Tests passing
- [ ] Update feature_request.json status

## Completion Checklist

- [ ] All implementation tasks complete
- [ ] All tests passing
- [ ] Manual testing completed
- [ ] Documentation updated
- [ ] PLAN.md updated with final notes
- [ ] Ready for review/merge

---
*Check off tasks as you complete them. Update status field above.*
