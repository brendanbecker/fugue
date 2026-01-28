use tracing::{info, debug};
use uuid::Uuid;
use fugue_protocol::{ErrorCode, ServerMessage};
use crate::handlers::{HandlerContext, HandlerResult};

impl HandlerContext {
    /// Handle SetMetadata - set metadata on a session
    pub async fn handle_set_metadata(
        &self,
        session_filter: String,
        key: String,
        value: String,
    ) -> HandlerResult {
        info!(
            "SetMetadata request from {}: session={}, key={}",
            self.client_id, session_filter, key
        );

        let mut session_manager = self.session_manager.write().await;

        // Find the session by UUID or name
        let session_id = if let Ok(uuid) = Uuid::parse_str(&session_filter) {
            if session_manager.get_session(uuid).is_some() {
                uuid
            } else {
                return HandlerContext::error(
                    ErrorCode::SessionNotFound,
                    format!("Session {} not found", session_filter),
                );
            }
        } else {
            // Try by name
            match session_manager.get_session_by_name(&session_filter) {
                Some(session) => session.id(),
                None => {
                    return HandlerContext::error(
                        ErrorCode::SessionNotFound,
                        format!("Session '{}' not found", session_filter),
                    );
                }
            }
        };

        // Get the session and set the metadata
        let session_name = if let Some(session) = session_manager.get_session_mut(session_id) {
            session.set_metadata(&key, &value);
            session.name().to_string()
        } else {
            return HandlerContext::error(
                ErrorCode::SessionNotFound,
                format!("Session {} not found", session_id),
            );
        };

        // Release lock before persistence call
        drop(session_manager);

        // Log to persistence (BUG-031: metadata must persist across restarts)
        if let Some(persistence_lock) = &self.persistence {
            let persistence = persistence_lock.read().await;
            if let Ok(seq) = persistence.log_session_metadata_set(session_id, &key, &value) {
                persistence.push_replay(seq, ServerMessage::MetadataSet {
                    session_id,
                    session_name: session_name.clone(),
                    key: key.clone(),
                    value: value.clone(),
                });
            }
        }

        HandlerResult::Response(ServerMessage::MetadataSet {
            session_id,
            session_name,
            key,
            value,
        })
    }

    /// Handle GetMetadata - get metadata from a session
    pub async fn handle_get_metadata(
        &self,
        session_filter: String,
        key: Option<String>,
    ) -> HandlerResult {
        debug!(
            "GetMetadata request from {}: session={}, key={:?}",
            self.client_id, session_filter, key
        );

        let session_manager = self.session_manager.read().await;

        // Find the session by UUID or name
        let session = if let Ok(uuid) = Uuid::parse_str(&session_filter) {
            session_manager.get_session(uuid)
        } else {
            session_manager.get_session_by_name(&session_filter)
        };

        let session = match session {
            Some(s) => s,
            None => {
                return HandlerContext::error(
                    ErrorCode::SessionNotFound,
                    format!("Session '{}' not found", session_filter),
                );
            }
        };

        let session_id = session.id();
        let session_name = session.name().to_string();

        // Get metadata - either specific key or all
        let metadata = if let Some(ref k) = key {
            // Get single key
            let mut meta = std::collections::HashMap::new();
            if let Some(v) = session.get_metadata(k) {
                meta.insert(k.clone(), v.clone());
            }
            meta
        } else {
            // Get all
            session.all_metadata().clone()
        };

        HandlerResult::Response(ServerMessage::MetadataList {
            session_id,
            session_name,
            metadata,
        })
    }

    // ==================== FEAT-048: Orchestration Tag Handlers ====================

    /// Handle SetTags - add or remove tags on a session
    pub async fn handle_set_tags(
        &self,
        session_filter: Option<String>,
        add: Vec<String>,
        remove: Vec<String>,
    ) -> HandlerResult {
        info!(
            "SetTags request from {}: session={:?}, add={:?}, remove={:?}",
            self.client_id, session_filter, add, remove
        );

        let mut session_manager = self.session_manager.write().await;

        // Find the session by UUID or name
        let session_id = if let Some(ref filter) = session_filter {
            if let Ok(uuid) = Uuid::parse_str(filter) {
                if session_manager.get_session(uuid).is_some() {
                    uuid
                } else {
                    return HandlerContext::error(
                        ErrorCode::SessionNotFound,
                        format!("Session {} not found", filter),
                    );
                }
            } else {
                // Try by name
                match session_manager.get_session_by_name(filter) {
                    Some(session) => session.id(),
                    None => {
                        return HandlerContext::error(
                            ErrorCode::SessionNotFound,
                            format!("Session '{}' not found", filter),
                        );
                    }
                }
            }
        } else {
            // Use active session if not specified (BUG-034 fix)
            match session_manager.active_session_id() {
                Some(id) => id,
                None => {
                    return HandlerContext::error(
                        ErrorCode::SessionNotFound,
                        "No sessions exist",
                    );
                }
            }
        };

        // Get the session and modify tags
        let (session_name, tags) = if let Some(session) = session_manager.get_session_mut(session_id) {
            // Add tags
            for tag in add {
                session.add_tag(tag);
            }
            // Remove tags
            for tag in &remove {
                session.remove_tag(tag);
            }
            (session.name().to_string(), session.tags().clone())
        } else {
            return HandlerContext::error(
                ErrorCode::SessionNotFound,
                format!("Session {} not found", session_id),
            );
        };

        // Broadcast updated session list to all clients (BUG-032)
        let sessions: Vec<_> = session_manager.list_sessions().iter().map(|s| s.to_info()).collect();

        HandlerResult::ResponseWithGlobalBroadcast {
            response: ServerMessage::TagsSet {
                session_id,
                session_name,
                tags,
            },
            broadcast: ServerMessage::SessionsChanged { sessions },
        }
    }

    /// Handle GetTags - get tags from a session
    ///
    /// BUG-073 FIX: Session parameter is now required. Previously, when session
    /// was omitted, this used `active_session()` which returned the globally
    /// focused session - not the caller's session. This caused agents to see
    /// wrong tags (e.g., workers seeing orchestrator tags) and misidentify their role.
    pub async fn handle_get_tags(
        &self,
        session_filter: Option<String>,
    ) -> HandlerResult {
        debug!(
            "GetTags request from {}: session={:?}",
            self.client_id, session_filter
        );

        // BUG-073 FIX: Require explicit session parameter
        // MCP clients aren't attached to sessions, so we can't infer the caller's session.
        // Falling back to active_session() returned the wrong session's tags.
        let filter = match session_filter {
            Some(f) => f,
            None => {
                return HandlerContext::error(
                    ErrorCode::InvalidOperation,
                    "Session parameter is required. MCP clients must specify which session's tags to retrieve.".to_string(),
                );
            }
        };

        let session_manager = self.session_manager.read().await;

        // Find the session by UUID or name
        let session = if let Ok(uuid) = Uuid::parse_str(&filter) {
            session_manager.get_session(uuid)
        } else {
            session_manager.get_session_by_name(&filter)
        };

        let session = match session {
            Some(s) => s,
            None => {
                return HandlerContext::error(
                    ErrorCode::SessionNotFound,
                    format!("Session '{}' not found", filter),
                );
            }
        };

        let session_id = session.id();
        let session_name = session.name().to_string();
        let tags = session.tags().clone();

        HandlerResult::Response(ServerMessage::TagsList {
            session_id,
            session_name,
            tags,
        })
    }
}
