# FEAT-092: Refactor fugue-protocol/src/messages.rs

**Priority**: P3
**Component**: fugue-protocol
**Type**: refactor
**Estimated Effort**: low-medium
**Current Size**: 15.1k tokens (2010 lines)
**Target Size**: <10k tokens per module

## Overview

The protocol messages file has grown to 15.1k tokens. It contains all client-server message definitions. Split into logical message groups.

## Current Structure Analysis

The file likely contains:
- ClientMessage enum (messages from client to server)
- ServerMessage enum (messages from server to client)
- Request/Response pairs for each operation
- Broadcast message types
- MCP-specific message wrappers
- Sequenced message wrapper (for persistence)

## Proposed Module Structure

```
fugue-protocol/src/messages/
├── mod.rs              # Re-exports, top-level enums (<3k)
├── client.rs           # ClientMessage variants
├── server.rs           # ServerMessage variants
├── requests.rs         # Request structs
├── responses.rs        # Response structs
└── broadcast.rs        # Broadcast message types
```

## Alternative: Keep Single File

At 15.1k tokens, this is borderline. An alternative is to:
- Add better section comments
- Group related messages together
- Consider if splitting adds value

## Refactoring Steps

1. **Audit message types** - List all variants
2. **Identify groupings** - Client vs Server vs shared
3. **Extract carefully** - Messages are tightly coupled
4. **Ensure derive macros work** - Serialize/Deserialize across modules

## Acceptance Criteria

- [ ] Messages organized logically
- [ ] Serialization/deserialization unchanged
- [ ] All protocol tests pass
- [ ] Bincode compatibility preserved

## Testing

- Protocol serialization tests
- Integration tests
- Verify message round-trips

## Notes

- Protocol messages are critical - be careful with serialization
- Consider keeping as single file with better organization if split doesn't add value
- At 15.1k this is the lowest priority of the refactoring features
