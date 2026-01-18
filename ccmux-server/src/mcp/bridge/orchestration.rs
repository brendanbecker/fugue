use std::time::Duration;
use tokio::time::Instant;
use uuid::Uuid;
use regex::Regex;
use serde_json::json;
use ccmux_protocol::{ClientMessage, ServerMessage};
use crate::mcp::error::McpError;
use crate::mcp::protocol::ToolResult;
use super::connection::ConnectionManager;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExpectAction {
    Notify,
    ClosePane,
    ReturnOutput,
}

impl std::str::FromStr for ExpectAction {
    type Err = McpError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "notify" => Ok(ExpectAction::Notify),
            "close_pane" => Ok(ExpectAction::ClosePane),
            "return_output" => Ok(ExpectAction::ReturnOutput),
            _ => Err(McpError::InvalidParams(format!("Invalid action: {}", s))),
        }
    }
}

/// Wait for a regex pattern to appear in a pane's output
pub async fn run_expect(
    connection: &mut ConnectionManager,
    pane_id: Uuid,
    pattern: &str,
    timeout_ms: u64,
    action: ExpectAction,
    poll_interval_ms: u64,
    lines: usize,
) -> Result<ToolResult, McpError> {
    let regex = Regex::new(pattern)
        .map_err(|e| McpError::InvalidParams(format!("Invalid regex pattern: {}", e)))?;
    
    let start_time = Instant::now();
    let timeout = Duration::from_millis(timeout_ms);
    let poll_interval = Duration::from_millis(poll_interval_ms);

    loop {
        // 1. Check for timeout
        let elapsed = start_time.elapsed();
        if elapsed > timeout {
            return Ok(ToolResult::text(json!({
                "status": "timeout",
                "pattern": pattern,
                "duration_ms": elapsed.as_millis(),
            }).to_string()));
        }

        // 2. Read pane content
        connection.send_to_daemon(ClientMessage::ReadPane { pane_id, lines }).await?;
        
        let content = match connection.recv_response_from_daemon().await? {
            ServerMessage::PaneContent { content, .. } => content,
            ServerMessage::Error { code, message, .. } => {
                // If pane not found or other error, return immediately
                return Ok(ToolResult::error(format!("{:?}: {}", code, message)));
            }
            msg => return Err(McpError::UnexpectedResponse(format!("{:?}", msg))),
        };

        // 3. Check for match
        if let Some(mat) = regex.find(&content) {
            let match_text = mat.as_str().to_string();
            // Extract the line containing the match for context
            // We'll just take the line where the match starts
            let start_index = mat.start();
            let line_start = content[..start_index].rfind('\n').map(|i| i + 1).unwrap_or(0);
            let line_end = content[start_index..].find('\n').map(|i| start_index + i).unwrap_or(content.len());
            let line_content = content[line_start..line_end].to_string();

            let mut response = json!({
                "status": "matched",
                "pattern": pattern,
                "match": match_text,
                "line": line_content,
                "duration_ms": elapsed.as_millis(),
            });

            // 4. Perform action
            match action {
                ExpectAction::Notify => {
                    // Just return success
                },
                ExpectAction::ClosePane => {
                    connection.send_to_daemon(ClientMessage::ClosePane { pane_id }).await?;
                    // Wait for confirmation or ignore? Best to confirm it closed.
                    // But we don't want to block indefinitely if it fails.
                    // We'll try to read the response.
                    match connection.recv_response_from_daemon().await? {
                        ServerMessage::PaneClosed { .. } => {}, 
                        ServerMessage::Error { code, message, .. } => {
                             return Ok(ToolResult::error(format!("Pattern found but failed to close pane: {:?}: {}", code, message)));
                        }
                         msg => return Err(McpError::UnexpectedResponse(format!("{:?}", msg))),
                    }
                    response["pane_closed"] = json!(true);
                },
                ExpectAction::ReturnOutput => {
                    response["output"] = json!(content);
                }
            }

            return Ok(ToolResult::text(response.to_string()));
        }

                    // 5. Wait before next poll
                tokio::time::sleep(poll_interval).await;
            }
        }
        
        #[cfg(test)]
        mod tests {
            use super::*;
            use std::str::FromStr;
        
            #[test]
            fn test_expect_action_parsing() {
                assert_eq!(ExpectAction::from_str("notify").unwrap(), ExpectAction::Notify);
                assert_eq!(ExpectAction::from_str("close_pane").unwrap(), ExpectAction::ClosePane);
                assert_eq!(ExpectAction::from_str("return_output").unwrap(), ExpectAction::ReturnOutput);
                assert!(ExpectAction::from_str("invalid").is_err());
            }
        
            #[test]
            fn test_regex_compilation() {
                let regex = Regex::new(r"___CCMUX_EXIT_\d+___");
                assert!(regex.is_ok());
                
                let regex = Regex::new(r"[invalid");
                assert!(regex.is_err());
            }
        
            #[test]
            fn test_regex_matching() {
                let regex = Regex::new(r"___CCMUX_EXIT_0___").unwrap();
                let content = "some output\n___CCMUX_EXIT_0___\nmore output";
                assert!(regex.find(content).is_some());
            }
        }
