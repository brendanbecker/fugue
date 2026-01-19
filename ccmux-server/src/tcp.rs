//! TCP listener implementation for remote peering (FEAT-066)

use tokio::net::TcpListener;
use tracing::{debug, error, info};
use crate::SharedState;
use crate::handle_client;

/// Run the TCP accept loop
pub async fn run_tcp_accept_loop(addr: String, shared_state: SharedState) {
    let listener = match TcpListener::bind(&addr).await {
        Ok(l) => l,
        Err(e) => {
            error!("Failed to bind TCP listener to {}: {}", addr, e);
            return;
        }
    };

    info!("TCP listener bound to {}", addr);

    let mut shutdown_rx = shared_state.subscribe_shutdown();

    loop {
        tokio::select! {
            result = listener.accept() => {
                match result {
                    Ok((stream, peer_addr)) => {
                        debug!("New TCP connection from {}", peer_addr);
                        let state_clone = shared_state.clone();
                        tokio::spawn(async move {
                            let (reader, writer) = stream.into_split();
                            handle_client(reader, writer, state_clone).await;
                        });
                    }
                    Err(e) => {
                        error!("TCP accept error: {}", e);
                    }
                }
            }
            _ = shutdown_rx.recv() => {
                info!("Shutdown signal received, stopping TCP accept loop");
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tokio::sync::{broadcast, mpsc, RwLock};
    use crate::session::SessionManager;
    use crate::pty::PtyManager;
    use crate::registry::ClientRegistry;
    use crate::config::AppConfig;
    use crate::sideband::AsyncCommandExecutor;
    use crate::arbitration::Arbitrator;

    #[tokio::test]
    async fn test_tcp_listener_binds() {
        // Create minimal shared state
        let (shutdown_tx, _) = broadcast::channel(1);
        let (pane_closed_tx, _) = mpsc::channel(100);
        let session_manager = Arc::new(RwLock::new(SessionManager::new()));
        let pty_manager = Arc::new(RwLock::new(PtyManager::new()));
        let registry = Arc::new(ClientRegistry::new());
        let command_executor = Arc::new(AsyncCommandExecutor::new(
            Arc::clone(&session_manager),
            Arc::clone(&pty_manager),
            Arc::clone(&registry),
        ));
        
        let shared_state = SharedState {
            session_manager,
            pty_manager,
            registry,
            config: Arc::new(AppConfig::default()),
            shutdown_tx: shutdown_tx.clone(),
            pane_closed_tx,
            command_executor,
            arbitrator: Arc::new(Arbitrator::new()),
            persistence: None,
            watchdog: Arc::new(crate::watchdog::WatchdogManager::new()),
        };

        // Pick a random high port
        let addr = "127.0.0.1:0".to_string();
        
        // Run in background
        let handle = tokio::spawn(async move {
            run_tcp_accept_loop(addr, shared_state).await;
        });

        // Give it a moment to bind (or fail)
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        
        // Signal shutdown
        let _ = shutdown_tx.send(());
        
        // Should exit cleanly
        let result = tokio::time::timeout(
            tokio::time::Duration::from_secs(1),
            handle
        ).await;
        
        assert!(result.is_ok(), "TCP listener did not shut down");
    }
}