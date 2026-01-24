# Implementation Plan: FEAT-007

**Work Item**: [FEAT-007: Protocol Layer - IPC Message Types and Codec](PROMPT.md)
**Component**: fugue-protocol
**Priority**: P1
**Created**: 2026-01-08
**Status**: Completed

## Overview

Client/server IPC message types (ClientMessage, ServerMessage), shared data types (SessionInfo, PaneInfo, ClaudeState), and tokio codec for length-prefixed bincode framing.

## Architecture Decisions

### Message Type Design

ClientMessage and ServerMessage are separate enums to provide type safety and clear direction of communication:

```rust
// Client -> Server
enum ClientMessage {
    CreateSession { name: Option<String>, ... },
    SendInput { session_id: Uuid, data: Vec<u8> },
    Resize { session_id: Uuid, cols: u16, rows: u16 },
    // ...
}

// Server -> Client
enum ServerMessage {
    SessionCreated { session_id: Uuid, info: SessionInfo },
    Output { session_id: Uuid, data: Vec<u8> },
    StateChanged { session_id: Uuid, state: ClaudeState },
    Error { message: String },
    // ...
}
```

### Codec Framing Strategy

Length-prefixed framing with 4-byte big-endian length header:

```
+----------------+------------------+
| Length (4 bytes) | Payload (N bytes) |
+----------------+------------------+
```

- Length field: u32 big-endian, excludes the 4-byte header itself
- Maximum message size: configurable, default 16MB
- Payload: bincode-serialized message

### Serialization Choice

Bincode selected for:
- Compact binary representation
- Fast serialization/deserialization
- Rust-native, no schema files needed
- Well-suited for IPC (not cross-language)

## Affected Components

| Component | Type of Change | Risk Level |
|-----------|----------------|------------|
| fugue-protocol/src/lib.rs | New - message types | Low |
| fugue-protocol/src/codec.rs | New - tokio codec | Low |

## Dependencies

No external work item dependencies. Crate dependencies:
- `serde` with derive feature
- `bincode` for serialization
- `tokio-util` for codec traits
- `bytes` for BytesMut
- `uuid` for session/pane IDs

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Protocol versioning needs | Medium | Medium | Reserve version field in message wrapper |
| Large message handling | Low | Medium | Configurable max size, chunking option |
| Serialization compatibility | Low | Low | Pin bincode version, test round-trips |

## Implementation Phases

### Phase 1: Core Types (Completed)
- Define all message enums
- Define shared data types
- Add serde derives

### Phase 2: Codec (Completed)
- Implement length-prefixed codec
- Integrate bincode serialization
- Add error handling

### Phase 3: Testing (Completed)
- Round-trip tests
- Edge cases
- Performance verification

## Rollback Strategy

If implementation causes issues:
1. Revert commits associated with this work item
2. Verify dependent code does not reference protocol types
3. Document what went wrong in comments.md

## Implementation Notes

Implementation completed. The protocol crate provides:
- `ClientMessage` enum with all client-to-server operations
- `ServerMessage` enum with all server-to-client responses/events
- Shared types: `SessionInfo`, `PaneInfo`, `WindowInfo`, `ClaudeState`, `ClaudeActivity`
- `MessageCodec` implementing tokio's `Encoder` and `Decoder` traits
- Length-prefixed bincode framing for reliable message boundaries

---
*This plan documents the completed implementation.*
