//! Metrics collection for ccmux
//!
//! Provides a simple, internal metrics system for tracking server health
//! and performance.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::OnceLock;

/// Global metrics collector
pub struct Metrics {
    /// Total number of client resyncs (replay or snapshot)
    pub client_resync_total: AtomicU64,
    /// Total number of client desyncs detected
    pub client_desync_total: AtomicU64,
    /// Total number of events requested for replay
    pub events_replay_requested_total: AtomicU64,
    /// Total number of failed event replays
    pub events_replay_failed_total: AtomicU64,
    
    /// Sum of command latency in milliseconds
    pub command_latency_sum_ms: AtomicU64,
    /// Total number of commands processed
    pub command_count: AtomicU64,
    
    /// Sum of event dispatch time in milliseconds
    pub event_dispatch_ms_sum: AtomicU64,
    /// Total number of events dispatched
    pub event_dispatch_count: AtomicU64,
    
    /// Sum of WAL append time in milliseconds
    pub wal_append_ms_sum: AtomicU64,
    /// Total number of WAL appends
    pub wal_append_count: AtomicU64,
    
    /// Sum of checkpoint duration in milliseconds
    pub checkpoint_duration_ms_sum: AtomicU64,
    /// Total number of checkpoints created
    pub checkpoint_count: AtomicU64,
    
    /// Total bytes written to WAL
    pub wal_bytes_written_total: AtomicU64,
    /// Total bytes written to checkpoints
    pub checkpoint_bytes_written_total: AtomicU64,
    
    /// Total Claude state transitions
    pub claude_state_transitions_total: AtomicU64,
}

impl Metrics {
    /// Get the global metrics instance
    pub fn global() -> &'static Self {
        static INSTANCE: OnceLock<Metrics> = OnceLock::new();
        INSTANCE.get_or_init(|| Self {
            client_resync_total: AtomicU64::new(0),
            client_desync_total: AtomicU64::new(0),
            events_replay_requested_total: AtomicU64::new(0),
            events_replay_failed_total: AtomicU64::new(0),
            command_latency_sum_ms: AtomicU64::new(0),
            command_count: AtomicU64::new(0),
            event_dispatch_ms_sum: AtomicU64::new(0),
            event_dispatch_count: AtomicU64::new(0),
            wal_append_ms_sum: AtomicU64::new(0),
            wal_append_count: AtomicU64::new(0),
            checkpoint_duration_ms_sum: AtomicU64::new(0),
            checkpoint_count: AtomicU64::new(0),
            wal_bytes_written_total: AtomicU64::new(0),
            checkpoint_bytes_written_total: AtomicU64::new(0),
            claude_state_transitions_total: AtomicU64::new(0),
        })
    }

    /// Record a client resync event
    pub fn record_resync(&self) {
        self.client_resync_total.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a client desync detection
    pub fn record_desync(&self) {
        self.client_desync_total.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a replay request
    pub fn record_replay_requested(&self) {
        self.events_replay_requested_total.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a failed replay
    pub fn record_replay_failed(&self) {
        self.events_replay_failed_total.fetch_add(1, Ordering::Relaxed);
    }

    /// Record command latency
    pub fn record_command_latency(&self, latency_ms: u64) {
        self.command_latency_sum_ms.fetch_add(latency_ms, Ordering::Relaxed);
        self.command_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Record event dispatch metrics
    pub fn record_event_dispatch(&self, duration_ms: u64) {
        self.event_dispatch_ms_sum.fetch_add(duration_ms, Ordering::Relaxed);
        self.event_dispatch_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Record WAL append metrics
    pub fn record_wal_append(&self, duration_ms: u64, bytes: u64) {
        self.wal_append_ms_sum.fetch_add(duration_ms, Ordering::Relaxed);
        self.wal_append_count.fetch_add(1, Ordering::Relaxed);
        self.wal_bytes_written_total.fetch_add(bytes, Ordering::Relaxed);
    }

    /// Record checkpoint metrics
    pub fn record_checkpoint(&self, duration_ms: u64, bytes: u64) {
        self.checkpoint_duration_ms_sum.fetch_add(duration_ms, Ordering::Relaxed);
        self.checkpoint_count.fetch_add(1, Ordering::Relaxed);
        self.checkpoint_bytes_written_total.fetch_add(bytes, Ordering::Relaxed);
    }

    /// Record Claude state transition
    pub fn record_claude_transition(&self) {
        self.claude_state_transitions_total.fetch_add(1, Ordering::Relaxed);
    }
}
