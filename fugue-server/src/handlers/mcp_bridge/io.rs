use tracing::debug;
use uuid::Uuid;
use fugue_protocol::{ErrorCode, ServerMessage};
use crate::handlers::{HandlerContext, HandlerResult};

impl HandlerContext {
    /// Handle ReadPane - read scrollback from a pane
    pub async fn handle_read_pane(
        &self,
        pane_id: Uuid,
        lines: usize,
    ) -> HandlerResult {
        debug!("ReadPane {} request from {} (lines: {})", pane_id, self.client_id, lines);

        let session_manager = self.session_manager.read().await;

        match session_manager.find_pane(pane_id) {
            Some((_, _, pane)) => {
                // Limit to reasonable number of lines
                let lines = lines.min(1000);

                // Get lines from scrollback
                let scrollback = pane.scrollback();
                let all_lines: Vec<&str> = scrollback.get_lines().collect();
                let start = all_lines.len().saturating_sub(lines);
                let content = all_lines[start..].join("\n");

                debug!("Read {} lines from pane {}", all_lines.len().min(lines), pane_id);
                HandlerResult::Response(ServerMessage::PaneContent {
                    pane_id,
                    content,
                })
            }
            None => {
                debug!("Pane {} not found for ReadPane", pane_id);
                HandlerContext::error(
                    ErrorCode::PaneNotFound,
                    format!("Pane {} not found", pane_id),
                )
            }
        }
    }
}

