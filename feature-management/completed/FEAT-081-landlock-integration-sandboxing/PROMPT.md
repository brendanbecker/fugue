# FEAT-081: Optional Landlock Integration for Per-Pane Sandboxing

## Overview
Provide unprivileged, fine-grained filesystem and network sandboxing for individual panes using **Linux Landlock** (kernel LSM). This allows users and agents to create strongly isolated environments without root privileges or external tools.

This feature is **optional**:
- Disabled by default.
- Only available on supported Linux kernels (5.13+).
- Gracefully falls back if unsupported.

## Motivation
- **Security**: Hard kernel-enforced limits on filesystem/network access.
- **Unprivileged**: No sudo or capabilities required.
- **Agent Safety**: Allow agents to request isolated sandboxes for untrusted tasks (e.g., analyzing malicious input).

## Requirements
- Add optional `landlock` dependency to `ccmux-server`.
- Extend per-pane configuration to support Landlock rules (fs paths, network ports).
- Implement runtime application of ruleset before spawning child process.
- Fallback gracefully if kernel support is missing.

## Design
### Config Keys
Extend the `config` object from FEAT-080:
```json
{
  "landlock": {
    "enabled": true,
    "fs_read_paths": ["/home/user/allowed-read", "/usr/bin"],
    "fs_write_paths": ["/tmp/sandbox-xyz"],
    "net_connect_ports": [443, 80]
  }
}
```

### Runtime Logic
- Check `landlock::is_supported()`.
- Build `Ruleset` from config.
- Call `ruleset.restrict_self()` after setting cwd/env but before spawning.

## Tasks
### Section 1: Dependencies & Configuration
- [ ] Add `landlock` crate as optional dependency.
- [ ] Define `LandlockConfig` struct in `ccmux-server`.

### Section 2: Implementation
- [ ] Implement `apply_landlock(config)` function in spawn logic.
- [ ] Map FS paths and network ports to Landlock rules.
- [ ] Handle `restrict_self` and error reporting.

### Section 3: Integration
- [ ] Integrate with sideband/MCP config (FEAT-080).
- [ ] Add log warnings for unsupported kernels.

## Acceptance Criteria
- [ ] Can spawn a pane that is restricted to reading only specific directories.
- [ ] Attempting to write outside allowed paths results in `EACCES`.
- [ ] Non-Landlock systems (or disabled config) function normally.
