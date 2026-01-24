# FEAT-045: MCP Declarative Layout Tools

**Priority**: P1
**Component**: fugue-server (MCP)
**Type**: enhancement
**Estimated Effort**: medium
**Business Value**: high
**Technical Complexity**: medium
**Status**: new

## Overview

Add declarative layout control to MCP tools, allowing LLMs to create complex terminal layouts from natural language descriptions like "create a 65% 35% vertical split with the right side split into quadrants". This exposes the existing `layout.rs` capabilities (nested layouts, custom ratios, presets) through the MCP protocol.

## Problem Statement

The current `fugue_create_pane` tool only creates a single pane with basic direction (horizontal/vertical). While the underlying `layout.rs` already supports:

- Nested layouts via `LayoutNode` tree structure
- Custom ratios via `split_with_ratios()`
- Layout presets like `MainLeft`, `Grid2x2`, `MainTop`
- Per-pane resize operations

...none of this power is exposed via MCP. There's no way to:

1. Create multiple panes atomically in a specific layout
2. Specify custom size ratios (everything is 50/50)
3. Target specific panes for splitting
4. Resize existing panes

### Current MCP Capabilities

```rust
// fugue_create_pane - only creates one pane at a time
{
    "session": "optional",
    "window": "optional",
    "direction": "horizontal | vertical",  // Always 50/50 split
    "command": "optional",
    "cwd": "optional"
}
```

### Desired MCP Capabilities

```rust
// fugue_create_layout - create complex layout in one call
{
    "direction": "horizontal",
    "splits": [
        {"ratio": 0.65, "pane": {"command": "vim"}},
        {"ratio": 0.35, "direction": "vertical", "splits": [
            {"ratio": 0.5, "pane": {"command": "claude"}},
            {"ratio": 0.5, "pane": {"command": "bash"}}
        ]}
    ]
}
```

## Requirements

### Part 1: `fugue_create_layout` Tool

Create a new MCP tool that builds complex layouts declaratively in a single call.

#### Schema

```json
{
    "name": "fugue_create_layout",
    "description": "Create a complex pane layout declaratively. Supports nested splits with custom ratios.",
    "input_schema": {
        "type": "object",
        "properties": {
            "session": {
                "type": "string",
                "description": "Target session (UUID or name). Uses active session if omitted."
            },
            "window": {
                "type": "string",
                "description": "Target window (UUID or name). Creates new window if omitted."
            },
            "layout": {
                "type": "object",
                "description": "Layout specification (recursive structure)",
                "oneOf": [
                    {
                        "type": "object",
                        "properties": {
                            "pane": {
                                "type": "object",
                                "properties": {
                                    "command": {"type": "string"},
                                    "cwd": {"type": "string"},
                                    "name": {"type": "string"}
                                }
                            }
                        },
                        "required": ["pane"]
                    },
                    {
                        "type": "object",
                        "properties": {
                            "direction": {
                                "type": "string",
                                "enum": ["horizontal", "vertical"]
                            },
                            "splits": {
                                "type": "array",
                                "items": {
                                    "type": "object",
                                    "properties": {
                                        "ratio": {"type": "number", "minimum": 0.1, "maximum": 0.9},
                                        "layout": {"$ref": "#/properties/layout"}
                                    },
                                    "required": ["ratio"]
                                }
                            }
                        },
                        "required": ["direction", "splits"]
                    }
                ]
            },
            "preset": {
                "type": "string",
                "enum": ["single", "split_horizontal", "split_vertical", "grid_2x2", "main_left", "main_top"],
                "description": "Use a preset layout instead of custom layout spec. Pane commands can be specified via pane_commands array."
            },
            "pane_commands": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "command": {"type": "string"},
                        "cwd": {"type": "string"},
                        "name": {"type": "string"}
                    }
                },
                "description": "Commands for each pane when using presets (in reading order: left-to-right, top-to-bottom)"
            }
        }
    }
}
```

#### Example Usage

**Create a 65/35 horizontal split with the right side split vertically:**

