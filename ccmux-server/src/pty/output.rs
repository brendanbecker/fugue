//! PTY output polling and broadcasting
//!
//! This module implements background tasks that poll PTY output and broadcast
//! it to connected clients in real-time. Each pane's PTY gets its own polling
//! task that reads output and routes it to all clients attached to that session.

use std::io::Read;
use std::sync::Arc;
use std::time::Duration;

use parking_lot::Mutex;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio::time::{interval, Instant};
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, trace, warn};
use uuid::Uuid;

use ccmux_protocol::ServerMessage;

use crate::registry::ClientRegistry;

/// Default buffer flush timeout in milliseconds
const DEFAULT_FLUSH_TIMEOUT_MS: u64 = 50;

/// Default maximum buffer size before forced flush
const DEFAULT_MAX_BUFFER_SIZE: usize = 16384;

/// Read buffer size for PTY reads
const READ_BUFFER_SIZE: usize = 4096;

/// Configuration for the output poller
#[derive(Debug, Clone)]
pub struct OutputPollerConfig {
    /// Timeout before flushing buffered output (default: 50ms)
    pub flush_timeout: Duration,
    /// Maximum buffer size before forced flush (default: 16KB)
    pub max_buffer_size: usize,
}

impl Default for OutputPollerConfig {
    fn default() -> Self {
        Self {
            flush_timeout: Duration::from_millis(DEFAULT_FLUSH_TIMEOUT_MS),
            max_buffer_size: DEFAULT_MAX_BUFFER_SIZE,
        }
    }
}

/// Handle for managing a running PTY output poller
///
/// Provides access to the cancellation token and join handle for a spawned
/// poller task. Use this to stop the poller when the pane is closed.
#[derive(Debug)]
pub struct PollerHandle {
    /// Token to cancel the poller
    pub cancel_token: CancellationToken,
    /// Handle to the spawned task
    pub join_handle: JoinHandle<()>,
}

impl PollerHandle {
    /// Cancel the poller and wait for it to complete
    pub async fn stop(self) {
        self.cancel_token.cancel();
        // Wait for the task to finish, ignoring any join errors
        let _ = self.join_handle.await;
    }

    /// Cancel the poller without waiting
    pub fn cancel(&self) {
        self.cancel_token.cancel();
    }
}

/// PTY output poller that reads from a PTY and broadcasts to session clients
///
/// Each pane gets its own poller instance that runs in a background task.
/// The poller:
/// - Reads output from the PTY in a blocking manner (via spawn_blocking)
/// - Buffers output for efficient broadcasting
/// - Flushes on newline, timeout, or buffer size threshold
/// - Broadcasts to all clients attached to the session
pub struct PtyOutputPoller {
    /// Pane ID this poller is associated with
    pane_id: Uuid,
    /// Session ID for broadcasting
    session_id: Uuid,
    /// PTY reader wrapped in Arc<Mutex> for thread-safe access
    pty_reader: Arc<Mutex<Box<dyn Read + Send>>>,
    /// Client registry for broadcasting output
    registry: Arc<ClientRegistry>,
    /// Output buffer
    buffer: Vec<u8>,
    /// Configuration
    config: OutputPollerConfig,
    /// Cancellation token for clean shutdown
    cancel_token: CancellationToken,
    /// Last time we received data (for timeout flush)
    last_data_time: Instant,
}

impl PtyOutputPoller {
    /// Spawn a new output poller for a pane
    ///
    /// Returns a handle that can be used to stop the poller.
    pub fn spawn(
        pane_id: Uuid,
        session_id: Uuid,
        pty_reader: Arc<Mutex<Box<dyn Read + Send>>>,
        registry: Arc<ClientRegistry>,
    ) -> PollerHandle {
        Self::spawn_with_config(pane_id, session_id, pty_reader, registry, OutputPollerConfig::default())
    }

    /// Spawn a new output poller with custom configuration
    pub fn spawn_with_config(
        pane_id: Uuid,
        session_id: Uuid,
        pty_reader: Arc<Mutex<Box<dyn Read + Send>>>,
        registry: Arc<ClientRegistry>,
        config: OutputPollerConfig,
    ) -> PollerHandle {
        let cancel_token = CancellationToken::new();
        let poller = Self {
            pane_id,
            session_id,
            pty_reader,
            registry,
            buffer: Vec::with_capacity(config.max_buffer_size),
            config,
            cancel_token: cancel_token.clone(),
            last_data_time: Instant::now(),
        };

        let join_handle = tokio::spawn(poller.run());

        PollerHandle {
            cancel_token,
            join_handle,
        }
    }

