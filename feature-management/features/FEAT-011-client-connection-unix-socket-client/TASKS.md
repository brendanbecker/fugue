# Task Breakdown: FEAT-011

**Work Item**: [FEAT-011: Client Connection - Unix Socket Client](PROMPT.md)
**Status**: Completed
**Last Updated**: 2026-01-08

## Prerequisites

- [x] Read and understand PROMPT.md
- [x] Review PLAN.md and update if needed
- [x] Verify FEAT-007 (Protocol codec) is available
- [x] Review ccmux-utils for error types and utilities

## Design Tasks

- [x] Design ConnectionState enum (Disconnected, Connecting, Connected, Reconnecting)
- [x] Design Connection struct with channels and state
- [x] Design message channel architecture (outgoing/incoming)
- [x] Design MessageHandler trait for incoming messages
- [x] Plan error handling strategy
- [x] Document connection lifecycle

## Implementation Tasks

### Connection Client
- [x] Create ConnectionState enum with Debug, Clone, Copy, PartialEq, Eq
- [x] Create Connection struct with socket_path, state, tx, rx, task_handle
- [x] Implement Connection::new() with default socket path
- [x] Implement Connection::with_socket_path() for custom paths
- [x] Implement Connection::state() getter
- [x] Implement Connection::connect() with state transitions
- [x] Implement Connection::disconnect() with task cleanup
- [x] Implement Connection::send() with connection check
- [x] Implement Connection::recv() async blocking receive
- [x] Implement Connection::try_recv() non-blocking receive
- [x] Implement Connection::sender() to get clonable MessageSender
- [x] Implement connection_task background worker
- [x] Implement Default trait for Connection

### Message Handler
- [x] Create MessageSender struct with Clone derive
- [x] Implement MessageSender::new()
- [x] Implement MessageSender::send() async with error handling
- [x] Implement MessageSender::send_nowait() fire-and-forget
- [x] Create MessageHandler trait with handle(), on_connected(), on_disconnected()
- [x] Create CallbackHandler struct for simple callbacks
- [x] Implement CallbackHandler::new()
- [x] Implement MessageHandler for CallbackHandler

### Error Handling
- [x] Use ServerNotRunning error when socket doesn't exist
- [x] Use Connection error for connect failures
- [x] Use ConnectionClosed error for channel send failures
- [x] Handle stream end gracefully in connection_task
- [x] Handle receive errors in connection_task

## Testing Tasks

### Connection Client Tests
- [x] test_connection_state_initial - verify initial Disconnected state
- [x] test_connect_no_server - error when socket doesn't exist
- [x] test_connect_to_server - successful connection with mock server
- [x] test_connect_already_connected - no-op when already connected
- [x] test_send_not_connected - error when sending without connection
- [x] test_disconnect - verify state transition
- [x] test_connection_default - Default trait implementation
- [x] test_connection_state_debug - Debug trait
- [x] test_connection_state_clone - Clone trait
- [x] test_connection_state_copy - Copy trait
- [x] test_try_recv_empty - empty channel returns None
- [x] test_sender_returns_message_sender - sender() method works
- [x] test_with_socket_path_sets_path - custom path is set
- [x] test_disconnect_when_not_connected - safe to call
- [x] test_state_transitions_on_failed_connect - state returns to Disconnected
- [x] test_connection_state_equality - PartialEq trait

### Message Handler Tests
- [x] test_message_sender_clone - Clone trait
- [x] test_callback_handler - basic callback invocation
- [x] test_message_sender_send_success - async send works
- [x] test_message_sender_send_channel_closed - error on closed channel
- [x] test_message_sender_send_nowait - fire-and-forget works
- [x] test_message_sender_send_nowait_channel_full - silently drops
- [x] test_message_sender_send_nowait_channel_closed - silently fails
- [x] test_callback_handler_receives_messages - counter increments
- [x] test_callback_handler_different_message_types - various ServerMessage types
- [x] test_message_handler_on_connected - lifecycle callback
- [x] test_message_handler_on_disconnected - lifecycle callback
- [x] test_callback_handler_default_on_connected - no panic
- [x] test_callback_handler_default_on_disconnected - no panic
- [x] test_message_sender_new - constructor works
- [x] test_callback_handler_is_send - Send trait bound satisfied

## Documentation Tasks

- [x] Add module-level documentation to client.rs
- [x] Add module-level documentation to handler.rs
- [x] Document public structs and methods
- [x] Create PROMPT.md with requirements and status
- [x] Create PLAN.md with architecture decisions
- [x] Create TASKS.md with task breakdown

## Verification Tasks

- [x] All acceptance criteria from PROMPT.md met
- [x] All tests passing
- [x] Update feature_request.json status to completed
- [x] Code compiles without warnings

## Completion Checklist

- [x] All implementation tasks complete
- [x] All tests passing
- [x] Documentation updated
- [x] PLAN.md reflects final implementation
- [x] Ready for review/merge

---
*All tasks completed.*
