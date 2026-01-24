use tracing::{info, debug};
use uuid::Uuid;
use ccmux_protocol::{ErrorCode, ServerMessage};
use crate::handlers::{HandlerContext, HandlerResult};

impl HandlerContext {
    /// Handle SetEnvironment - set an environment variable on a session
    pub async fn handle_set_environment(
        &self,
        session_filter: String,
        key: String,
        value: String,
    ) -> HandlerResult {
        info!(
            "SetEnvironment request from {}: session={}, key={}",
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

        // Get the session and set the environment variable
        let session_name = if let Some(session) = session_manager.get_session_mut(session_id) {
            session.set_env(&key, &value);
            session.name().to_string()
        } else {
            return HandlerContext::error(
                ErrorCode::SessionNotFound,
                format!("Session {} not found", session_id),
            );
        };

        // Release lock before persistence call
        drop(session_manager);

        // Log to persistence (FEAT-086: environment must persist across restarts)
        if let Some(persistence_lock) = &self.persistence {
            let persistence = persistence_lock.read().await;
            if let Ok(seq) = persistence.log_session_environment_set(session_id, &key, &value) {
                persistence.push_replay(seq, ServerMessage::EnvironmentSet {
                    session_id,
                    session_name: session_name.clone(),
                    key: key.clone(),
                    value: value.clone(),
                });
            }
        }

        HandlerResult::Response(ServerMessage::EnvironmentSet {
            session_id,
            session_name,
            key,
            value,
        })
    }

    /// Handle GetEnvironment - get environment variables from a session
    pub async fn handle_get_environment(
        &self,
        session_filter: String,
        key: Option<String>,
    ) -> HandlerResult {
        debug!(
            "GetEnvironment request from {}: session={}, key={:?}",
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

        // Get environment - either specific key or all
        let environment = if let Some(ref k) = key {
            // Get single key
            let mut env = std::collections::HashMap::new();
            if let Some(v) = session.get_env(k) {
                env.insert(k.clone(), v.clone());
            }
            env
        } else {
            // Get all
            session.environment().clone()
        };

        HandlerResult::Response(ServerMessage::EnvironmentList {
            session_id,
            session_name,
            environment,
        })
    }
}
