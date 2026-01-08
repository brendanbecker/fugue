# FEAT-017: Configuration - TOML Config with Hot Reload

**Priority**: P2
**Component**: ccmux-server
**Type**: new_feature
**Estimated Effort**: medium
**Business Value**: medium
**Status**: completed

## Overview

TOML configuration with hot-reload via notify, lock-free access using ArcSwap, and validation.

## Requirements

- TOML configuration file parsing
- Hot-reload using notify crate for file watching
- Lock-free config access via ArcSwap
- Configuration validation on load
- Default values for all settings
- XDG-compliant config file location
- Schema definition for all config options

## Affected Files

- `ccmux-server/src/config/loader.rs`
- `ccmux-server/src/config/schema.rs`
- `ccmux-server/src/config/mod.rs`

## Implementation Tasks

### Section 1: Design
- [x] Review TOML parsing crate options (toml, serde)
- [x] Design configuration schema structure
- [x] Design hot-reload architecture with notify
- [x] Plan ArcSwap integration for lock-free access

### Section 2: Configuration Schema
- [x] Define all configuration options in schema.rs
- [x] Implement Deserialize/Serialize for config types
- [x] Add validation logic for config values
- [x] Implement Default trait with sensible defaults
- [x] Document all configuration options

### Section 3: TOML Parsing
- [x] Implement TOML file loading
- [x] Handle missing config file (use defaults)
- [x] Handle parse errors gracefully
- [x] Merge file config with defaults

### Section 4: XDG Compliance
- [x] Implement XDG config path detection
- [x] Support ~/.config/ccmux/config.toml
- [x] Support /etc/ccmux/config.toml fallback
- [x] Support CCMUX_CONFIG env var override

### Section 5: Hot Reload
- [x] Implement file watcher using notify crate
- [x] Detect config file changes
- [x] Reload and validate on change
- [x] Handle reload errors without crashing
- [x] Log configuration changes

### Section 6: Lock-Free Access
- [x] Wrap config in ArcSwap
- [x] Implement Config::current() accessor
- [x] Ensure atomic config updates
- [x] Provide subscription mechanism for change notifications

### Section 7: Testing
- [x] Unit tests for schema validation
- [x] Unit tests for TOML parsing
- [x] Integration tests for hot-reload
- [x] Test XDG path resolution

## Acceptance Criteria

- [x] TOML configuration file is properly parsed
- [x] Hot-reload detects and applies config changes
- [x] Lock-free config access works correctly
- [x] All config values are validated
- [x] Default values are sensible
- [x] XDG paths are correctly resolved
- [x] All tests passing

## Dependencies

- FEAT-008: Utilities - Error Types, Logging, and Path Helpers

## Notes

- notify crate provides cross-platform file watching
- ArcSwap enables lock-free reads with atomic swaps
- Consider debouncing rapid config file changes
- Log both old and new values on config change for debugging
