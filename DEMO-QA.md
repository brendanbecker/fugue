# ccmux Demo QA Orchestrator

You are running inside ccmux, a terminal multiplexer with MCP tools. You have access to tools that let you control the terminal environment: spawn panes, send input, read output, check status, and navigate.

Your job: Run a self-guided demo showcasing ccmux's capabilities **while actively QA testing**. Move at a readable pace - pause 2-3 seconds between major actions so a viewer can follow.

## QA Protocol

**CRITICAL**: This is a QA run, not just a demo. When something doesn't work as expected:

1. **STOP** - Do not proceed to the next step
2. **Document** - Create a bug report in `feature-management/bugs/` following the existing format
3. **Report** - Tell the viewer what went wrong and that you've filed a bug
4. **Adapt** - Try an alternative approach or skip to the next act if blocked

### Bug Report Location

Create new bugs in: `feature-management/bugs/BUG-XXX-short-description/`

Each bug needs:
- `PROMPT.md` - Description of the bug, steps to reproduce, expected vs actual behavior
- `bug_report.json` - Structured metadata (priority, component, status)

Use the next available BUG number (check existing bugs first).

### What Counts as a Bug

- MCP tool returns an error
- MCP tool succeeds but behavior doesn't match description
- Pane doesn't appear after create
- Input doesn't arrive in target pane
- Focus doesn't switch
- Status returns unexpected/wrong state
- Any crash, hang, or timeout
- Output is garbled or missing

## Demo Script

### Act 1: Introduction (speak to the viewer)

Say: "I'm Claude, running inside ccmux in QA mode. I'll demo the MCP tools and file bugs for anything that doesn't work. Let's see how solid this is."

### Act 2: Spawn a Worker

1. Say: "First, I'll spawn a background pane to run ccmux's test suite."
2. Call `ccmux_create_pane` to create a new pane (split right or below)
3. **QA CHECK**: Did the tool return success? Did a pane actually appear?
4. If failed: File bug, report to viewer, attempt workaround
5. Say: "Pane created. Now I'll start the tests."
6. Call `ccmux_send_input` to that pane: `cargo test 2>&1\n`
7. **QA CHECK**: Did send_input return success?
8. Say: "Tests are running. I can monitor them without leaving this conversation."

### Act 3: Monitor the Background Process

1. Wait 3 seconds
2. Call `ccmux_read_pane` on the test pane
3. **QA CHECK**: Did read_pane return content? Is it the expected output from cargo test?
4. If empty or wrong pane: File bug
5. Report what you see: "I can see [X] tests have run so far..." or "Compiling dependencies..." - whatever the output shows
6. Say: "I'll keep checking while we talk."

### Act 4: Show State Detection

1. Call `ccmux_get_status` on the test pane
2. **QA CHECK**: Does status return? Is the state reasonable (not Unknown/Crashed when process is running)?
3. If state seems wrong: File bug with expected vs actual
4. Report the cognitive state: "ccmux detects the pane state as [running/idle/etc]"
5. Say: "This is how an orchestrator knows which agents need attention."

### Act 5: Spawn a Second Worker

1. Say: "Let me spawn another pane - this one will show the git log."
2. Call `ccmux_create_pane` for a third pane
3. **QA CHECK**: Pane created successfully?
4. Call `ccmux_send_input`: `git log --oneline -20\n`
5. **QA CHECK**: Input sent successfully?
6. Say: "Now I have two background tasks I'm managing."

### Act 6: Navigation

1. Say: "I can navigate to any pane."
2. Call `ccmux_focus_pane` to switch to the test pane
3. **QA CHECK**: Did focus change? Tool return success?
4. Pause 2 seconds (viewer sees the switch)
5. Call `ccmux_focus_pane` to switch back to the original pane
6. **QA CHECK**: Focus returned?
7. Say: "Agents don't need humans to switch contexts for them."

### Act 7: Check Test Results

1. Call `ccmux_read_pane` on the test pane
2. **QA CHECK**: Can we still read the pane after focus changes?
3. If tests are still running, report progress
4. If tests finished, report the result: "Tests complete - [passed/failed] with [summary]"

### Act 8: QA Summary

1. Say: "QA run complete."
2. Summarize: "I tested [N] MCP operations. [X] worked as expected. [Y] bugs filed."
3. List any bugs filed with their IDs
4. If all passed: "Clean run - all MCP tools working as documented."

### Act 9: Wrap Up

Say: "That's ccmux under QA. MCP tools for terminal control. When things break, we catch them and file bugs. That's how you build reliable agent infrastructure."

Say: "Check it out at github.com/brendanbecker/ccmux"

## Rules

- Be conversational, not robotic
- Narrate what you're doing before you do it
- After each MCP call, briefly confirm what happened
- **If something fails, STOP and file a bug before continuing**
- If completely blocked, skip to next act and note what was skipped
- Keep total runtime under 3 minutes (QA takes longer than demo)
- You're testing, not just showing off - be honest about what works and what doesn't

## MCP Tools Available (18 total)

**Sessions**
- `ccmux_list_sessions` - List all sessions with metadata
- `ccmux_create_session` - Create a new session
- `ccmux_rename_session` - Rename a session for easier identification
- `ccmux_select_session` - Switch to a different session

**Windows**
- `ccmux_list_windows` - List windows in a session
- `ccmux_create_window` - Create a new window
- `ccmux_select_window` - Switch to a different window

**Panes**
- `ccmux_list_panes` - List all panes with metadata
- `ccmux_create_pane` - Create a new pane (split)
- `ccmux_close_pane` - Close a pane
- `ccmux_focus_pane` - Focus a specific pane

**I/O**
- `ccmux_read_pane` - Read output buffer from pane
- `ccmux_send_input` - Send keystrokes to pane (use `\n` for Enter)
- `ccmux_get_status` - Get pane state (shell, Claude, etc.)

**Layouts**
- `ccmux_create_layout` - Create complex layouts declaratively
- `ccmux_split_pane` - Split a pane with custom ratio
- `ccmux_resize_pane` - Resize a pane dynamically

**Note**: `ccmux_get_status` returns Claude-specific state detection: Idle, Thinking, ToolExecution, Streaming, Complete, Crashed

## Begin

Start the QA demo now. Remember: test thoroughly, file bugs immediately, then continue.
