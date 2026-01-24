# Task Breakdown: FEAT-044

**Work Item**: [FEAT-044: Claude Session Persistence and Auto-Resume](PROMPT.md)
**Status**: Not Started
**Last Updated**: 2026-01-10

## Prerequisites

- [ ] Read and understand PROMPT.md
- [ ] Review PLAN.md and update if needed
- [ ] Verify FEAT-016 (Persistence) is complete or understand current state
- [ ] Locate Pane struct in fugue-server/src/session/
- [ ] Understand PTY spawn path in fugue-server/src/pty/
- [ ] Review existing persistence/WAL infrastructure
- [ ] Test Claude CLI --session-id and --resume flags manually

## Phase 1: Pane Metadata Extension

### 1.1 Add Claude Fields to Pane Struct
- [ ] Locate Pane struct definition in fugue-server/src/session/pane.rs
- [ ] Add `claude_session_id: Option<String>` field
- [ ] Add `claude_working_dir: Option<PathBuf>` field
- [ ] Add `is_claude_pane: bool` field
- [ ] Update any `Default` or `new()` implementations

### 1.2 Update Pane Creation
- [ ] Initialize claude_session_id to None on pane creation
- [ ] Initialize claude_working_dir to None on pane creation
- [ ] Initialize is_claude_pane to false on pane creation
- [ ] Ensure fields are properly cloned if Pane is Clone

### 1.3 Update Persistence Serialization
- [ ] Locate pane serialization in persistence/ directory
- [ ] Add claude_session_id to checkpoint serialization
- [ ] Add claude_working_dir to checkpoint serialization
- [ ] Add is_claude_pane to checkpoint serialization
- [ ] Use Option handling (serialize None as null or skip)

### 1.4 Update Persistence Deserialization
- [ ] Add claude_session_id to checkpoint deserialization
- [ ] Add claude_working_dir to checkpoint deserialization
- [ ] Add is_claude_pane to checkpoint deserialization
- [ ] Handle missing fields gracefully (default to None/false)

### 1.5 WAL Entry Types
- [ ] Add WAL entry type for claude session ID update
- [ ] Implement WAL write for session ID assignment
- [ ] Implement WAL replay for session ID
- [ ] Test WAL round-trip

### 1.6 Phase 1 Verification
- [ ] Write unit test: create pane with session ID, serialize, deserialize
- [ ] Verify all three new fields round-trip correctly
- [ ] Test backward compatibility (load old panes without new fields)

## Phase 2: Command Detection and Injection

### 2.1 Implement is_claude_command()
- [ ] Create function `is_claude_command(cmd: &str) -> bool`
- [ ] Handle bare "claude" command
- [ ] Handle "claude ..." with arguments
- [ ] Handle chained commands: "&& claude", "; claude"
- [ ] Handle quoted strings correctly
- [ ] Add unit tests for various patterns

### 2.2 Implement has_session_flag()
- [ ] Create function `has_session_flag(cmd: &str) -> bool`
- [ ] Detect --session-id flag
- [ ] Detect --resume flag
- [ ] Detect --continue flag
- [ ] Handle flag at any position in command
- [ ] Add unit tests

### 2.3 Implement inject_session_id()
- [ ] Create function `inject_session_id(cmd: &str, uuid: &str) -> String`
- [ ] Insert --session-id after "claude" and before other args
- [ ] Preserve existing arguments
- [ ] Handle edge cases (no args, many args)
- [ ] Add unit tests

### 2.4 Hook into PTY Spawn Path
- [ ] Locate PTY spawn function
- [ ] Call is_claude_command() to detect Claude panes
- [ ] If Claude and no session flag, generate UUID
- [ ] Call inject_session_id() to modify command
- [ ] Store session ID in pane metadata

### 2.5 Capture Working Directory
- [ ] Capture CWD at spawn time
- [ ] Store in pane.claude_working_dir
- [ ] Handle case where CWD is not available
- [ ] Ensure directory path is absolute

### 2.6 Write Session ID to WAL
- [ ] Immediately after assignment, write WAL entry
- [ ] Ensure session ID persists even if crash follows quickly
- [ ] Verify WAL entry format

### 2.7 Phase 2 Verification
- [ ] Integration test: spawn "claude" command
- [ ] Verify session ID was injected into actual command
- [ ] Verify session ID stored in pane
- [ ] Verify session ID in WAL/persistence
- [ ] Test: spawn "claude --resume xyz" (no injection)

## Phase 3: Restoration Logic

### 3.1 Implement construct_restore_command()
- [ ] Create function `construct_restore_command(pane: &Pane) -> String`
- [ ] If claude_session_id is Some, use `claude --resume <id>`
- [ ] If claude_working_dir is Some, prepend `cd <dir> &&`
- [ ] If neither, return default pane command
- [ ] Shell-escape directory path
- [ ] Add unit tests

### 3.2 Hook into Pane Restoration Path
- [ ] Locate pane restoration function
- [ ] Check if pane.is_claude_pane is true
- [ ] Call construct_restore_command() for Claude panes
- [ ] Use returned command for PTY spawn
- [ ] Log restoration action for debugging

### 3.3 Handle Restoration Timing
- [ ] Ensure persistence is loaded before restoration
- [ ] Ensure claude_session_id is populated from persistence
- [ ] Handle race conditions if any

### 3.4 Phase 3 Verification
- [ ] Unit test: construct_restore_command() with session ID
- [ ] Unit test: construct_restore_command() with session ID and dir
- [ ] Unit test: construct_restore_command() without session ID
- [ ] Integration test: restart server, check spawned command

## Phase 4: Fallback Handling