```json
{
    "layout": {
        "direction": "horizontal",
        "splits": [
            {"ratio": 0.65, "layout": {"pane": {"command": "vim", "name": "editor"}}},
            {"ratio": 0.35, "layout": {
                "direction": "vertical",
                "splits": [
                    {"ratio": 0.5, "layout": {"pane": {"command": "claude", "name": "claude-main"}}},
                    {"ratio": 0.5, "layout": {"pane": {"command": "bash", "name": "terminal"}}}
                ]
            }}
        ]
    }
}
```

**Use a preset with custom commands:**

```json
{
    "preset": "main_left",
    "pane_commands": [
        {"command": "vim", "name": "editor"},
        {"command": "claude", "name": "claude-1"},
        {"command": "claude", "name": "claude-2"}
    ]
}
```

#### Response

```json
{
    "success": true,
    "session_id": "uuid",
    "session_name": "dev",
    "window_id": "uuid",
    "window_name": "workspace",
    "panes": [
        {"id": "uuid", "name": "editor", "position": "left"},
        {"id": "uuid", "name": "claude-main", "position": "top-right"},
        {"id": "uuid", "name": "terminal", "position": "bottom-right"}
    ],
    "layout_applied": "custom | preset_name"
}
```

### Part 2: `fugue_split_pane` Tool

Split a specific pane with custom ratio (not just 50/50).

#### Schema

```json
{
    "name": "fugue_split_pane",
    "description": "Split a specific pane with a custom size ratio",
    "input_schema": {
        "type": "object",
        "properties": {
            "pane_id": {
                "type": "string",
                "description": "UUID of the pane to split"
            },
            "direction": {
                "type": "string",
                "enum": ["horizontal", "vertical"],
                "description": "Split direction"
            },
            "ratio": {
                "type": "number",
                "minimum": 0.1,
                "maximum": 0.9,
                "description": "Size ratio for the original pane (0.0-1.0). New pane gets the remainder."
            },
            "command": {
                "type": "string",
                "description": "Command to run in the new pane"
            },
            "cwd": {
                "type": "string",
                "description": "Working directory for the new pane"
            },
            "name": {
                "type": "string",
                "description": "Optional name for the new pane"
            }
        },
        "required": ["pane_id", "direction"]
    }
}
```

#### Example Usage

**Split a pane with 70/30 ratio:**

```json
{
    "pane_id": "abc123-uuid",
    "direction": "vertical",
    "ratio": 0.7,
    "command": "claude",
    "name": "worker-1"
}
```

### Part 3: `fugue_resize_pane` Tool

Adjust pane sizes after creation.

#### Schema

```json
{
    "name": "fugue_resize_pane",
    "description": "Resize a pane by adjusting its ratio relative to its sibling",
    "input_schema": {
        "type": "object",
        "properties": {
            "pane_id": {
                "type": "string",
                "description": "UUID of the pane to resize"
            },
            "delta": {
                "type": "number",
                "minimum": -0.5,
                "maximum": 0.5,
                "description": "Amount to grow (+) or shrink (-) the pane. E.g., 0.1 grows by 10%."
            }
        },
        "required": ["pane_id", "delta"]
    }
}
```

#### Example Usage

**Make a pane 10% larger:**

```json
{
    "pane_id": "abc123-uuid",
    "delta": 0.1
}
```

### Part 4: Enhanced `fugue_create_pane`

Update the existing tool to support custom ratios.

#### Additional Parameter

```json
{
    "ratio": {
        "type": "number",
        "minimum": 0.1,
        "maximum": 0.9,
        "description": "Size ratio for the new pane (0.1-0.9). Default: 0.5"
    },
    "source_pane": {
        "type": "string",
        "description": "UUID of pane to split. Uses active pane if omitted."
    }
}
```

## Use Cases

### 1. Natural Language: "Create three sessions on the right half"

LLM translates to:
```json
{
    "layout": {
        "direction": "horizontal",
        "splits": [
            {"ratio": 0.5, "layout": {"pane": {}}},
            {"ratio": 0.5, "layout": {
                "direction": "vertical",
                "splits": [
                    {"ratio": 0.33, "layout": {"pane": {"command": "claude"}}},
                    {"ratio": 0.33, "layout": {"pane": {"command": "claude"}}},
                    {"ratio": 0.34, "layout": {"pane": {"command": "claude"}}}
                ]
            }}
        ]
    }
}
```

