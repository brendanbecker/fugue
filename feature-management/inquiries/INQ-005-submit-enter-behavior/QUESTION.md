# INQ-005: Submit/Enter Behavior in fugue Input Tools

## Problem Statement

During orchestration setup, we observed that Enter key presses may not be reliably sent through fugue input mechanisms:

1. **`fugue_send_input` with `submit: true`** - When sending a prompt to a Claude agent pane with `submit: true`, the Enter may not be sent, leaving the message in the input buffer unsubmitted.

2. **Watchdog timer carriage return** - The native watchdog timer (BUG-054 workaround) is supposed to send the message text, wait 50ms, then send `\r`. This mechanism may not be functioning after daemon rebuilds or may have race conditions.

## Observed Behavior

- Sent initial prompt to watchdog pane with `submit: true` - unclear if Enter was sent
- Watchdog timer sent "check" message but it sat in the input buffer without submitting
- Manual `fugue_send_input` with `key: "Enter"` was required to submit

## Research Questions

### Area 1: `fugue_send_input` Submit Parameter

- How is `submit: true` implemented in the MCP handler?
- Does it send `\r`, `\n`, or both?
- Is there a timing issue between message and Enter?
- Are there edge cases where submit fails (e.g., pane state, TUI mode)?

### Area 2: Watchdog Timer Carriage Return

- Review `watchdog.rs` implementation of the 50ms delay + `\r` send
- Is the delay sufficient for Claude Code TUI?
- Are there race conditions with the pane's input buffer?
- Does rebuilding the daemon affect timer behavior?

### Area 3: Claude Code TUI Input Handling

- How does Claude Code's TUI process incoming characters?
- Does it buffer input or process character-by-character?
- Are there scenarios where `\r` is not recognized as Enter?

## Success Criteria

- Clear understanding of current implementation
- Identification of any bugs or race conditions
- Recommendation: fix bugs, adjust timing, or document expected behavior

## Constraints

- Focus on fugue codebase only
- Single session time budget
- May result in BUG or FEAT if issues found