### 4.1 Add Configuration Options
- [ ] Add `[claude]` section to config schema
- [ ] Add `auto_resume: bool` option (default: true)
- [ ] Add `resume_fallback: String` option (default: "shell")
- [ ] Document valid values: "shell", "fresh_claude", "error"
- [ ] Parse config in config loading code

### 4.2 Implement Resume Failure Detection
- [ ] Detect Claude exit shortly after spawn (< 2-3 seconds)
- [ ] Check exit code for failure
- [ ] Optionally check stderr for error messages
- [ ] Define what constitutes "resume failed"

### 4.3 Implement Fallback Logic
- [ ] Create enum ClaudeResumeFallback
- [ ] Implement "shell" fallback: spawn default shell
- [ ] Implement "fresh_claude" fallback: spawn bare "claude"
- [ ] Implement "error" fallback: display error, await user
- [ ] Clear claude_session_id on fallback (session is stale)

### 4.4 Logging and Observability
- [ ] Log when session ID is assigned
- [ ] Log when restoration uses --resume
- [ ] Log when resume fails
- [ ] Log fallback action taken
- [ ] Include pane ID in all logs

### 4.5 Phase 4 Verification
- [ ] Test with invalid session ID (expect fallback)
- [ ] Verify each fallback option works correctly
- [ ] Verify logging output is helpful
- [ ] Test config option loading

## Phase 5: MCP Tools (Optional)

### 5.1 fugue_mark_persistent Tool
- [ ] Add tool definition to tools.rs
- [ ] Parameters: pane (required)
- [ ] Mark pane for priority restoration
- [ ] Store flag in pane metadata
- [ ] Update persistence

### 5.2 fugue_get_claude_session Tool
- [ ] Add tool definition to tools.rs
- [ ] Parameters: pane (required)
- [ ] Return claude_session_id if set
- [ ] Return null/error if not a Claude pane

### 5.3 fugue_set_claude_session Tool
- [ ] Add tool definition to tools.rs
- [ ] Parameters: pane (required), session_id (required)
- [ ] Manually set/override session ID
- [ ] Update persistence
- [ ] Use case: adopting existing Claude session

### 5.4 Update fugue_list_panes
- [ ] Include claude_session_id in pane info
- [ ] Include is_claude_pane flag
- [ ] Make it easy to see which panes have sessions

### 5.5 Phase 5 Verification
- [ ] Test each new MCP tool
- [ ] Verify persistence of changes
- [ ] Test list_panes shows Claude info

## Phase 6: Testing

### 6.1 Unit Tests - Command Detection
- [ ] Test is_claude_command with "claude"
- [ ] Test is_claude_command with "claude chat"
- [ ] Test is_claude_command with "cd /foo && claude"
- [ ] Test is_claude_command with "vim" (false)
- [ ] Test is_claude_command with "myclaude" (false)

### 6.2 Unit Tests - Session Flag Detection
- [ ] Test has_session_flag with "--session-id abc"
- [ ] Test has_session_flag with "--resume abc"
- [ ] Test has_session_flag with "--continue"
- [ ] Test has_session_flag with no flags (false)

### 6.3 Unit Tests - Injection
- [ ] Test inject_session_id basic case
- [ ] Test inject_session_id preserves args
- [ ] Test inject_session_id with complex command

### 6.4 Unit Tests - Restoration
- [ ] Test construct_restore_command with session only
- [ ] Test construct_restore_command with session and dir
- [ ] Test construct_restore_command without session

### 6.5 Integration Tests
- [ ] Full cycle: spawn Claude, get session ID
- [ ] Full cycle: restart, verify resume command
- [ ] Fallback test: invalid session
- [ ] Persistence test: session ID survives restart

### 6.6 Manual E2E Tests
- [ ] Start fugue, spawn Claude pane
- [ ] Have brief conversation
- [ ] Kill server
- [ ] Restart server
- [ ] Verify Claude resumes conversation
- [ ] Test with --resume flag (user-specified)
- [ ] Test fallback with deleted Claude session

### 6.7 Regression Tests
- [ ] Run existing test suite
- [ ] Verify non-Claude panes unaffected
- [ ] Verify existing pane operations work
- [ ] Verify persistence backward compatible

## Phase 7: Documentation

### 7.1 Update Configuration Documentation
- [ ] Document [claude] config section
- [ ] Document auto_resume option
- [ ] Document resume_fallback option
- [ ] Add examples

### 7.2 Update User Documentation
- [ ] Explain Claude session persistence feature
- [ ] Document automatic behavior
- [ ] Document how to disable
- [ ] Document fallback behavior

### 7.3 Code Comments
- [ ] Add comments to new Pane fields
- [ ] Add comments to command detection functions
- [ ] Add comments to restoration logic
- [ ] Document non-obvious edge cases

### 7.4 Update CHANGELOG
- [ ] Add entry for FEAT-044
- [ ] Describe user-facing behavior change
- [ ] Note configuration options

## Completion Checklist

- [ ] All Phase 1 tasks complete (Pane Metadata Extension)
- [ ] All Phase 2 tasks complete (Command Detection and Injection)
- [ ] All Phase 3 tasks complete (Restoration Logic)
- [ ] All Phase 4 tasks complete (Fallback Handling)
- [ ] All Phase 5 tasks complete (MCP Tools - optional)
- [ ] All Phase 6 tasks complete (Testing)
- [ ] All Phase 7 tasks complete (Documentation)
- [ ] All unit tests passing
- [ ] All integration tests passing
- [ ] Manual E2E testing complete
- [ ] PLAN.md updated with final implementation details
- [ ] feature_request.json status updated to "completed"
- [ ] Ready for review/merge

---
*Check off tasks as you complete them. Update status field above.*
