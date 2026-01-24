# ADR-002: Claude Communication Protocol

## Status

**Accepted** - 2026-01-07

## Context

fugue needs a way for Claude Code to communicate with the multiplexer for:

1. Spawning new panes (sub-agents, test runners)
2. Reading output from sibling panes
3. Controlling pane focus and layout
4. Querying pane status

Two approaches were identified in research:

### Option A: MCP Server

Model Context Protocol (MCP) is Anthropic's standard for tool integration. fugue would expose tools that Claude can call:

```json
{
  "tool": "fugue_create_pane",
  "input": {
    "direction": "horizontal",
    "command": "npm test"
  }
}
```

**Pros:**
- Structured, typed interface
- Official Anthropic protocol
- Deterministic behavior
- Easy to test and validate

**Cons:**
- Requires MCP configuration in Claude's settings
- Additional process/socket overhead
- Not always available (depends on Claude configuration)

### Option B: XML Sideband

Parse structured commands from Claude's terminal output:

```xml
<fugue:spawn direction="vertical" command="cargo build" />
```

**Pros:**
- Works without any configuration
- No additional infrastructure
- Natural in output stream

**Cons:**
- Parsing complexity
- Potential false positives
- Less structured than MCP
- 95-98% compliance rate (may fail occasionally)

## Decision

**Implement both protocols**. Use MCP for formal orchestration scenarios and XML sideband for lightweight, opportunistic integration.

## Rationale

### Why Both?

Different scenarios benefit from different protocols:

| Scenario | Best Protocol | Reason |
|----------|---------------|--------|
| Automated pipelines | MCP | Reliability, structure |
| Interactive sessions | Sideband | Zero config, works anywhere |
| CI/CD integration | MCP | Machine-to-machine |
| SKILL.md workflows | Sideband | User-defined, flexible |
| Production systems | MCP | Deterministic |

### Protocol Priority

When both are available:
1. MCP commands take precedence (more reliable)
2. Sideband parsed only if MCP not configured
3. User can disable sideband parsing via config

### MCP Implementation

```rust
// Expose fugue as MCP server
impl McpServer for CcmuxMcp {
    fn list_tools(&self) -> Vec<Tool> {
        vec![
            Tool::new("fugue_list_panes", "List all panes with metadata"),
            Tool::new("fugue_create_pane", "Create a new pane"),
            Tool::new("fugue_read_pane", "Read pane output buffer"),
            Tool::new("fugue_send_input", "Send input to pane"),
            Tool::new("fugue_get_status", "Get pane state"),
            Tool::new("fugue_focus_pane", "Switch focus to pane"),
        ]
    }

    async fn call_tool(&self, name: &str, input: Value) -> Result<Value> {
        match name {
            "fugue_create_pane" => self.create_pane(input).await,
            "fugue_read_pane" => self.read_pane(input).await,
            // ...
        }
    }
}
```

### Sideband Implementation

```rust
// Parse and strip fugue commands from output
impl SidebandParser {
    pub fn process(&mut self, output: &str) -> (String, Vec<Command>) {
        let mut display = String::new();
        let mut commands = Vec::new();

        // Regex: <fugue:cmd attr="val">content</fugue:cmd>
        //    or: <fugue:cmd attr="val" />
        let re = regex::Regex::new(
            r"<fugue:(\w+)([^>]*)(?:>(.*?)</fugue:\1>|/>)"
        ).unwrap();

        let mut last = 0;
        for cap in re.captures_iter(output) {
            let m = cap.get(0).unwrap();

            // Keep text before command for display
            display.push_str(&output[last..m.start()]);
            last = m.end();

            // Parse command
            if let Some(cmd) = self.parse_command(&cap) {
                commands.push(cmd);
            }
        }

        display.push_str(&output[last..]);
        (display, commands)
    }
}
```

### Command Set

Both protocols support the same commands:

| Command | MCP Tool | Sideband Tag | Description |
|---------|----------|--------------|-------------|
| List panes | `fugue_list_panes` | `<fugue:list />` | Get pane info |
| Create pane | `fugue_create_pane` | `<fugue:spawn>` | New pane |
| Read output | `fugue_read_pane` | `<fugue:read>` | Get buffer |
| Send input | `fugue_send_input` | `<fugue:input>` | Type in pane |
| Focus | `fugue_focus_pane` | `<fugue:focus>` | Switch pane |
| Control | `fugue_control` | `<fugue:control>` | Resize, close |

## Consequences

### Positive

- Flexible: works in any environment
- Reliable: MCP for critical paths
- Future-proof: MCP is Anthropic's direction
- Backward compatible: sideband needs no setup

### Negative

- Two codepaths to maintain
- Potential confusion about which to use
- MCP requires server management
- Sideband has edge cases

### Configuration

```toml
# ~/.fugue/config/fugue.toml

[claude.communication]
# Enable MCP server
mcp_enabled = true
mcp_socket = "~/.fugue/mcp.sock"

# Enable sideband parsing
sideband_enabled = true

# Priority when both available
prefer = "mcp"  # or "sideband"
```

## Migration Path

1. **Phase 1**: Implement sideband (simpler, no deps)
2. **Phase 2**: Add MCP via `rmcp` crate
3. **Phase 3**: Make sideband opt-in, MCP default
4. **Future**: Deprecate sideband if MCP becomes universal

## Security Considerations

### MCP

- Socket permissions restrict access
- Tool calls authenticated via MCP handshake
- Structured input validation

### Sideband

- Only parse from Claude-type panes
- Validate command parameters
- Rate limit command execution
- Sanitize any user-controlled content

## References

- [Model Context Protocol](https://modelcontextprotocol.io/)
- [rmcp crate](https://crates.io/crates/rmcp) - MCP Rust SDK
- Research: `docs/research/SYNTHESIS.md` Section 3.4
- Research: `docs/research/parsed/gemini_abstracts.md` (MCP emphasis)
- Research: `docs/research/parsed/claude_abstracts.md` (Sideband design)
