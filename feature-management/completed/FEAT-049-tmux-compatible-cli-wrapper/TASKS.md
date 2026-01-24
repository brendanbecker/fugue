# Task Breakdown: FEAT-049

**Work Item**: [FEAT-049: Add tmux-compatible CLI wrapper (fugue-compat)](PROMPT.md)
**Status**: Not Started
**Last Updated**: 2026-01-10

## Prerequisites

- [ ] Read and understand PROMPT.md
- [ ] Review PLAN.md and update if needed
- [ ] Review Gas Town's tmux usage patterns (internal/tmux/tmux.go)
- [ ] Review existing fugue MCP tools

## Crate Setup Tasks

- [ ] Create fugue-compat/ directory structure
- [ ] Create Cargo.toml with dependencies
- [ ] Add fugue-compat to workspace Cargo.toml
- [ ] Create src/main.rs with basic structure
- [ ] Verify crate builds

## CLI Parsing Tasks

- [ ] Set up clap with tmux-style subcommands
- [ ] Implement `new-session` argument parsing
  - [ ] -d (detached)
  - [ ] -s NAME (session name)
  - [ ] -c DIR (working directory)
  - [ ] [CMD] (initial command)
- [ ] Implement `send-keys` argument parsing
  - [ ] -t TARGET (target)
  - [ ] -l (literal)
  - [ ] TEXT... (keys to send)
- [ ] Implement `kill-session` argument parsing
  - [ ] -t NAME (target)
- [ ] Implement `has-session` argument parsing
  - [ ] -t =NAME (exact match target)
- [ ] Implement `list-sessions` argument parsing
  - [ ] -F FORMAT (format string)
- [ ] Implement `capture-pane` argument parsing
  - [ ] -p (print to stdout)
  - [ ] -t TARGET (target)
  - [ ] -S start (start line)
  - [ ] -E end (end line)

## Connection Layer Tasks

- [ ] Create connection module
- [ ] Implement Unix socket connection
- [ ] Implement protocol codec (reuse from fugue-protocol)
- [ ] Handle connection errors with appropriate exit codes
- [ ] Auto-start server if not running (like fugue-client)

## Command Translation Tasks

- [ ] Create translation module
- [ ] Implement new-session translation
  - [ ] Map to fugue_create_session
  - [ ] Handle initial command via fugue_send_input
- [ ] Implement send-keys translation
  - [ ] Map to fugue_send_input
  - [ ] Handle special keys (Enter, C-c, etc.)
  - [ ] Handle -l literal mode
- [ ] Implement kill-session translation
  - [ ] Map to fugue_kill_session
- [ ] Implement has-session translation
  - [ ] Map to fugue_list_sessions + filter
  - [ ] Return exit code 0/1
- [ ] Implement list-sessions translation
  - [ ] Map to fugue_list_sessions
  - [ ] Apply format string
- [ ] Implement capture-pane translation
  - [ ] Map to fugue_read_pane
  - [ ] Handle line range options

## Output Formatting Tasks

- [ ] Create output module
- [ ] Implement format string parser
- [ ] Implement format variable substitution
  - [ ] #{session_name}
  - [ ] #{session_windows}
  - [ ] #{session_created}
  - [ ] #{session_attached}
- [ ] Match tmux default format exactly
- [ ] Ensure newlines and whitespace match

## Exit Code Tasks

- [ ] Document tmux exit codes for each command
- [ ] Implement matching exit codes
  - [ ] 0: success
  - [ ] 1: not found / no sessions
  - [ ] 2: invalid arguments
- [ ] Test exit codes match tmux

## Testing Tasks

- [ ] Create tests/ directory
- [ ] Add unit tests for CLI parsing
- [ ] Add unit tests for format string parsing
- [ ] Add integration tests for each command
- [ ] Create comparison test script (tmux vs fugue-compat)
- [ ] Test with mock Gas Town usage patterns

## Documentation Tasks

- [ ] Add README.md for fugue-compat
- [ ] Document supported commands
- [ ] Document known differences from tmux
- [ ] Add migration guide
- [ ] Update workspace README

## Verification Tasks

- [ ] All acceptance criteria from PROMPT.md met
- [ ] Tests passing
- [ ] Update feature_request.json status
- [ ] Manual testing with Gas Town patterns

## Completion Checklist

- [ ] All implementation tasks complete
- [ ] All tests passing
- [ ] Documentation updated
- [ ] PLAN.md reflects final implementation
- [ ] Ready for review/merge

---
*Check off tasks as you complete them. Update status field above.*
