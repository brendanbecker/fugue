# Tasks: FEAT-077 - Human-control mode UX indicator and MCP error details

## Section 1: Protocol Updates
- [x] Define `ErrorDetails` enum in `fugue-protocol`.
- [x] Update `ServerMessage::Error` to include `details: Option<ErrorDetails>`.
- [x] Update `ErrorCode` if needed (or rely on `ErrorDetails`).

## Section 2: Server Implementation
- [x] Update `Arbitrator` to return detailed timing info (already done in FEAT-079, just need to propagate it).
- [x] Update handlers to populate `ErrorDetails` when returning `UserPriorityActive` error.

## Section 3: Client Implementation
- [x] Update `fugue-client` to parse `ErrorDetails`.
- [x] Implement `HumanControlIndicator` widget in TUI.
- [x] Render indicator when `ErrorDetails::HumanControl` is received or client-side prediction logic triggers.

## Section 4: MCP Bridge Implementation
- [x] Update `mcp-bridge` to forward error details in JSON-RPC error data (via informative error messages).