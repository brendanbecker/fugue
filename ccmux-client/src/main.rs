//! ccmux client - Terminal UI for ccmux

use ccmux_protocol::ClientMessage;
use ccmux_utils::{init_logging_with_config, LogConfig, Result};
use uuid::Uuid;

mod connection;
mod input;
mod ui;

use connection::Connection;

#[tokio::main]
async fn main() -> Result<()> {
    init_logging_with_config(LogConfig::client())?;
    tracing::info!("ccmux client starting");

    // Create connection
    let mut conn = Connection::new();

    // Try to connect
    match conn.connect().await {
        Ok(()) => {
            tracing::info!("Connected to server");

            // Send handshake
            let client_id = Uuid::new_v4();
            conn.send(ClientMessage::Connect {
                client_id,
                protocol_version: ccmux_protocol::PROTOCOL_VERSION,
            })
            .await?;

            // Wait for response
            if let Some(msg) = conn.recv().await {
                tracing::info!("Received: {:?}", msg);
            }
        }
        Err(e) => {
            tracing::error!("Failed to connect: {}", e);
            eprintln!("Error: {}", e);
            eprintln!("Is the ccmux server running?");
        }
    }

    Ok(())
}
