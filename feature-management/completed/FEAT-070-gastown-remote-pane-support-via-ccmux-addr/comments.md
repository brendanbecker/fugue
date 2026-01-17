# Comments: FEAT-070

## 2026-01-13: Feature created

Created by user request. This feature enables gastown multi-agent system to leverage ccmux remote peering support (FEAT-066, FEAT-067, FEAT-068) for hybrid orchestration workflows.

**Key Design Points**:
- Uses `GASTOWN_CCMUX_ADDR` environment variable for remote addressing
- Supports `tcp://host:port` and `unix://path` URL schemes
- Maintains backward compatibility (local execution as default)
- Delegates state sync to external tooling (git, rsync)
- Enables Mayor local + polecats remote workflow

**Source Documentation**: Based on `docs/gastown-ccmux-remote-support.md` design specification.

**Implementation Priority**: P2 (medium priority) - Blocks remote gastown workflows but ccmux remote peering features must be completed first.
