# FEAT-117: strip_escapes parameter for fugue_read_pane

**Priority**: P1
**Component**: fugue-server/mcp
**Type**: enhancement
**Estimated Effort**: small
**Business Value**: high

## Overview

Add a `strip_escapes: bool` parameter to the `fugue_read_pane` MCP tool that strips ANSI escape sequences from the output before returning.

## Problem Statement

When orchestrators read pane output via `fugue_read_pane`, 30-50% of the returned tokens are ANSI escape sequences (colors, cursor movement, formatting). These sequences:

1. **Waste context**: LLM tokens spent on `\x1b[32m`, `\x1b[0m`, etc. provide no semantic value
2. **Obscure content**: The actual text becomes harder to parse programmatically
3. **Inconsistent**: Different terminal apps emit different escape sequences

Example raw output (~500 tokens):
```
\x1b[0m\x1b[32m❯\x1b[0m \x1b[36mcargo\x1b[0m build
   \x1b[32mCompiling\x1b[0m fugue v0.1.0
```

Example stripped output (~100 tokens):
```
❯ cargo build
   Compiling fugue v0.1.0
```

**Token savings: 70-80%**

## API Design

### Updated Tool Schema

```json
{
  "name": "fugue_read_pane",
  "inputSchema": {
    "type": "object",
    "properties": {
      "pane_id": {
        "type": "string",
        "description": "UUID of the pane to read"
      },
      "lines": {
        "type": "integer",
        "description": "Number of lines to read (default: 100, max: 1000)"
      },
      "strip_escapes": {
        "type": "boolean",
        "default": false,
        "description": "Strip ANSI escape sequences from output"
      }
    },
    "required": ["pane_id"]
  }
}
```

### Response

When `strip_escapes: true`, return cleaned output:
```json
{
  "pane_id": "...",
  "content": "❯ cargo build\n   Compiling fugue v0.1.0\n",
  "lines_returned": 2
}
```

## Implementation

### Approach 1: strip-ansi-escapes crate (Recommended)

Use the `strip-ansi-escapes` crate which is well-maintained and handles all ANSI/VT100 sequences:

```rust
use strip_ansi_escapes::strip;

fn strip_escapes(input: &[u8]) -> String {
    let stripped = strip(input);
    String::from_utf8_lossy(&stripped).to_string()
}
```

Add to `Cargo.toml`:
```toml
strip-ansi-escapes = "0.2"
```

### Approach 2: Manual regex

If avoiding dependencies:
```rust
use regex::Regex;

lazy_static! {
    static ref ANSI_ESCAPE: Regex = Regex::new(
        r"\x1b\[[0-9;]*[a-zA-Z]|\x1b\].*?\x07|\x1b[()][AB012]"
    ).unwrap();
}

fn strip_escapes(input: &str) -> String {
    ANSI_ESCAPE.replace_all(input, "").to_string()
}
```

### Where to implement

The stripping should happen in the MCP handler, after reading from the pane buffer:

**File**: `fugue-server/src/mcp/bridge/handlers.rs`

In the `handle_read_pane` function:
1. Add `strip_escapes` parameter extraction
2. After getting content, conditionally strip escapes
3. Return cleaned content

## Implementation Tasks

### Section 1: Add Dependency

- [ ] Add `strip-ansi-escapes = "0.2"` to `fugue-server/Cargo.toml`
- [ ] Run `cargo build` to verify dependency resolves

### Section 2: Update Tool Schema

- [ ] Edit `fugue-server/src/mcp/tools.rs`
- [ ] Add `strip_escapes` property to `fugue_read_pane` schema
- [ ] Set default to `false` for backwards compatibility

### Section 3: Update Handler

- [ ] Edit `fugue-server/src/mcp/bridge/handlers.rs`
- [ ] Extract `strip_escapes` from request arguments
- [ ] After reading content, apply stripping if enabled
- [ ] Return stripped content

### Section 4: Testing

- [ ] Unit test: verify escape stripping works
- [ ] Integration test: read pane with `strip_escapes: true`
- [ ] Integration test: verify default behavior unchanged
- [ ] Manual test: run Claude in pane, read with stripping

## Files to Modify

| File | Changes |
|------|---------|
| `fugue-server/Cargo.toml` | Add `strip-ansi-escapes` dependency |
| `fugue-server/src/mcp/tools.rs` | Add `strip_escapes` to schema |
| `fugue-server/src/mcp/bridge/handlers.rs` | Implement stripping logic |

## Acceptance Criteria

- [ ] `strip_escapes` parameter available on `fugue_read_pane`
- [ ] Default is `false` (backwards compatible)
- [ ] When `true`, output contains no ANSI escape sequences
- [ ] Unicode characters preserved (emojis, box drawing, etc.)
- [ ] Performance: stripping adds <1ms latency
- [ ] All existing tests pass

## Notes

### Edge Cases

- Binary output: May contain bytes that look like escapes but aren't
- Partial sequences: Output may end mid-escape sequence
- OSC sequences: `\x1b]...\x07` title sequences should also be stripped
- Wide characters: CJK and emoji should be preserved

### Testing Commands

```bash
# Generate colorful output to test stripping
cargo build 2>&1 | head -10

# Typical Claude output has lots of escapes
claude --version
```
