# FEAT-097: Orchestration Message Receive

## Goal
Implement the receiving side of the orchestration protocol, allowing workers to report status and poll for messages directed to them.

## Requirements

### MCP Tools
1.  **`ccmux_get_worker_status`**
    *   Retrieves the current status of a specific worker (or all workers if no ID provided).
    *   Input: `worker_id` (optional string).
    *   Output: JSON object with worker status, current task, etc.

2.  **`ccmux_poll_messages`**
    *   Allows a worker to check for new messages in its inbox.
    *   Input: `worker_id` (string, required).
    *   Output: List of messages (and potentially clears them or marks them read).

### Backend Changes
1.  **Daemon Memory Store**:
    *   Implement a mechanism to store worker status in the daemon's memory.
    *   Ensure this status is updated when workers report it (presumably via an existing or new mechanism, but `get_worker_status` implies reading it).
    *   *Correction*: The prompt implies we need `get_worker_status`. Wait, usually "worker status" is reported *by* the worker via `report_status` (which exists? `ccmux_report_status` is in the tool list in the system prompt).
    *   So `get_worker_status` is for the *orchestrator* to check on workers.

2.  **Message Routing/Inboxes**:
    *   Implement "inboxes" for messages.
    *   When `ccmux_send_orchestration` (existing tool) sends a message to a target, it should land in an inbox if it's not a broadcast that's immediately handled.
    *   `ccmux_poll_messages` retrieves from this inbox.

## Implementation Details
-   Update `ccmux-server` to handle these new MCP tools.
-   Add necessary data structures for inboxes and status storage in `ccmux-server`.
-   Ensure thread safety for the in-memory store.