### 2. Natural Language: "Create a 65% 35% vertical split with the right side split into quadrants"

LLM translates to:
```json
{
    "layout": {
        "direction": "horizontal",
        "splits": [
            {"ratio": 0.65, "layout": {"pane": {}}},
            {"ratio": 0.35, "layout": {
                "direction": "vertical",
                "splits": [
                    {"ratio": 0.5, "layout": {
                        "direction": "horizontal",
                        "splits": [
                            {"ratio": 0.5, "layout": {"pane": {}}},
                            {"ratio": 0.5, "layout": {"pane": {}}}
                        ]
                    }},
                    {"ratio": 0.5, "layout": {
                        "direction": "horizontal",
                        "splits": [
                            {"ratio": 0.5, "layout": {"pane": {}}},
                            {"ratio": 0.5, "layout": {"pane": {}}}
                        ]
                    }}
                ]
            }}
        ]
    }
}
```

### 3. Natural Language: "Make the left pane larger"

LLM translates to:
```json
{
    "pane_id": "left-pane-uuid",
    "delta": 0.15
}
```

### 4. Natural Language: "Create a main editor with a small terminal at the bottom"

LLM translates to:
```json
{
    "layout": {
        "direction": "vertical",
        "splits": [
            {"ratio": 0.8, "layout": {"pane": {"command": "vim", "name": "editor"}}},
            {"ratio": 0.2, "layout": {"pane": {"command": "bash", "name": "terminal"}}}
        ]
    }
}
```

### 5. Using Presets for Common Layouts

```json
{
    "preset": "main_left",
    "pane_commands": [
        {"command": "vim"},
        {"command": "cargo watch -x test"},
        {"command": "cargo build --release"}
    ]
}
```

## Implementation Approach

### Leverage Existing Layout Infrastructure

The `layout.rs` file already has the necessary primitives:

1. **`LayoutNode`** - Tree structure for nested layouts
2. **`split_with_ratios()`** - Custom ratio splits
3. **`LayoutPreset`** - Common layout patterns
4. **`LayoutManager`** - Operations on layouts

### New Code Required

1. **MCP Schema Parsing** - Parse the declarative JSON into `LayoutNode` tree
2. **Pane Creation from Layout** - Walk the tree and spawn panes
3. **Layout Application** - Replace window's layout with the new structure
4. **Broadcast to TUI Clients** - Notify clients of layout changes (uses FEAT-039)

### Key Implementation Steps

1. **Add layout parsing in `mcp/handlers.rs`**:
   ```rust
   fn parse_layout_spec(spec: &serde_json::Value) -> Result<LayoutNode, Error> {
       // Recursive parsing of layout specification
   }
   ```

2. **Add pane spawning for layout**:
   ```rust
   async fn spawn_panes_for_layout(
       layout: &LayoutNode,
       session: &SessionId,
       window: &WindowId,
       commands: &[PaneCommand],
   ) -> Result<Vec<PaneInfo>, Error> {
       // Walk tree, spawn PTYs, collect pane IDs
   }
   ```

3. **Wire up new tools in `mcp/tools.rs`**

4. **Add handlers in `mcp/bridge.rs` or `mcp/handlers.rs`**

## Files Affected

| File | Changes |
|------|---------|
| `fugue-server/src/mcp/tools.rs` | Add tool definitions for create_layout, split_pane, resize_pane |
| `fugue-server/src/mcp/handlers.rs` | Implement handlers for new tools |
| `fugue-server/src/mcp/bridge.rs` | Route new tool calls |
| `fugue-server/src/session/manager.rs` | Add layout application methods |
| `fugue-client/src/ui/layout.rs` | May need additional methods for client-side layout sync |
| `fugue-protocol/src/messages.rs` | Add layout-related message types if needed |

## Implementation Tasks

### Section 1: Tool Definitions
- [ ] Add `fugue_create_layout` tool definition to `tools.rs`
- [ ] Add `fugue_split_pane` tool definition to `tools.rs`
- [ ] Add `fugue_resize_pane` tool definition to `tools.rs`
- [ ] Update `fugue_create_pane` with `ratio` and `source_pane` parameters

