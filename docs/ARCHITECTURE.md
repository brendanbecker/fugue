# fugue Architecture

> **Moved**: The full architecture documentation is now in [docs/architecture/](./architecture/).

## Quick Links

- [**ARCHITECTURE.md**](./architecture/ARCHITECTURE.md) - System overview, client-server model, data flow
- [**CRATE_STRUCTURE.md**](./architecture/CRATE_STRUCTURE.md) - Rust workspace organization
- [**CLAUDE_INTEGRATION.md**](./architecture/CLAUDE_INTEGRATION.md) - Claude Code detection and communication
- [**PERSISTENCE.md**](./architecture/PERSISTENCE.md) - Crash recovery and state management
- [**CONFIGURATION.md**](./architecture/CONFIGURATION.md) - Hot-reload configuration system

## Architecture Decision Records

- [ADR-001: Terminal Parser Selection](./architecture/ADR/001-terminal-parser.md)
- [ADR-002: Claude Communication Protocol](./architecture/ADR/002-claude-communication.md)
- [ADR-003: Session Isolation Strategy](./architecture/ADR/003-session-isolation.md)

## Key Decisions Summary

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Architecture | Client-server | Crash isolation, detach/attach |
| Terminal Parser | vt100 (start) | Simpler API, migration path |
| Claude Communication | MCP + Sideband | Different use cases |
| Session Isolation | CLAUDE_CONFIG_DIR | Preserves shell environment |
| Persistence | Checkpoint + WAL | Balance durability/performance |
| Config Reload | ArcSwap | Lock-free 60fps rendering |
