# Task Breakdown: FEAT-007

**Work Item**: [FEAT-007: Protocol Layer - IPC Message Types and Codec](PROMPT.md)
**Status**: Completed
**Last Updated**: 2026-01-08

## Prerequisites

- [x] Read and understand PROMPT.md
- [x] Review PLAN.md and update if needed
- [x] Review tokio-util codec patterns
- [x] Review bincode serialization options

## Design Tasks

- [x] Define ClientMessage enum variants
- [x] Define ServerMessage enum variants
- [x] Design SessionInfo struct fields
- [x] Design PaneInfo struct fields
- [x] Design WindowInfo struct fields
- [x] Design ClaudeState enum variants
- [x] Design ClaudeActivity struct fields
- [x] Choose length prefix format (u32 big-endian)
- [x] Document codec framing strategy

## Implementation Tasks

### Message Types (fugue-protocol/src/lib.rs)
- [x] Create ClientMessage enum with variants:
  - [x] CreateSession
  - [x] AttachSession
  - [x] DetachSession
  - [x] ListSessions
  - [x] SendInput
  - [x] Resize
  - [x] GetSessionInfo
  - [x] CloseSession
- [x] Create ServerMessage enum with variants:
  - [x] SessionCreated
  - [x] SessionAttached
  - [x] SessionDetached
  - [x] SessionList
  - [x] SessionInfo
  - [x] Output
  - [x] StateChanged
  - [x] SessionClosed
  - [x] Error

### Shared Data Types (fugue-protocol/src/lib.rs)
- [x] Implement SessionInfo struct
- [x] Implement PaneInfo struct
- [x] Implement WindowInfo struct
- [x] Implement ClaudeState enum
- [x] Implement ClaudeActivity struct
- [x] Add serde derives to all types
- [x] Add Debug, Clone derives as appropriate

### Codec Implementation (fugue-protocol/src/codec.rs)
- [x] Create MessageCodec struct
- [x] Implement Encoder<ClientMessage> for MessageCodec
- [x] Implement Decoder for MessageCodec (returns ServerMessage)
- [x] Implement length prefix encoding (4-byte big-endian)
- [x] Implement length prefix decoding
- [x] Add bincode serialization in encode
- [x] Add bincode deserialization in decode
- [x] Handle partial reads correctly
- [x] Add max message size check
- [x] Define and handle codec errors

### Error Handling
- [x] Create CodecError enum
- [x] Implement std::error::Error for CodecError
- [x] Handle serialization errors
- [x] Handle deserialization errors
- [x] Handle IO errors
- [x] Handle message too large errors

## Testing Tasks

- [x] Unit test: ClientMessage serialization round-trip
- [x] Unit test: ServerMessage serialization round-trip
- [x] Unit test: SessionInfo serialization
- [x] Unit test: Codec encode produces correct length prefix
- [x] Unit test: Codec decode handles partial data
- [x] Unit test: Codec decode handles complete messages
- [x] Unit test: Error handling for oversized messages
- [x] Integration test: Multiple messages through codec

## Documentation Tasks

- [x] Document ClientMessage variants
- [x] Document ServerMessage variants
- [x] Document shared types
- [x] Document codec usage example
- [x] Add module-level documentation

## Verification Tasks

- [x] All acceptance criteria from PROMPT.md met
- [x] Tests passing
- [x] Update feature_request.json status to completed
- [x] Document completion

## Completion Checklist

- [x] All implementation tasks complete
- [x] All tests passing
- [x] Documentation complete
- [x] PLAN.md reflects final implementation
- [x] Ready for use by dependent features

---
*All tasks completed. Feature implementation is done.*
