//! Observability subsystem for fugue
//!
//! Provides metrics, structured logging, and tracing infrastructure.

pub mod http;
pub mod metrics;

pub use http::run_metrics_server;
pub use metrics::Metrics;
