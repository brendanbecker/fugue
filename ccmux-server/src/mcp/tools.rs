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
                        "description": "Optional session name to filter by"
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
                    "direction": {
                        "type": "string",
                        "enum": ["horizontal", "vertical"],
                        "description": "Split direction (default: vertical)"
                    },
                    "command": {
                        "type": "string",
                        "description": "Command to run in the new pane (default: shell)"
                    },
                    "cwd": {
                        "type": "string",
                        "description": "Working directory for the new pane"
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
                        "description": "Text to send to the pane (use \\n for Enter)"
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
                        "description": "Session name or ID (uses first session if omitted)"
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
                        "description": "Session name or ID (uses first session if omitted)"
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
    }
}