### Section 2: Layout Parsing
- [ ] Implement `parse_layout_spec()` to convert JSON to `LayoutNode`
- [ ] Handle nested layout parsing recursively
- [ ] Validate ratios sum to 1.0 (or normalize)
- [ ] Handle preset name to `LayoutPreset` mapping
- [ ] Add error handling for invalid layout specs

### Section 3: Pane Creation
- [ ] Implement `spawn_panes_for_layout()` to walk tree and create panes
- [ ] Spawn PTY for each leaf node
- [ ] Collect pane IDs and names
- [ ] Handle command/cwd/name for each pane
- [ ] Maintain pane order for response

### Section 4: Layout Application
- [ ] Add method to replace a window's layout tree
- [ ] Ensure existing panes are properly closed (if any)
- [ ] Wire new pane IDs into layout nodes
- [ ] Handle atomic creation (rollback on partial failure)

### Section 5: Resize Implementation
- [ ] Expose `resize_pane()` via MCP
- [ ] Validate delta bounds
- [ ] Handle resize at correct level of tree
- [ ] Broadcast layout change to clients

### Section 6: Client Sync
- [ ] Ensure TUI clients receive layout updates (FEAT-039 dependency)
- [ ] Test client layout reconstruction from server state
- [ ] Handle layout sync on client reconnect

### Section 7: Testing
- [ ] Unit tests for layout spec parsing
- [ ] Unit tests for nested layout creation
- [ ] Integration test: create complex layout via MCP
- [ ] Integration test: resize pane via MCP
- [ ] Integration test: preset layouts with custom commands
- [ ] Test ratio normalization edge cases
- [ ] Test error handling for invalid specs

### Section 8: Documentation
- [ ] Document layout spec JSON format
- [ ] Add examples for common layouts
- [ ] Update tool descriptions for clarity
- [ ] Update CHANGELOG

## Acceptance Criteria

- [ ] `fugue_create_layout` creates multi-pane layouts in one call
- [ ] Nested layouts (splits within splits) work correctly
- [ ] Custom ratios are respected (not just 50/50)
- [ ] Presets (`main_left`, `grid_2x2`, etc.) work with custom commands
- [ ] `fugue_split_pane` splits a specific pane with custom ratio
- [ ] `fugue_resize_pane` adjusts pane sizes dynamically
- [ ] TUI clients display correct layout after MCP operations
- [ ] All existing tests pass
- [ ] New tools have comprehensive test coverage

## Example Interaction

**User**: "Create a development workspace with vim on the left taking 60% of the space, and two Claude instances stacked on the right"

**LLM uses `fugue_create_layout`**:
```json
{
    "layout": {
        "direction": "horizontal",
        "splits": [
            {"ratio": 0.6, "layout": {"pane": {"command": "vim", "name": "editor"}}},
            {"ratio": 0.4, "layout": {
                "direction": "vertical",
                "splits": [
                    {"ratio": 0.5, "layout": {"pane": {"command": "claude", "name": "claude-1"}}},
                    {"ratio": 0.5, "layout": {"pane": {"command": "claude", "name": "claude-2"}}}
                ]
            }}
        ]
    }
}
```

**Result**:
```
+------------------+--------+
|                  |claude-1|
|      vim         +--------+
|     (60%)        |claude-2|
|                  | (40%)  |
+------------------+--------+
```

## Dependencies

- **FEAT-029**: MCP Natural Language Terminal Control (provides base MCP infrastructure)
- **FEAT-039**: MCP Pane Creation Broadcast (for syncing layouts to TUI clients)
- **FEAT-036**: Session-aware MCP Commands (for better session/window targeting)

## Notes

- The `layout.rs` infrastructure already exists in `fugue-client` - this feature primarily exposes it via MCP
- Ratio normalization should be forgiving (auto-normalize if ratios don't sum to 1.0)
- Consider adding a `fugue_get_layout` tool to query current layout structure
- Future enhancement: save/restore named layouts
- Future enhancement: animated resize transitions