    /// Main polling loop
    async fn run(mut self) {
        info!(
            pane_id = %self.pane_id,
            session_id = %self.session_id,
            "PTY output poller started"
        );

        // Channel for receiving data from blocking reads
        let (data_tx, mut data_rx) = mpsc::channel::<ReadResult>(16);

        // Spawn the blocking reader task
        let reader = self.pty_reader.clone();
        let reader_cancel = self.cancel_token.clone();
        let pane_id = self.pane_id;

        tokio::spawn(async move {
            Self::blocking_reader_task(reader, data_tx, reader_cancel, pane_id).await;
        });

        // Create a timer for periodic flush checks
        let mut flush_interval = interval(self.config.flush_timeout);
        flush_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            tokio::select! {
                // Check for cancellation
                _ = self.cancel_token.cancelled() => {
                    debug!(pane_id = %self.pane_id, "Poller cancelled");
                    break;
                }

                // Handle incoming data from the PTY reader
                result = data_rx.recv() => {
                    match result {
                        Some(ReadResult::Data(data)) => {
                            self.handle_output(&data).await;
                        }
                        Some(ReadResult::Eof) => {
                            debug!(pane_id = %self.pane_id, "PTY EOF");
                            // Flush any remaining buffer
                            self.flush().await;
                            break;
                        }
                        Some(ReadResult::Error(e)) => {
                            error!(pane_id = %self.pane_id, error = %e, "PTY read error");
                            // Flush any remaining buffer
                            self.flush().await;
                            break;
                        }
                        None => {
                            // Channel closed - reader task ended
                            debug!(pane_id = %self.pane_id, "Reader channel closed");
                            self.flush().await;
                            break;
                        }
                    }
                }

                // Periodic flush check
                _ = flush_interval.tick() => {
                    if self.should_flush_timeout() {
                        self.flush().await;
                    }
                }
            }
        }

        // Final flush before exiting
        self.flush().await;

        // Notify clients that the pane has closed
        let close_msg = ServerMessage::PaneClosed {
            pane_id: self.pane_id,
            exit_code: None, // We don't have access to the exit code here
        };
        self.registry.broadcast_to_session(self.session_id, close_msg).await;

