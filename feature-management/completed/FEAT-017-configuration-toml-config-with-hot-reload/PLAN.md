# Implementation Plan: FEAT-017

**Work Item**: [FEAT-017: Configuration - TOML Config with Hot Reload](PROMPT.md)
**Component**: fugue-server
**Priority**: P2
**Created**: 2026-01-08
**Status**: Completed

## Overview

TOML configuration with hot-reload via notify, lock-free access using ArcSwap, and validation.

## Architecture Decisions

### Configuration Schema

The configuration is split into logical sections:

```rust
pub struct Config {
    pub server: ServerConfig,
    pub session: SessionConfig,
    pub pty: PtyConfig,
    pub logging: LoggingConfig,
}
```

### Hot Reload Architecture

```
File System Event -> notify watcher -> debounce timer ->
  parse TOML -> validate -> ArcSwap::store() -> notify subscribers
```

### Lock-Free Access Pattern

Using ArcSwap for configuration access:

```rust
static CONFIG: Lazy<ArcSwap<Config>> = Lazy::new(|| {
    ArcSwap::from_pointee(Config::default())
});

pub fn current() -> Guard<Arc<Config>> {
    CONFIG.load()
}
```

### XDG Path Resolution Order

1. `$FUGUE_CONFIG` environment variable
2. `$XDG_CONFIG_HOME/fugue/config.toml`
3. `~/.config/fugue/config.toml`
4. `/etc/fugue/config.toml`
5. Built-in defaults

## Affected Components

| Component | Type of Change | Risk Level |
|-----------|----------------|------------|
| fugue-server/src/config/loader.rs | New - Config loading and hot-reload | Medium |
| fugue-server/src/config/schema.rs | New - Configuration schema | Low |
| fugue-server/src/config/mod.rs | New - Module exports | Low |

## Dependencies

- `toml` crate for TOML parsing
- `serde` for serialization/deserialization
- `notify` crate for file watching
- `arc-swap` for lock-free config access
- `dirs` crate for XDG path resolution
- FEAT-008 for error types and logging

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Config parse errors | Medium | Low | Graceful fallback to previous config |
| File watcher failures | Low | Medium | Log errors, continue with current config |
| Race conditions | Low | Medium | ArcSwap provides atomic operations |
| Invalid config values | Medium | Medium | Comprehensive validation on load |

## Implementation Phases

### Phase 1: Configuration Schema (Completed)
- Config struct definitions
- Serde derives
- Default implementations
- Validation methods

### Phase 2: TOML Parsing (Completed)
- File loading
- Parse error handling
- Merge with defaults
- Validation integration

### Phase 3: XDG Paths (Completed)
- Path resolution logic
- Environment variable support
- Fallback chain

### Phase 4: Hot Reload (Completed)
- File watcher setup
- Change detection
- Debouncing
- Error handling

### Phase 5: Lock-Free Access (Completed)
- ArcSwap integration
- Accessor methods
- Change notification

## Rollback Strategy

If implementation causes issues:
1. Revert commits associated with this work item
2. Remove config module from fugue-server
3. Document what went wrong in comments.md

## Testing Strategy

1. **Unit Tests**: Schema validation, default values
2. **Unit Tests**: TOML parsing, error handling
3. **Integration Tests**: Hot-reload cycle
4. **Integration Tests**: XDG path resolution

## Implementation Notes

Implementation completed. Key decisions made:
- Used notify v6 for async file watching
- ArcSwap provides lock-free reads
- Debounce window of 100ms for rapid changes
- Validation runs before any config update

---
*Implementation completed 2026-01-08.*
