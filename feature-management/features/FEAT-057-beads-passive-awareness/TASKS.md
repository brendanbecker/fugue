# Task Breakdown: FEAT-057

**Work Item**: [FEAT-057: Beads Passive Awareness - Auto-Detection and Environment Setup](PROMPT.md)
**Status**: Not Started
**Last Updated**: 2026-01-11

## Prerequisites

- [ ] Read and understand PROMPT.md
- [ ] Review PLAN.md and update if needed
- [ ] Review existing PTY spawn and config patterns
- [ ] Understand existing env var infrastructure from FEAT-047

## Design Tasks

- [ ] Review requirements and acceptance criteria
- [ ] Design solution architecture
- [ ] Identify affected components
- [ ] Update PLAN.md with approach
- [ ] Consider edge cases (symlinks, permission errors, network mounts)

## Implementation Tasks

### Configuration
- [ ] Add `BeadsConfig` struct to config schema
- [ ] Add `[beads]` section parsing with defaults
- [ ] Add config validation for beads settings
- [ ] Write unit tests for config parsing

### Detection Logic
- [ ] Implement `detect_beads_root(cwd: &Path) -> Option<PathBuf>`
- [ ] Add fast path optimization for current directory
- [ ] Handle symlinks correctly
- [ ] Handle permission errors gracefully (return None)
- [ ] Write unit tests for detection function

### Pane Metadata
- [ ] Add `beads_root: Option<PathBuf>` field to Pane struct
- [ ] Update Pane serialization for persistence
- [ ] Update Pane creation to call detection
- [ ] Ensure beads_root is populated before PTY spawn

### Environment Integration
- [ ] Modify PTY spawn to check for beads_root
- [ ] Set BEADS_DIR when auto_set_beads_dir is enabled
- [ ] Set BEADS_NO_DAEMON when no_daemon_default is enabled
- [ ] Merge with existing session environment correctly
- [ ] Write integration tests for env var propagation

### Status Line
- [ ] Add beads indicator to pane status rendering
- [ ] Use "[bd]" badge format
- [ ] Ensure indicator appears in both tab bar and status line
- [ ] Self-review changes

## Testing Tasks

- [ ] Add unit tests for beads directory detection
- [ ] Test detection with nested directories
- [ ] Test detection with symlinked .beads/
- [ ] Add unit tests for BeadsConfig parsing
- [ ] Add integration tests for env var propagation
- [ ] Test that BEADS_DIR is set correctly
- [ ] Test that BEADS_NO_DAEMON respects config
- [ ] Verify no performance regression on pane creation
- [ ] Run full test suite

## Documentation Tasks

- [ ] Update configuration documentation with [beads] section
- [ ] Add beads integration section to README/docs
- [ ] Add code comments where needed
- [ ] Update CHANGELOG if applicable

## Verification Tasks

- [ ] All acceptance criteria from PROMPT.md met
- [ ] Tests passing
- [ ] Manual testing with actual beads repository
- [ ] Update feature_request.json status
- [ ] Document completion in comments.md

## Completion Checklist

- [ ] All implementation tasks complete
- [ ] All tests passing
- [ ] Documentation updated
- [ ] PLAN.md reflects final implementation
- [ ] Ready for review/merge

---
*Check off tasks as you complete them. Update status field above.*
