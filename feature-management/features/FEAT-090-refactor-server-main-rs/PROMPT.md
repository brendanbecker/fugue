# FEAT-090: Refactor fugue-server/src/main.rs

**Priority**: P3
**Component**: fugue-server
**Type**: refactor
**Estimated Effort**: medium
**Current Size**: 18.3k tokens (1966 lines)
**Target Size**: <8k tokens for main.rs

## Overview

The server's main.rs has grown to 18.3k tokens. While some complexity in main.rs is expected, much of this can be extracted into dedicated modules for server initialization, signal handling, and subsystem coordination.

## Current Structure Analysis

The file likely contains:
- CLI argument parsing
- Configuration loading
- Logging/tracing setup
- Server initialization
- Subsystem spawning (PTY poller, MCP bridge, etc.)
- Signal handling (SIGTERM, SIGINT)
- Graceful shutdown coordination
- Socket listener setup
- Main event loop

## Proposed Module Structure

```
fugue-server/src/
├── main.rs             # Entry point, CLI, high-level coordination (<5k)
├── init.rs             # Server initialization, config loading
├── signals.rs          # Signal handling, shutdown coordination
├── subsystems.rs       # Subsystem spawning and management
└── server.rs           # Main server struct and event loop (may exist)
```

## Refactoring Steps

1. **Identify initialization code** - Config, logging, socket setup
2. **Extract signal handling** - Separate module for clean shutdown
3. **Extract subsystem management** - PTY poller, MCP bridge spawning
4. **Keep main.rs lean** - Just CLI and coordination

## Acceptance Criteria

- [ ] `main.rs` reduced to <8k tokens
- [ ] Server starts and runs identically
- [ ] Graceful shutdown works
- [ ] All subsystems spawn correctly
- [ ] Logging unchanged

## Testing

- Server startup/shutdown tests
- Signal handling tests (if any)
- Full integration test suite

## Notes

- main.rs is special - some frameworks expect certain things there
- Keep CLI parsing in main.rs (clap integration)
- This is a pure refactor
