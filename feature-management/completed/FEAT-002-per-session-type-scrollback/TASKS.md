# Task Breakdown: FEAT-002

**Work Item**: [FEAT-002: Per-Session-Type Scrollback Configuration](PROMPT.md)
**Status**: Not Started
**Last Updated**: 2026-01-08

## Prerequisites

- [ ] Read and understand PROMPT.md
- [ ] Review PLAN.md and update if needed
- [ ] Verify existing config hot-reload mechanism works
- [ ] Review current `TerminalConfig` in `schema.rs`
- [ ] Review `Pane` struct in `pane.rs`

## Phase 1: Config Schema Extension

- [ ] Define `ScrollbackConfig` struct in `schema.rs`
  - [ ] Add `default: usize` field (default: 1000)
  - [ ] Add `orchestrator: usize` field (default: 50000)
  - [ ] Add `worker: usize` field (default: 500)
  - [ ] Add `#[serde(flatten)] custom: HashMap<String, usize>` for extensibility
- [ ] Update `TerminalConfig` to replace `scrollback_lines` with `scrollback: ScrollbackConfig`
- [ ] Add backwards compatibility handling for legacy `scrollback_lines` field
- [ ] Add `impl Default for ScrollbackConfig`
- [ ] Add unit tests for `ScrollbackConfig` parsing
- [ ] Add unit test for legacy config migration
- [ ] Update example config files if they exist

## Phase 2: Scrollback Buffer Implementation

- [ ] Create `fugue-server/src/pty/buffer.rs`
- [ ] Define `ScrollbackBuffer` struct
  - [ ] `lines: VecDeque<String>`
  - [ ] `max_lines: usize`
  - [ ] `total_bytes: usize` (for memory tracking)
- [ ] Implement `ScrollbackBuffer::new(max_lines: usize)`
- [ ] Implement `ScrollbackBuffer::push_line(line: String)`
  - [ ] Handle circular buffer wrap-around
  - [ ] Update byte count tracking
- [ ] Implement `ScrollbackBuffer::get_lines(start: usize, count: usize) -> &[String]`
- [ ] Implement `ScrollbackBuffer::len() -> usize`
- [ ] Implement `ScrollbackBuffer::total_bytes() -> usize`
- [ ] Implement `ScrollbackBuffer::clear()`
- [ ] Add module to `fugue-server/src/pty/mod.rs`
- [ ] Add unit tests for all buffer operations
- [ ] Add test for wrap-around behavior
- [ ] Add test for memory tracking accuracy

## Phase 3: Pane Integration

- [ ] Add `scrollback_override: Option<usize>` field to `Pane` struct
- [ ] Add `scrollback_buffer: ScrollbackBuffer` field to `Pane` struct
- [ ] Update `Pane::new()` signature to accept scrollback configuration
- [ ] Add `Pane::effective_scrollback_size()` method
  - [ ] Return override if set
  - [ ] Otherwise return session-type default from config
- [ ] Add `Pane::set_scrollback_override(size: Option<usize>)` method
- [ ] Add `Pane::append_output(data: &[u8])` method
  - [ ] Parse bytes into lines
  - [ ] Handle partial lines (buffer incomplete line)
  - [ ] Push complete lines to scrollback buffer
- [ ] Update `Pane::to_info()` to include scrollback info if needed
- [ ] Add unit tests for pane scrollback integration
- [ ] Add test for effective size resolution

## Phase 4: PTY Output Integration

- [ ] Update `PtyHandle` to expose output events
- [ ] Create output processing pipeline in pane/session layer
- [ ] Wire PTY output to `Pane::append_output()`
- [ ] Handle high-throughput scenarios efficiently
- [ ] Add integration test for end-to-end output capture

## Phase 5: Spawn Directive Enhancement

- [ ] Locate spawn directive parser
- [ ] Add `scrollback` attribute parsing to `<fugue:spawn>`
- [ ] Validate scrollback value (positive integer)
- [ ] Pass scrollback override through pane creation
- [ ] Add unit test for spawn directive parsing
- [ ] Add integration test for spawn with scrollback override

## Phase 6: Memory Management

- [ ] Add global memory tracking for all scrollback buffers
- [ ] Implement memory warning threshold (configurable)
- [ ] Log warning when approaching memory limit
- [ ] Consider implementing buffer trimming under pressure (stretch goal)
- [ ] Add metrics/stats for monitoring buffer memory usage

## Documentation Tasks

- [ ] Update configuration documentation with new scrollback options
- [ ] Document session type configuration
- [ ] Document spawn directive scrollback attribute
- [ ] Add memory considerations to documentation

## Verification Tasks

- [ ] All acceptance criteria from PROMPT.md verified
- [ ] All tests passing
- [ ] Manual testing of scrollback in TUI
- [ ] Memory usage profiling with large buffers
- [ ] Hot-reload behavior verified
- [ ] Update feature_request.json status to "completed"
- [ ] Document completion in comments.md

## Completion Checklist

- [ ] All implementation tasks complete
- [ ] All tests passing
- [ ] Documentation updated
- [ ] PLAN.md reflects final implementation
- [ ] Ready for review/merge

---
*Check off tasks as you complete them. Update status field above.*
