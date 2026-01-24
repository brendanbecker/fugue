# FEAT-086: Environment Variable Persistence Across Daemon Restarts

## Overview
Follow-up to BUG-031 (metadata persistence). Session environment variables set via `fugue_set_environment` are currently lost when the daemon restarts. This feature adds WAL-based persistence for environment variables, mirroring the approach used for metadata.

## Motivation
- **Consistency**: Metadata now persists (BUG-031 fix), but environment variables do not
- **Agent Workflows**: Agents set environment variables to configure pane behavior; these should survive restarts
- **Feature Parity**: `set_metadata` and `set_environment` should have equivalent durability guarantees

## Background
BUG-031 fixed metadata persistence by:
1. Adding `SessionMetadataSet` WAL entry variant
2. Logging metadata changes in `handle_set_metadata`
3. Replaying metadata during recovery

This feature applies the same pattern to environment variables.

## Requirements
- Environment variables set via MCP persist across daemon restarts
- Recovery replays environment changes in order
- Existing sessions without environment data migrate cleanly (empty HashMap)

## Tasks

### Section 1: Data Model Updates
- [x] Add `environment: HashMap<String, String>` field to `SessionSnapshot` in `fugue-persistence`
- [x] Add `SessionEnvironmentSet { session_id, key, value }` WAL entry variant
- [x] Ensure backward compatibility with existing checkpoint files (`#[serde(default)]`)

### Section 2: Persistence Implementation
- [x] Add WAL logging call in `handle_set_environment` (mcp_bridge.rs)
- [x] Update recovery logic to replay `SessionEnvironmentSet` entries
- [x] Update checkpoint save to include environment data

### Section 3: Testing
- [x] Add integration test: `test_persistence_environment_via_wal` (mod.rs)
- [x] Backward compatibility verified via `#[serde(default)]` attribute

## Files to Modify

| File | Changes |
|------|---------|
| `fugue-persistence/src/snapshot.rs` | Add `environment` field to `SessionSnapshot` |
| `fugue-persistence/src/wal.rs` | Add `SessionEnvironmentSet` entry variant |
| `fugue-persistence/src/recovery.rs` | Replay environment entries during recovery |
| `fugue-server/src/handlers/mcp_bridge.rs` | Log WAL entry in `handle_set_environment` |

## Acceptance Criteria
- [x] `fugue_set_environment` changes persist across daemon restarts
- [x] `fugue_get_session_info` returns correct environment after restart
- [x] Existing checkpoints without environment data load without error
- [x] WAL replay correctly reconstructs environment state

## Reference
- **BUG-031 fix**: Commit `2286aab` - pattern to follow for WAL logging
- **Persistence crate**: `fugue-persistence/`
- **MCP handlers**: `fugue-server/src/handlers/mcp_bridge.rs`
