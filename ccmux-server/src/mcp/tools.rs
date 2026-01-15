//! MCP tool definitions for ccmux
//!
//! Defines the tools exposed to Claude Code through the MCP protocol.

use super::protocol::Tool;

/// Get all tool definitions for the ccmux MCP server
pub fn get_tool_definitions() -> Vec<Tool> {
    vec![
        Tool {
            name: "ccmux_list_panes".into(),
            description: "List all panes in ccmux with their status, including Claude detection state".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "session": {
                        "type": "string",
                        "description": "Session name or ID to filter by. Uses active session if omitted."
                    }
                }
            }),
        },
        Tool {
            name: "ccmux_read_pane".into(),
            description: "Read the output buffer (scrollback) from a pane".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "pane_id": {
                        "type": "string",
                        "description": "UUID of the pane to read"
                    },
                    "lines": {
                        "type": "integer",
                        "description": "Number of lines to read (default: 100, max: 1000)"
                    }
                },
                "required": ["pane_id"]
            }),
        },
        Tool {
            name: "ccmux_create_pane".into(),
            description: "Create a new pane in a session".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "session": {
                        "type": "string",
                        "description": "Target session (UUID or name). Uses active session if omitted."
                    },
                    "window": {
                        "type": "string",
                        "description": "Target window (UUID or name). Uses first window in session if omitted."
                    },
                    "name": {
                        "type": "string",
                        "description": "Optional name for the pane"
                    },
                    "direction": {
                        "type": "string",
                        "enum": ["horizontal", "vertical"],
                        "description": "Split direction: 'vertical' creates side-by-side panes, 'horizontal' creates stacked panes (default: vertical)"
                    },
                    "command": {
                        "type": "string",
                        "description": "Command to run in the new pane (default: shell)"
                    },
                    "cwd": {
                        "type": "string",
                        "description": "Working directory for the new pane"
                    },
                    "select": {
                        "type": "boolean",
                        "default": false,
                        "description": "If true, focus the new pane after creation (default: false, keeps current focus)"
                    },
                    "model": {
                        "type": "string",
                        "description": "Claude model to use (e.g. claude-3-5-sonnet-20241022). Overrides default/preset."
                    },
                    "config": {
                        "type": "object",
                        "description": "Full Claude configuration object (merged with presets)"
                    },
                    "preset": {
                        "type": "string",
                        "description": "Configuration preset name (e.g. 'haiku-worker')"
                    }
                }
            }),
        },
        Tool {
            name: "ccmux_send_input".into(),
            description: "Send input (keystrokes) to a pane".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "pane_id": {
                        "type": "string",
                        "description": "UUID of the target pane"
                    },
                    "input": {
                        "type": "string",
                        "description": "Text to send to the pane"
                    },
                    "submit": {
                        "type": "boolean",
                        "default": false,
                        "description": "If true, press Enter after sending input (sends carriage return)"
                    }
                },
                "required": ["pane_id", "input"]
            }),
        },
        Tool {
            name: "ccmux_get_status".into(),
            description: "Get detailed status of a pane including Claude state if applicable".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "pane_id": {
                        "type": "string",
                        "description": "UUID of the pane"
                    }
                },
                "required": ["pane_id"]
            }),
        },
        Tool {
            name: "ccmux_close_pane".into(),
            description: "Close a pane by killing its process".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "pane_id": {
                        "type": "string",
                        "description": "UUID of the pane to close"
                    }
                },
                "required": ["pane_id"]
            }),
        },
        Tool {
            name: "ccmux_focus_pane".into(),
            description: "Focus a pane (make it the active pane)".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "pane_id": {
                        "type": "string",
                        "description": "UUID of the pane to focus"
                    }
                },
                "required": ["pane_id"]
            }),
        },
        Tool {
            name: "ccmux_select_window".into(),
            description: "Switch to a specific window (make it the active window)".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "window_id": {
                        "type": "string",
                        "description": "UUID of the window to select"
                    }
                },
                "required": ["window_id"]
            }),
        },
        Tool {
            name: "ccmux_select_session".into(),
            description: "Switch to a specific session (make it the active session)".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "session_id": {
                        "type": "string",
                        "description": "UUID of the session to select"
                    }
                },
                "required": ["session_id"]
            }),
        },
        Tool {
            name: "ccmux_list_sessions".into(),
            description: "List all terminal sessions".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {}
            }),
        },
        Tool {
            name: "ccmux_list_windows".into(),
            description: "List all windows in a session".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "session": {
                        "type": "string",
                        "description": "Session name or ID. Uses active session if omitted."
                    }
                }
            }),
        },
        Tool {
            name: "ccmux_create_session".into(),
            description: "Create a new terminal session".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "name": {
                        "type": "string",
                        "description": "Optional name for the session (auto-generated if omitted)"
                    },
                    "command": {
                        "type": "string",
                        "description": "Command to run in the default pane (default: user's shell)"
                    },
                    "cwd": {
                        "type": "string",
                        "description": "Working directory for the session"
                    }
                }
            }),
        },
        Tool {
            name: "ccmux_create_window".into(),
            description: "Create a new window in a session".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "session": {
                        "type": "string",
                        "description": "Session name or ID. Uses active session if omitted."
                    },
                    "name": {
                        "type": "string",
                        "description": "Optional name for the new window"
                    },
                    "command": {
                        "type": "string",
                        "description": "Command to run in the default pane (default: shell)"
                    }
                }
            }),
        },
        Tool {
            name: "ccmux_rename_session".into(),
            description: "Rename a session for easier identification".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "session": {
                        "type": "string",
                        "description": "Session to rename (UUID or current name)"
                    },
                    "name": {
                        "type": "string",
                        "description": "New display name for the session"
                    }
                },
                "required": ["session", "name"]
            }),
        },
        // FEAT-036: Pane and window rename tools
        Tool {
            name: "ccmux_rename_pane".into(),
            description: "Rename a pane for easier identification".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "pane_id": {
                        "type": "string",
                        "description": "UUID of the pane to rename"
                    },
                    "name": {
                        "type": "string",
                        "description": "New display name for the pane"
                    }
                },
                "required": ["pane_id", "name"]
            }),
        },
        Tool {
            name: "ccmux_rename_window".into(),
            description: "Rename a window for easier identification".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "window_id": {
                        "type": "string",
                        "description": "UUID of the window to rename"
                    },
                    "name": {
                        "type": "string",
                        "description": "New display name for the window"
                    }
                },
                "required": ["window_id", "name"]
            }),
        },
        Tool {
            name: "ccmux_split_pane".into(),
            description: "Split a specific pane with custom ratio".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "pane_id": {
                        "type": "string",
                        "description": "UUID of the pane to split"
                    },
                    "direction": {
                        "type": "string",
                        "enum": ["horizontal", "vertical"],
                        "description": "Split direction: 'vertical' creates side-by-side panes, 'horizontal' creates stacked panes (default: vertical)"
                    },
                    "ratio": {
                        "type": "number",
                        "description": "Size ratio for the original pane (0.1 to 0.9, default: 0.5). The new pane gets the remaining space."
                    },
                    "command": {
                        "type": "string",
                        "description": "Command to run in the new pane (default: shell)"
                    },
                    "cwd": {
                        "type": "string",
                        "description": "Working directory for the new pane"
                    },
                    "select": {
                        "type": "boolean",
                        "default": false,
                        "description": "If true, focus the new pane after creation (default: false)"
                    }
                },
                "required": ["pane_id"]
            }),
        },
        Tool {
            name: "ccmux_resize_pane".into(),
            description: "Adjust pane sizes dynamically".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "pane_id": {
                        "type": "string",
                        "description": "UUID of the pane to resize"
                    },
                    "delta": {
                        "type": "number",
                        "description": "Size change as a fraction (-0.5 to 0.5). Positive values grow the pane, negative values shrink it."
                    }
                },
                "required": ["pane_id", "delta"]
            }),
        },
        Tool {
            name: "ccmux_kill_session".into(),
            description: "Kill/destroy a session and all its windows and panes".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "session": {
                        "type": "string",
                        "description": "Session to kill (UUID or name)"
                    }
                },
                "required": ["session"]
            }),
        },
        Tool {
            name: "ccmux_set_environment".into(),
            description: "Set an environment variable on a session (inherited by new panes)".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "session": {
                        "type": "string",
                        "description": "Session UUID or name"
                    },
                    "key": {
                        "type": "string",
                        "description": "Environment variable name"
                    },
                    "value": {
                        "type": "string",
                        "description": "Environment variable value"
                    }
                },
                "required": ["session", "key", "value"]
            }),
        },
        Tool {
            name: "ccmux_get_environment".into(),
            description: "Get environment variables from a session".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "session": {
                        "type": "string",
                        "description": "Session UUID or name"
                    },
                    "key": {
                        "type": "string",
                        "description": "Specific environment variable to get (omit to get all)"
                    }
                },
                "required": ["session"]
            }),
        },
        Tool {
            name: "ccmux_set_metadata".into(),
            description: "Set metadata on a session (arbitrary key-value pairs for application use)".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "session": {
                        "type": "string",
                        "description": "Session UUID or name"
                    },
                    "key": {
                        "type": "string",
                        "description": "Metadata key"
                    },
                    "value": {
                        "type": "string",
                        "description": "Metadata value"
                    }
                },
                "required": ["session", "key", "value"]
            }),
        },
        Tool {
            name: "ccmux_get_metadata".into(),
            description: "Get metadata from a session".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "session": {
                        "type": "string",
                        "description": "Session UUID or name"
                    },
                    "key": {
                        "type": "string",
                        "description": "Specific metadata key to get (omit to get all)"
                    }
                },
                "required": ["session"]
            }),
        },
        Tool {
            name: "ccmux_create_layout".into(),
            description: "Create complex layouts declaratively in a single call. Supports nested splits with custom ratios.".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "session": {
                        "type": "string",
                        "description": "Target session (UUID or name). Uses active session if omitted."
                    },
                    "window": {
                        "type": "string",
                        "description": "Target window (UUID or name). Uses first window in session if omitted."
                    },
                    "layout": {
                        "type": "object",
                        "description": "Layout specification. Can be a pane or a split.",
                        "oneOf": [
                            {
                                "type": "object",
                                "properties": {
                                    "pane": {
                                        "type": "object",
                                        "properties": {
                                            "command": { "type": "string", "description": "Command to run (default: shell)" },
                                            "cwd": { "type": "string", "description": "Working directory" },
                                            "name": { "type": "string", "description": "Optional name for the pane" }
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
                                        "enum": ["horizontal", "vertical"],
                                        "description": "Split direction"
                                    },
                                    "splits": {
                                        "type": "array",
                                        "items": {
                                            "type": "object",
                                            "properties": {
                                                "ratio": { "type": "number", "description": "Size ratio (0.0-1.0)" },
                                                "layout": { "type": "object", "description": "Nested layout (pane or split)" }
                                            },
                                            "required": ["ratio", "layout"]
                                        },
                                        "description": "Child layouts with their ratios (should sum to 1.0)"
                                    }
                                },
                                "required": ["direction", "splits"]
                            }
                        ]
                    }
                },
                "required": ["layout"]
            }),
        },
        // ==================== FEAT-048: Orchestration MCP Tools ====================
        Tool {
            name: "ccmux_send_orchestration".into(),
            description: "Send orchestration message to other sessions using tag-based routing".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "target": {
                        "type": "object",
                        "description": "Target for the message. Use ONE of: {\"tag\": \"..\"}, {\"session\": \"uuid\"}, {\"broadcast\": true}, {\"worktree\": \"path\"}",
                        "oneOf": [
                            {
                                "type": "object",
                                "properties": {
                                    "tag": {
                                        "type": "string",
                                        "description": "Send to sessions with this tag (e.g., 'orchestrator', 'worker')"
                                    }
                                },
                                "required": ["tag"]
                            },
                            {
                                "type": "object",
                                "properties": {
                                    "session": {
                                        "type": "string",
                                        "format": "uuid",
                                        "description": "Send to specific session by UUID"
                                    }
                                },
                                "required": ["session"]
                            },
                            {
                                "type": "object",
                                "properties": {
                                    "broadcast": {
                                        "type": "boolean",
                                        "const": true,
                                        "description": "Broadcast to all sessions"
                                    }
                                },
                                "required": ["broadcast"]
                            },
                            {
                                "type": "object",
                                "properties": {
                                    "worktree": {
                                        "type": "string",
                                        "description": "Send to sessions in specific worktree path"
                                    }
                                },
                                "required": ["worktree"]
                            }
                        ]
                    },
                    "msg_type": {
                        "type": "string",
                        "description": "Message type identifier (e.g., 'status.update', 'task.assigned', 'help.request')"
                    },
                    "payload": {
                        "type": "object",
                        "description": "Message payload - structure defined by the workflow/message type"
                    }
                },
                "required": ["target", "msg_type", "payload"]
            }),
        },
        Tool {
            name: "ccmux_set_tags".into(),
            description: "Add or remove tags on a session for routing purposes".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "session": {
                        "type": "string",
                        "description": "Session UUID or name. Uses first session if omitted."
                    },
                    "add": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "Tags to add (e.g., ['orchestrator', 'primary'])"
                    },
                    "remove": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "Tags to remove"
                    }
                }
            }),
        },
        Tool {
            name: "ccmux_get_tags".into(),
            description: "Get tags from a session".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "session": {
                        "type": "string",
                        "description": "Session UUID or name. Uses first session if omitted."
                    }
                }
            }),
        },
        Tool {
            name: "ccmux_report_status".into(),
            description: "Report current session status to orchestrator (sends to sessions tagged 'orchestrator')".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "status": {
                        "type": "string",
                        "enum": ["idle", "working", "waiting_for_input", "blocked", "complete", "error"],
                        "description": "Current status of the session"
                    },
                    "message": {
                        "type": "string",
                        "description": "Optional status message with details"
                    }
                },
                "required": ["status"]
            }),
        },
        Tool {
            name: "ccmux_request_help".into(),
            description: "Request help from orchestrator (sends to sessions tagged 'orchestrator')".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "context": {
                        "type": "string",
                        "description": "Description of what help is needed"
                    }
                },
                "required": ["context"]
            }),
        },
        Tool {
            name: "ccmux_broadcast".into(),
            description: "Broadcast a message to all other sessions".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "msg_type": {
                        "type": "string",
                        "description": "Message type identifier"
                    },
                    "payload": {
                        "type": "object",
                        "description": "Message payload"
                    }
                },
                "required": ["msg_type", "payload"]
            }),
        },
        // ==================== FEAT-059: Beads Workflow Integration Tools ====================
        Tool {
            name: "ccmux_beads_assign".into(),
            description: "Assign a beads issue to the current pane. Tracks which pane is working on which issue.".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "issue_id": {
                        "type": "string",
                        "description": "The beads issue ID to assign (e.g., 'bd-456', 'BUG-042')"
                    },
                    "pane_id": {
                        "type": "string",
                        "description": "UUID of the pane to assign the issue to. Uses first pane if omitted."
                    }
                },
                "required": ["issue_id"]
            }),
        },
        Tool {
            name: "ccmux_beads_release".into(),
            description: "Release/unassign the current beads issue from a pane. Records outcome in history.".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "pane_id": {
                        "type": "string",
                        "description": "UUID of the pane to release the issue from. Uses first pane if omitted."
                    },
                    "outcome": {
                        "type": "string",
                        "enum": ["completed", "abandoned", "transferred"],
                        "description": "Outcome of the issue work (default: completed)"
                    }
                }
            }),
        },
        Tool {
            name: "ccmux_beads_find_pane".into(),
            description: "Find the pane currently working on a specific beads issue.".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "issue_id": {
                        "type": "string",
                        "description": "The beads issue ID to search for"
                    }
                },
                "required": ["issue_id"]
            }),
        },
        Tool {
            name: "ccmux_beads_pane_history".into(),
            description: "Get the issue history for a pane, showing all issues worked on.".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "pane_id": {
                        "type": "string",
                        "description": "UUID of the pane to get history for. Uses first pane if omitted."
                    }
                }
            }),
        },
        // ==================== FEAT-060: Connection Status Tool ====================
        Tool {
            name: "ccmux_connection_status".into(),
            description: "Get the current daemon connection status including health and recovery info".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {}
            }),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_definitions_not_empty() {
        let tools = get_tool_definitions();
        assert!(!tools.is_empty());
    }

    #[test]
    fn test_all_tools_have_names() {
        let tools = get_tool_definitions();
        for tool in &tools {
            assert!(!tool.name.is_empty());
            assert!(tool.name.starts_with("ccmux_"));
        }
    }

    #[test]
    fn test_all_tools_have_descriptions() {
        let tools = get_tool_definitions();
        for tool in &tools {
            assert!(!tool.description.is_empty());
        }
    }

    #[test]
    fn test_all_tools_have_valid_schemas() {
        let tools = get_tool_definitions();
        for tool in &tools {
            // All schemas should be objects
            assert!(tool.input_schema.is_object());
            assert_eq!(tool.input_schema["type"], "object");
        }
    }

    #[test]
    fn test_expected_tools_present() {
        let tools = get_tool_definitions();
        let names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();

        assert!(names.contains(&"ccmux_list_panes"));
        assert!(names.contains(&"ccmux_read_pane"));
        assert!(names.contains(&"ccmux_create_pane"));
        assert!(names.contains(&"ccmux_send_input"));
        assert!(names.contains(&"ccmux_get_status"));
        assert!(names.contains(&"ccmux_close_pane"));
        assert!(names.contains(&"ccmux_focus_pane"));
        assert!(names.contains(&"ccmux_list_sessions"));
        assert!(names.contains(&"ccmux_list_windows"));
        assert!(names.contains(&"ccmux_create_session"));
        assert!(names.contains(&"ccmux_create_window"));
        assert!(names.contains(&"ccmux_rename_session"));
        // FEAT-036: Pane and window rename tools
        assert!(names.contains(&"ccmux_rename_pane"));
        assert!(names.contains(&"ccmux_rename_window"));
        // New declarative layout tools (FEAT-045)
        assert!(names.contains(&"ccmux_split_pane"));
        assert!(names.contains(&"ccmux_resize_pane"));
        assert!(names.contains(&"ccmux_create_layout"));
        assert!(names.contains(&"ccmux_kill_session"));
        assert!(names.contains(&"ccmux_set_environment"));
        assert!(names.contains(&"ccmux_get_environment"));
        // FEAT-050: Session metadata
        assert!(names.contains(&"ccmux_set_metadata"));
        assert!(names.contains(&"ccmux_get_metadata"));
        // FEAT-048: Orchestration MCP tools
        assert!(names.contains(&"ccmux_send_orchestration"));
        assert!(names.contains(&"ccmux_set_tags"));
        assert!(names.contains(&"ccmux_get_tags"));
        assert!(names.contains(&"ccmux_report_status"));
        assert!(names.contains(&"ccmux_request_help"));
        assert!(names.contains(&"ccmux_broadcast"));
        // FEAT-059: Beads workflow integration tools
        assert!(names.contains(&"ccmux_beads_assign"));
        assert!(names.contains(&"ccmux_beads_release"));
        assert!(names.contains(&"ccmux_beads_find_pane"));
        assert!(names.contains(&"ccmux_beads_pane_history"));
        // FEAT-060: Connection status tool
        assert!(names.contains(&"ccmux_connection_status"));
    }
}
