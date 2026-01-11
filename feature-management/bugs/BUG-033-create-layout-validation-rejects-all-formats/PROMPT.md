# BUG-033: ccmux_create_layout Validation Rejects All Layout Formats

## Summary
`ccmux_create_layout` rejects all layout specifications with the error "Invalid layout specification: must contain 'pane' or 'splits'" even when the layout clearly contains `pane` or `splits` keys.

## Steps to Reproduce

1. Create a session: `ccmux_create_session` with name "dev-qa" - **succeeds**
2. Call `ccmux_create_layout` with any of these layouts:

**Simple pane:**
```json
{"pane": {}}
```
Result: `InvalidOperation: Invalid layout specification: must contain 'pane' or 'splits'`

**Two-pane split:**
```json
{"direction": "vertical", "splits": [{"ratio": 0.5, "layout": {"pane": {}}}, {"ratio": 0.5, "layout": {"pane": {}}}]}
```
Result: Same error

**Nested layout (from docs):**
```json
{"direction": "horizontal", "splits": [{"ratio": 0.7, "layout": {"direction": "vertical", "splits": [{"ratio": 0.6, "layout": {"pane": {"name": "editor"}}}, {"ratio": 0.4, "layout": {"pane": {"name": "sidebar"}}}]}}, {"ratio": 0.3, "layout": {"pane": {"name": "terminal"}}}]}
```
Result: Same error

## Expected Behavior
- Layout should be created according to the specification
- At minimum, `{"pane": {}}` should create a single pane

## Actual Behavior
- All layout formats are rejected with the same validation error
- Error message says to include 'pane' or 'splits' but they ARE present

## Environment
- ccmux version: current main branch (commit c8b0904)
- Platform: Linux (WSL2)
- Triggered during: QA demo run

## Relationship to BUG-028
- BUG-028 was about daemon crashes on create_layout
- That appears to be fixed (daemon no longer crashes)
- But now validation is too strict and rejects valid layouts

## Impact
- **Severity**: P1 - Declarative layouts completely non-functional
- **Affected Component**: daemon, create_layout validation
- **Workaround**: Use `ccmux_split_pane` repeatedly instead

## Investigation Findings

**Code Flow** (traced via source analysis):

1. MCP bridge receives tool call at `bridge.rs:760-764`
2. `arguments["layout"].clone()` extracts the layout parameter
3. `tool_create_layout()` at `bridge.rs:1503-1512` converts to `JsonValue` for bincode
4. Daemon handler at `handlers/mod.rs:246-254` calls `.into_inner()` to extract raw Value
5. Handler at `mcp_bridge.rs:1137-1216` validates layout

**Key Files**:
- `ccmux-server/src/mcp/bridge.rs` lines 760-764 (tool dispatch)
- `ccmux-server/src/mcp/bridge.rs` lines 1503-1512 (tool_create_layout)
- `ccmux-server/src/handlers/mod.rs` lines 246-254 (message dispatch)
- `ccmux-server/src/handlers/mcp_bridge.rs` lines 1137-1216 (validation)
- `ccmux-protocol/src/types.rs` lines 14-75 (JsonValue wrapper)

**Working Tests** (bypass bincode):
- `test_handle_create_layout_simple_pane` at mcp_bridge.rs:2401-2428 passes
- These tests call `handle_create_layout` directly with `serde_json::Value`

**Hypothesis**:
The issue may be in how the MCP client (Claude) sends the layout parameter. If sent as a JSON string instead of an object, `arguments["layout"]` would be a string, not an object, causing `layout.get("pane")` to return None.

## Debug Steps (ULTRATHINK)

1. Add logging at `bridge.rs:763` to see what `arguments["layout"]` actually contains
2. Check if it's `Value::String` instead of `Value::Object`
3. If string, add JSON parsing: `serde_json::from_str(&layout_str)`

## Notes
- The daemon remains stable (no crash)
- Other MCP operations continue to work
- Tests that bypass MCP bridge/bincode pass correctly
