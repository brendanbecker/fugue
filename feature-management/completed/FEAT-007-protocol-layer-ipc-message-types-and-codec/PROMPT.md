# FEAT-007: Protocol Layer - IPC Message Types and Codec

**Priority**: P1
**Component**: fugue-protocol
**Type**: new_feature
**Estimated Effort**: medium
**Business Value**: high
**Status**: completed

## Overview

Client/server IPC message types (ClientMessage, ServerMessage), shared data types (SessionInfo, PaneInfo, ClaudeState), and tokio codec for length-prefixed bincode framing.

## Requirements

### ClientMessage Enum
- Enum for all client-to-server messages
- Serializable/deserializable with serde
- Covers all client operations (create session, send input, resize, etc.)

### ServerMessage Enum
- Enum for all server-to-client messages
- Serializable/deserializable with serde
- Covers all server responses and events (output, state changes, errors, etc.)

### Shared Data Types
- **SessionInfo**: Session metadata (id, name, state, etc.)
- **PaneInfo**: Pane metadata (id, dimensions, position, etc.)
- **WindowInfo**: Window metadata (id, layout, panes, etc.)
- **ClaudeState**: Claude Code activity state
- **ClaudeActivity**: Detailed Claude activity information

### Tokio Codec
- Length-prefixed framing for message boundaries
- Bincode serialization for efficient binary encoding
- Tokio-compatible codec implementation
- Support for both encode and decode operations

### Serialization/Deserialization
- serde Serialize/Deserialize derives on all types
- Efficient binary serialization with bincode
- Support for versioning/forward compatibility considerations

## Affected Files

- `fugue-protocol/src/lib.rs` - Main protocol types and exports
- `fugue-protocol/src/codec.rs` - Tokio codec implementation

## Implementation Tasks

### Section 1: Design
- [x] Define ClientMessage variants
- [x] Define ServerMessage variants
- [x] Design shared data type structures
- [x] Plan codec framing strategy

### Section 2: Core Types Implementation
- [x] Implement ClientMessage enum
- [x] Implement ServerMessage enum
- [x] Implement SessionInfo struct
- [x] Implement PaneInfo struct
- [x] Implement WindowInfo struct
- [x] Implement ClaudeState enum
- [x] Implement ClaudeActivity struct

### Section 3: Codec Implementation
- [x] Create length-prefixed frame codec
- [x] Implement bincode serialization integration
- [x] Implement Encoder trait
- [x] Implement Decoder trait
- [x] Add error handling for codec operations

### Section 4: Testing
- [x] Unit tests for message serialization
- [x] Unit tests for codec encode/decode
- [x] Round-trip tests for all message types
- [x] Edge case tests (empty messages, large payloads)

### Section 5: Documentation
- [x] Document message types and their purposes
- [x] Document codec usage
- [x] Add code examples

## Acceptance Criteria

- [x] ClientMessage enum covers all client operations
- [x] ServerMessage enum covers all server responses/events
- [x] All shared types are properly defined
- [x] Codec correctly frames messages with length prefix
- [x] Bincode serialization works correctly
- [x] All types implement serde Serialize/Deserialize
- [x] Tests verify round-trip correctness
- [x] Documentation is complete

## Dependencies

None - this is a foundational feature.

## Notes

- Protocol versioning may be needed for future compatibility
- Consider adding compression for large messages in future
- Length prefix should use a consistent byte order (big-endian recommended)
- Maximum message size should be configurable to prevent memory exhaustion
