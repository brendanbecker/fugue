//! Metrics collection for ccmux
//!
//! Provides a simple, internal metrics system for tracking server health
//! and performance. Supports Prometheus text format export (FEAT-074).

use dashmap::DashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::OnceLock;

/// Point-in-time gauge values collected on each metrics request
#[derive(Debug, Default)]
pub struct GaugeSnapshot {
    /// Number of active client connections
    pub active_connections: u64,
    /// Number of active sessions
    pub active_sessions: u64,
    /// Number of active panes
    pub active_panes: u64,
    /// Process resident set size in bytes (Linux only)
    pub process_memory_bytes: Option<u64>,
    /// Number of open file descriptors (Linux only)
    pub process_open_fds: Option<u64>,
}

impl GaugeSnapshot {
    /// Collect gauge values from /proc on Linux
    #[cfg(target_os = "linux")]
    pub fn collect_process_metrics(&mut self) {
        // Read RSS from /proc/self/statm (page-based memory stats)
        if let Ok(content) = std::fs::read_to_string("/proc/self/statm") {
            let parts: Vec<&str> = content.split_whitespace().collect();
            if parts.len() >= 2 {
                // Second field is RSS in pages, convert to bytes (assuming 4KB pages)
                if let Ok(pages) = parts[1].parse::<u64>() {
                    self.process_memory_bytes = Some(pages * 4096);
                }
            }
        }

        // Count open file descriptors from /proc/self/fd
        if let Ok(entries) = std::fs::read_dir("/proc/self/fd") {
            self.process_open_fds = Some(entries.count() as u64);
        }
    }

    /// No-op for non-Linux platforms
    #[cfg(not(target_os = "linux"))]
    pub fn collect_process_metrics(&mut self) {
        // Process metrics not available on this platform
    }
}

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

    // FEAT-074: Per-type counters for detailed telemetry
    /// Requests by message type
    pub requests_by_type: DashMap<String, AtomicU64>,
    /// Errors by error code
    pub errors_by_code: DashMap<String, AtomicU64>,
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
            requests_by_type: DashMap::new(),
            errors_by_code: DashMap::new(),
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
    #[allow(dead_code)] // Symmetrical with record_replay_requested; for future use
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

    // FEAT-074: New methods for per-type tracking

    /// Record a request by message type name
    pub fn record_request(&self, msg_type: &str) {
        self.requests_by_type
            .entry(msg_type.to_string())
            .or_insert_with(|| AtomicU64::new(0))
            .fetch_add(1, Ordering::Relaxed);
    }

    /// Record an error by error code name
    pub fn record_error(&self, error_code: &str) {
        self.errors_by_code
            .entry(error_code.to_string())
            .or_insert_with(|| AtomicU64::new(0))
            .fetch_add(1, Ordering::Relaxed);
    }

    /// Export metrics in Prometheus text format
    pub fn to_prometheus(&self, gauges: &GaugeSnapshot) -> String {
        use std::fmt::Write;

        let mut output = String::with_capacity(4096);

        // Helper macro to write a counter metric
        macro_rules! counter {
            ($name:expr, $help:expr, $value:expr) => {
                let _ = writeln!(output, "# HELP {} {}", $name, $help);
                let _ = writeln!(output, "# TYPE {} counter", $name);
                let _ = writeln!(output, "{} {}", $name, $value.load(Ordering::Relaxed));
            };
        }

        // Helper macro to write a gauge metric
        macro_rules! gauge {
            ($name:expr, $help:expr, $value:expr) => {
                let _ = writeln!(output, "# HELP {} {}", $name, $help);
                let _ = writeln!(output, "# TYPE {} gauge", $name);
                let _ = writeln!(output, "{} {}", $name, $value);
            };
        }

        // === Counters ===

        counter!(
            "ccmux_command_count",
            "Total number of commands processed",
            self.command_count
        );
        counter!(
            "ccmux_command_latency_sum_ms",
            "Sum of command latency in milliseconds",
            self.command_latency_sum_ms
        );
        counter!(
            "ccmux_client_resync_total",
            "Total number of client resyncs",
            self.client_resync_total
        );
        counter!(
            "ccmux_client_desync_total",
            "Total number of client desyncs detected",
            self.client_desync_total
        );
        counter!(
            "ccmux_events_replay_requested_total",
            "Total number of events requested for replay",
            self.events_replay_requested_total
        );
        counter!(
            "ccmux_events_replay_failed_total",
            "Total number of failed event replays",
            self.events_replay_failed_total
        );
        counter!(
            "ccmux_wal_append_total",
            "Total number of WAL appends",
            self.wal_append_count
        );
        counter!(
            "ccmux_wal_append_sum_ms",
            "Sum of WAL append time in milliseconds",
            self.wal_append_ms_sum
        );
        counter!(
            "ccmux_wal_bytes_written_total",
            "Total bytes written to WAL",
            self.wal_bytes_written_total
        );
        counter!(
            "ccmux_checkpoint_total",
            "Total number of checkpoints created",
            self.checkpoint_count
        );
        counter!(
            "ccmux_checkpoint_duration_sum_ms",
            "Sum of checkpoint duration in milliseconds",
            self.checkpoint_duration_ms_sum
        );
        counter!(
            "ccmux_checkpoint_bytes_written_total",
            "Total bytes written to checkpoints",
            self.checkpoint_bytes_written_total
        );
        counter!(
            "ccmux_event_dispatch_count",
            "Total number of events dispatched",
            self.event_dispatch_count
        );
        counter!(
            "ccmux_event_dispatch_sum_ms",
            "Sum of event dispatch time in milliseconds",
            self.event_dispatch_ms_sum
        );
        counter!(
            "ccmux_claude_state_transitions_total",
            "Total Claude state transitions",
            self.claude_state_transitions_total
        );

        // === Per-type counters ===

        // Requests by message type
        if !self.requests_by_type.is_empty() {
            let _ = writeln!(
                output,
                "# HELP ccmux_requests_total Total requests by message type"
            );
            let _ = writeln!(output, "# TYPE ccmux_requests_total counter");
            for entry in self.requests_by_type.iter() {
                let _ = writeln!(
                    output,
                    "ccmux_requests_total{{message_type=\"{}\"}} {}",
                    entry.key(),
                    entry.value().load(Ordering::Relaxed)
                );
            }
        }

        // Errors by code
        if !self.errors_by_code.is_empty() {
            let _ = writeln!(output, "# HELP ccmux_errors_total Total errors by code");
            let _ = writeln!(output, "# TYPE ccmux_errors_total counter");
            for entry in self.errors_by_code.iter() {
                let _ = writeln!(
                    output,
                    "ccmux_errors_total{{code=\"{}\"}} {}",
                    entry.key(),
                    entry.value().load(Ordering::Relaxed)
                );
            }
        }

        // === Gauges ===

        gauge!(
            "ccmux_active_connections",
            "Number of active client connections",
            gauges.active_connections
        );
        gauge!(
            "ccmux_active_sessions",
            "Number of active sessions",
            gauges.active_sessions
        );
        gauge!(
            "ccmux_active_panes",
            "Number of active panes",
            gauges.active_panes
        );

        // Process metrics (Linux only)
        if let Some(memory) = gauges.process_memory_bytes {
            gauge!(
                "ccmux_process_memory_bytes",
                "Process resident set size in bytes",
                memory
            );
        }
        if let Some(fds) = gauges.process_open_fds {
            gauge!(
                "ccmux_process_open_fds",
                "Number of open file descriptors",
                fds
            );
        }

        output
    }
}