        info!(
            pane_id = %self.pane_id,
            session_id = %self.session_id,
            "PTY output poller exiting"
        );
    }

    /// Blocking reader task that runs in spawn_blocking
    async fn blocking_reader_task(
        reader: Arc<Mutex<Box<dyn Read + Send>>>,
        data_tx: mpsc::Sender<ReadResult>,
        cancel_token: CancellationToken,
        pane_id: Uuid,
    ) {
        loop {
            // Check cancellation before each read
            if cancel_token.is_cancelled() {
                trace!(pane_id = %pane_id, "Blocking reader cancelled");
                break;
            }

            // Clone what we need for spawn_blocking
            let reader_clone = reader.clone();

            // Perform blocking read in spawn_blocking
            let result = tokio::task::spawn_blocking(move || {
                let mut buf = [0u8; READ_BUFFER_SIZE];
                let mut reader_guard = reader_clone.lock();
                match reader_guard.read(&mut buf) {
                    Ok(0) => ReadResult::Eof,
                    Ok(n) => ReadResult::Data(buf[..n].to_vec()),
                    Err(e) => {
                        // Check for specific error kinds that indicate normal closure
                        if e.kind() == std::io::ErrorKind::BrokenPipe
                            || e.kind() == std::io::ErrorKind::UnexpectedEof
                        {
                            ReadResult::Eof
                        } else {
                            ReadResult::Error(e.to_string())
                        }
                    }
                }
            })
            .await;

            match result {
                Ok(read_result) => {
                    let is_terminal = matches!(read_result, ReadResult::Eof | ReadResult::Error(_));

                    // Send result to main loop
                    if data_tx.send(read_result).await.is_err() {
                        // Main loop has closed - exit
                        trace!(pane_id = %pane_id, "Data channel closed, reader exiting");
                        break;
                    }

                    // If we hit EOF or error, exit the loop
                    if is_terminal {
                        break;
                    }
                }
                Err(e) => {
                    // spawn_blocking panicked or was cancelled
                    warn!(pane_id = %pane_id, error = %e, "spawn_blocking failed");
                    let _ = data_tx.send(ReadResult::Error(e.to_string())).await;
                    break;
                }
            }
        }
    }

    /// Handle new output data
    async fn handle_output(&mut self, data: &[u8]) {
        self.buffer.extend_from_slice(data);
        self.last_data_time = Instant::now();

        trace!(
            pane_id = %self.pane_id,
            bytes = data.len(),
            buffer_size = self.buffer.len(),
            "Received PTY output"
        );

        // Check if we should flush
        if self.should_flush() {
            self.flush().await;
        }
    }

    /// Check if buffer should be flushed
    fn should_flush(&self) -> bool {
        // Flush if buffer exceeds max size
        if self.buffer.len() >= self.config.max_buffer_size {
            return true;
        }

        // Flush if buffer contains a newline
        if self.buffer.contains(&b'\n') {
            return true;
        }

        false
    }

    /// Check if we should flush due to timeout
    fn should_flush_timeout(&self) -> bool {
        !self.buffer.is_empty() && self.last_data_time.elapsed() >= self.config.flush_timeout
    }

    /// Flush the buffer by broadcasting to session clients
    async fn flush(&mut self) {
        if self.buffer.is_empty() {
            return;
        }

        let data = std::mem::take(&mut self.buffer);
        self.buffer = Vec::with_capacity(self.config.max_buffer_size);

        trace!(
            pane_id = %self.pane_id,
            session_id = %self.session_id,
            bytes = data.len(),
            "Flushing output to session"
        );

        let msg = ServerMessage::Output {
            pane_id: self.pane_id,
            data,
        };

        let delivered = self.registry.broadcast_to_session(self.session_id, msg).await;

        if delivered == 0 {
            trace!(
                pane_id = %self.pane_id,
                session_id = %self.session_id,
                "No clients received output (session may have no attached clients)"
            );
        }
    }
}

/// Result of a PTY read operation
#[derive(Debug)]
enum ReadResult {
    /// Successfully read data
    Data(Vec<u8>),
    /// End of file (PTY closed)
    Eof,
    /// Read error
    Error(String),
}

/// Manages output pollers for multiple panes
///
/// Provides a central place to track, start, and stop output pollers.
/// Use this to ensure pollers are properly cleaned up when panes close.
#[derive(Default)]
pub struct PollerManager {
    /// Active pollers by pane ID
    handles: std::collections::HashMap<Uuid, PollerHandle>,
}

impl PollerManager {
    /// Create a new empty poller manager
    pub fn new() -> Self {
        Self::default()
    }

    /// Start a new poller for a pane
    ///
    /// If a poller already exists for this pane, it will be stopped first.
    pub fn start(
        &mut self,
        pane_id: Uuid,
        session_id: Uuid,
        pty_reader: Arc<Mutex<Box<dyn Read + Send>>>,
        registry: Arc<ClientRegistry>,
    ) {
        self.start_with_config(pane_id, session_id, pty_reader, registry, OutputPollerConfig::default())
    }

    /// Start a new poller with custom configuration
    pub fn start_with_config(
        &mut self,
        pane_id: Uuid,
        session_id: Uuid,
        pty_reader: Arc<Mutex<Box<dyn Read + Send>>>,
        registry: Arc<ClientRegistry>,
        config: OutputPollerConfig,
    ) {
        // Stop existing poller if any
        if let Some(old_handle) = self.handles.remove(&pane_id) {
            old_handle.cancel();
            debug!(pane_id = %pane_id, "Stopped existing poller before starting new one");
        }

        let handle = PtyOutputPoller::spawn_with_config(
            pane_id,
            session_id,
            pty_reader,
            registry,
            config,
        );

        self.handles.insert(pane_id, handle);
        debug!(pane_id = %pane_id, "Started output poller");
    }

    /// Stop a poller for a pane (non-blocking)
    ///
    /// Returns true if a poller was found and cancelled.
    pub fn stop(&mut self, pane_id: Uuid) -> bool {
        if let Some(handle) = self.handles.remove(&pane_id) {
            handle.cancel();
            debug!(pane_id = %pane_id, "Stopped output poller");
            true
        } else {
            false
        }
    }

