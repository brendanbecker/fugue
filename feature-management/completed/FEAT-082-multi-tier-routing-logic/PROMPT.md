# FEAT-082: Multi-tier routing logic

## Metadata
- **ID**: FEAT-082
- **Title**: Multi-tier routing logic
- **Type**: Enhancement
- **Priority**: P2
- **Estimated Effort**: Medium
- **Component**: ccmux-client

## Description
Logic to route commands and connections across multiple ccmux tiers (local Mayor -> remote Polecat) to enable hybrid orchestration workflows. Includes intelligent target selection based on configuration or environment (e.g., GASTOWN_CCMUX_ADDR).

## Benefits
Enables distributed workflows where heavy tasks run on remote servers while keeping the control plane local.

## Dependencies
FEAT-070 (Gastown remote support), FEAT-066 (TCP listener)

## Implementation Plan

### 1. Client Configuration
- [x] Add `remotes` section to `ccmux-client` config (config.toml).
- [x] Implement `resolve_remote` function to look up addresses by name.

### 2. Client CLI
- [x] Add `--target <NAME>` flag to `ccmux-client`.
- [x] Implement address resolution logic in `main.rs`.
- [x] Add conditional auto-start logic (don't start local daemon if connecting to remote).

### 3. Testing
- [x] Add unit tests for config parsing.
- [x] Manual verification of CLI flags.

## Usage
```toml
# ~/.config/ccmux/config.toml
[remotes]
gaming-pc = "tcp://192.168.1.5:9999"
cloud-gpu = "tcp://203.0.113.10:9999"
```

```bash
# Connect to remote
ccmux --target gaming-pc

# Run command on remote
ccmux --target cloud-gpu claude --resume
```