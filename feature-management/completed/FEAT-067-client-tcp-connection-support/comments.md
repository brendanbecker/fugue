# Comments: FEAT-067

## 2026-01-13 - Feature Created

Created FEAT-067 for client TCP connection support. This is Phase 2 of remote peering, complementing FEAT-066 (daemon TCP listener).

**Key Design Decisions**:
- URL-style address format (`tcp://host:port`, `unix://path`)
- CLI flag `--addr` with `FUGUE_ADDR` environment variable
- Default to Unix socket for backward compatibility
- Transport abstraction using trait objects

**Dependencies**:
- Requires FEAT-066 for TCP daemon support
- Blocks FEAT-068 (SSH tunnel documentation)

**Next Steps**:
1. Wait for FEAT-066 completion (or coordinate parallel development)
2. Implement URL parsing and CLI flags
3. Add TCP connection logic
4. Test with FEAT-066 daemon

---
*Add implementation notes, decisions, and progress updates below*