    /// Stop a poller and wait for it to complete
    pub async fn stop_and_wait(&mut self, pane_id: Uuid) -> bool {
        if let Some(handle) = self.handles.remove(&pane_id) {
            handle.stop().await;
            debug!(pane_id = %pane_id, "Stopped and waited for output poller");
            true
        } else {
            false
        }
    }

    /// Stop all pollers (non-blocking)
    pub fn stop_all(&mut self) {
        for (pane_id, handle) in self.handles.drain() {
            handle.cancel();
            debug!(pane_id = %pane_id, "Stopped output poller (stop_all)");
        }
    }

    /// Stop all pollers and wait for them to complete
    pub async fn stop_all_and_wait(&mut self) {
        let handles: Vec<_> = self.handles.drain().collect();
        for (pane_id, handle) in handles {
            handle.stop().await;
            debug!(pane_id = %pane_id, "Stopped and waited for output poller (stop_all)");
        }
    }

    /// Check if a poller is running for a pane
    pub fn has_poller(&self, pane_id: Uuid) -> bool {
        self.handles.contains_key(&pane_id)
    }

    /// Get the number of active pollers
    pub fn count(&self) -> usize {
        self.handles.len()
    }

    /// Get all pane IDs with active pollers
    pub fn pane_ids(&self) -> Vec<Uuid> {
        self.handles.keys().copied().collect()
    }
}

