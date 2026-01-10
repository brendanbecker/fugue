# Task Breakdown: FEAT-017

**Work Item**: [FEAT-017: Configuration - TOML Config with Hot Reload](PROMPT.md)
**Status**: Completed
**Last Updated**: 2026-01-08

## Prerequisites

- [x] Read and understand PROMPT.md
- [x] Review PLAN.md and update if needed
- [x] Review toml and serde crate documentation
- [x] Review notify crate documentation
- [x] Review arc-swap crate documentation

## Design Tasks

- [x] Design Config struct hierarchy
- [x] Design validation approach
- [x] Design hot-reload event flow
- [x] Design subscriber notification mechanism
- [x] Document XDG path resolution

## Implementation Tasks

### Configuration Schema
- [x] Create Config struct with all sections
- [x] Create ServerConfig struct
- [x] Create SessionConfig struct
- [x] Create PtyConfig struct
- [x] Create LoggingConfig struct
- [x] Implement Deserialize/Serialize for all types
- [x] Implement Default for all types
- [x] Add validation methods

### TOML Parsing
- [x] Implement load_from_file() function
- [x] Implement load_from_string() function
- [x] Handle missing file gracefully
- [x] Handle parse errors with context
- [x] Merge partial config with defaults

### XDG Compliance
- [x] Implement config_path() function
- [x] Check CCMUX_CONFIG env var
- [x] Check XDG_CONFIG_HOME
- [x] Check ~/.config/ccmux/
- [x] Check /etc/ccmux/
- [x] Return first existing path or default

### Hot Reload
- [x] Set up notify file watcher
- [x] Watch config file for modifications
- [x] Implement debounce for rapid changes
- [x] Reload config on change
- [x] Validate before applying
- [x] Log old and new config values
- [x] Handle watcher errors gracefully

### Lock-Free Access
- [x] Create static ArcSwap<Config>
- [x] Implement Config::current() accessor
- [x] Implement Config::update() method
- [x] Implement change subscriber registration
- [x] Notify subscribers on update

## Testing Tasks

- [x] Unit test: Default config values
- [x] Unit test: Config validation
- [x] Unit test: TOML parsing success
- [x] Unit test: TOML parsing errors
- [x] Unit test: Partial config merge
- [x] Integration test: XDG path resolution
- [x] Integration test: Hot-reload cycle
- [x] Integration test: Concurrent access

## Documentation Tasks

- [x] Document all config options
- [x] Document XDG path behavior
- [x] Document hot-reload behavior
- [x] Add example config file

## Verification Tasks

- [x] All acceptance criteria from PROMPT.md met
- [x] Tests passing
- [x] Update feature_request.json status
- [x] Document completion in PLAN.md

## Completion Checklist

- [x] All implementation tasks complete
- [x] All tests passing
- [x] Documentation updated
- [x] PLAN.md reflects final implementation
- [x] Ready for review/merge

---
*All tasks completed 2026-01-08.*