impl std::fmt::Debug for PollerManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PollerManager")
            .field("count", &self.handles.len())
            .field("pane_ids", &self.handles.keys().collect::<Vec<_>>())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use tokio::sync::mpsc as tokio_mpsc;
    use tokio::time::timeout;

    // Helper to create a mock registry
    fn create_test_registry() -> Arc<ClientRegistry> {
        Arc::new(ClientRegistry::new())
    }

    // Helper to create a reader from bytes
    fn create_reader(data: &[u8]) -> Arc<Mutex<Box<dyn Read + Send>>> {
        Arc::new(Mutex::new(Box::new(Cursor::new(data.to_vec()))))
    }

    #[tokio::test]
    async fn test_output_poller_config_default() {
        let config = OutputPollerConfig::default();
        assert_eq!(config.flush_timeout, Duration::from_millis(50));
        assert_eq!(config.max_buffer_size, 16384);
    }

    #[tokio::test]
    async fn test_poller_handle_stop() {
        let pane_id = Uuid::new_v4();
        let session_id = Uuid::new_v4();
        let registry = create_test_registry();

        // Create a reader that will block (empty cursor will return EOF immediately)
        let reader = create_reader(b"");

        let handle = PtyOutputPoller::spawn(pane_id, session_id, reader, registry);

        // Stop should complete quickly
        let result = timeout(Duration::from_secs(1), handle.stop()).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_poller_handle_cancel() {
        let pane_id = Uuid::new_v4();
        let session_id = Uuid::new_v4();
        let registry = create_test_registry();
        let reader = create_reader(b"");

        let handle = PtyOutputPoller::spawn(pane_id, session_id, reader, registry);

        // Cancel should not block
        handle.cancel();

        // Task should eventually complete
        let result = timeout(Duration::from_secs(1), handle.join_handle).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_poller_broadcasts_output() {
        let pane_id = Uuid::new_v4();
        let session_id = Uuid::new_v4();
        let registry = create_test_registry();

        // Create a client attached to the session
        let (tx, mut rx) = tokio_mpsc::channel(10);
        let client_id = registry.register_client(tx);
        registry.attach_to_session(client_id, session_id);

        // Create reader with test data
        let test_data = b"Hello, World!\n";
        let reader = create_reader(test_data);

        let handle = PtyOutputPoller::spawn(pane_id, session_id, reader, registry);

        // Wait for message to be received
        let msg = timeout(Duration::from_secs(2), rx.recv()).await;

        // Stop the poller
        handle.cancel();

        // Verify the message
        assert!(msg.is_ok());
        let msg = msg.unwrap();
        assert!(msg.is_some());

        if let Some(ServerMessage::Output { pane_id: pid, data }) = msg {
            assert_eq!(pid, pane_id);
            assert_eq!(data, test_data);
        } else {
            panic!("Expected Output message");
        }
    }

    #[tokio::test]
    async fn test_poller_flushes_on_newline() {
        let pane_id = Uuid::new_v4();
        let session_id = Uuid::new_v4();
        let registry = create_test_registry();

        let (tx, mut rx) = tokio_mpsc::channel(10);
        let client_id = registry.register_client(tx);
        registry.attach_to_session(client_id, session_id);

        // Data with newline should flush immediately
        let test_data = b"line1\n";
        let reader = create_reader(test_data);

        let handle = PtyOutputPoller::spawn(pane_id, session_id, reader, registry);

        // Should receive message quickly (within flush timeout)
        let msg = timeout(Duration::from_millis(100), rx.recv()).await;
        handle.cancel();

        assert!(msg.is_ok());
    }

    #[tokio::test]
    async fn test_poller_eof_handling() {
        let pane_id = Uuid::new_v4();
        let session_id = Uuid::new_v4();
        let registry = create_test_registry();

        // Empty reader will immediately return EOF
        let reader = create_reader(b"");

        let handle = PtyOutputPoller::spawn(pane_id, session_id, reader, registry);

        // Task should complete due to EOF
        let result = timeout(Duration::from_secs(2), handle.join_handle).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_poller_no_clients() {
        let pane_id = Uuid::new_v4();
        let session_id = Uuid::new_v4();
        let registry = create_test_registry();

        // No clients attached - should still work without panicking
        let test_data = b"Hello\n";
        let reader = create_reader(test_data);

        let handle = PtyOutputPoller::spawn(pane_id, session_id, reader, registry);

        // Give it time to process
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Should complete cleanly
        let result = timeout(Duration::from_secs(1), handle.stop()).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_poller_custom_config() {
        let pane_id = Uuid::new_v4();
        let session_id = Uuid::new_v4();
        let registry = create_test_registry();
        let reader = create_reader(b"");

        let config = OutputPollerConfig {
            flush_timeout: Duration::from_millis(10),
            max_buffer_size: 1024,
        };

        let handle = PtyOutputPoller::spawn_with_config(
            pane_id,
            session_id,
            reader,
            registry,
            config,
        );

        // Should work with custom config
        let result = timeout(Duration::from_secs(1), handle.stop()).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_poller_multiple_outputs() {
        let pane_id = Uuid::new_v4();
        let session_id = Uuid::new_v4();
        let registry = create_test_registry();

        let (tx, mut rx) = tokio_mpsc::channel(10);
        let client_id = registry.register_client(tx);
        registry.attach_to_session(client_id, session_id);

        // Multiple lines - may be batched or separate
        let test_data = b"line1\nline2\nline3\n";
        let reader = create_reader(test_data);

        let handle = PtyOutputPoller::spawn(pane_id, session_id, reader, registry);

        // Collect all messages
        let mut received = Vec::new();
        loop {
            match timeout(Duration::from_millis(200), rx.recv()).await {
                Ok(Some(msg)) => received.push(msg),
                _ => break,
            }
        }

        handle.cancel();

        // Should have received at least one message with the data
        assert!(!received.is_empty());

        // Verify all data was received
        let mut all_data = Vec::new();
        for msg in received {
            if let ServerMessage::Output { data, .. } = msg {
                all_data.extend(data);
            }
        }
        assert_eq!(all_data, test_data);
    }

    #[test]
    fn test_read_result_debug() {
        let data = ReadResult::Data(vec![1, 2, 3]);
        let eof = ReadResult::Eof;
        let err = ReadResult::Error("test".to_string());

        // Should not panic
        let _ = format!("{:?}", data);
        let _ = format!("{:?}", eof);
        let _ = format!("{:?}", err);
    }

    // ==================== PollerManager Tests ====================

    #[test]
    fn test_poller_manager_new() {
        let manager = PollerManager::new();
        assert_eq!(manager.count(), 0);
    }

    #[test]
    fn test_poller_manager_default() {
        let manager = PollerManager::default();
        assert_eq!(manager.count(), 0);
    }

    #[test]
    fn test_poller_manager_debug() {
        let manager = PollerManager::new();
        let debug = format!("{:?}", manager);
        assert!(debug.contains("PollerManager"));
        assert!(debug.contains("count"));
    }

    #[tokio::test]
    async fn test_poller_manager_start_stop() {
        let mut manager = PollerManager::new();
        let pane_id = Uuid::new_v4();
        let session_id = Uuid::new_v4();
        let registry = create_test_registry();
        let reader = create_reader(b"");

        manager.start(pane_id, session_id, reader, registry);

        assert!(manager.has_poller(pane_id));
        assert_eq!(manager.count(), 1);

        // Give poller time to start
        tokio::time::sleep(Duration::from_millis(10)).await;

        let stopped = manager.stop(pane_id);
        assert!(stopped);
        assert!(!manager.has_poller(pane_id));
        assert_eq!(manager.count(), 0);
    }

    #[tokio::test]
    async fn test_poller_manager_stop_nonexistent() {
        let mut manager = PollerManager::new();
        let pane_id = Uuid::new_v4();

        let stopped = manager.stop(pane_id);
        assert!(!stopped);
    }

    #[tokio::test]
    async fn test_poller_manager_stop_and_wait() {
        let mut manager = PollerManager::new();
        let pane_id = Uuid::new_v4();
        let session_id = Uuid::new_v4();
        let registry = create_test_registry();
        let reader = create_reader(b"");

        manager.start(pane_id, session_id, reader, registry);

        let stopped = manager.stop_and_wait(pane_id).await;
        assert!(stopped);
        assert!(!manager.has_poller(pane_id));
    }

    #[tokio::test]
    async fn test_poller_manager_stop_all() {
        let mut manager = PollerManager::new();
        let registry = create_test_registry();

        // Start multiple pollers
        for _ in 0..3 {
            let pane_id = Uuid::new_v4();
            let session_id = Uuid::new_v4();
            let reader = create_reader(b"");
            manager.start(pane_id, session_id, reader, registry.clone());
        }

        assert_eq!(manager.count(), 3);

        manager.stop_all();
        assert_eq!(manager.count(), 0);
    }

    #[tokio::test]
    async fn test_poller_manager_stop_all_and_wait() {
        let mut manager = PollerManager::new();
        let registry = create_test_registry();

        // Start multiple pollers
        for _ in 0..3 {
            let pane_id = Uuid::new_v4();
            let session_id = Uuid::new_v4();
            let reader = create_reader(b"");
            manager.start(pane_id, session_id, reader, registry.clone());
        }

        assert_eq!(manager.count(), 3);

        manager.stop_all_and_wait().await;
        assert_eq!(manager.count(), 0);
    }

    #[tokio::test]
    async fn test_poller_manager_pane_ids() {
        let mut manager = PollerManager::new();
        let registry = create_test_registry();

        let pane1 = Uuid::new_v4();
        let pane2 = Uuid::new_v4();
        let session_id = Uuid::new_v4();

        manager.start(pane1, session_id, create_reader(b""), registry.clone());
        manager.start(pane2, session_id, create_reader(b""), registry);

        let ids = manager.pane_ids();
        assert_eq!(ids.len(), 2);
        assert!(ids.contains(&pane1));
        assert!(ids.contains(&pane2));

        manager.stop_all();
    }

    #[tokio::test]
    async fn test_poller_manager_restart_replaces() {
        let mut manager = PollerManager::new();
        let pane_id = Uuid::new_v4();
        let session_id = Uuid::new_v4();
        let registry = create_test_registry();

        // Start first poller
        manager.start(pane_id, session_id, create_reader(b""), registry.clone());
        assert_eq!(manager.count(), 1);

        // Start again - should replace
        manager.start(pane_id, session_id, create_reader(b""), registry);
        assert_eq!(manager.count(), 1);

        manager.stop_all();
    }

    #[tokio::test]
    async fn test_poller_manager_with_config() {
        let mut manager = PollerManager::new();
        let pane_id = Uuid::new_v4();
        let session_id = Uuid::new_v4();
        let registry = create_test_registry();
        let reader = create_reader(b"");

        let config = OutputPollerConfig {
            flush_timeout: Duration::from_millis(10),
            max_buffer_size: 1024,
        };

        manager.start_with_config(pane_id, session_id, reader, registry, config);

        assert!(manager.has_poller(pane_id));
        manager.stop_all();
    }
}
