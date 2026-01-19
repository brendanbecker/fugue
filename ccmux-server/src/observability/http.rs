//! HTTP metrics endpoint for Prometheus scraping (FEAT-074)
//!
//! Provides a lightweight HTTP server that exposes metrics at `/metrics`
//! in Prometheus text format.

use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;

use http_body_util::Full;
use hyper::body::Bytes;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Method, Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use tokio::net::TcpListener;
use tracing::{error, info, warn};

use super::metrics::{GaugeSnapshot, Metrics};
use crate::SharedState;

/// Run the metrics HTTP server
///
/// Listens on the configured address and serves `/metrics` endpoint
/// with Prometheus-format metrics data.
pub async fn run_metrics_server(addr: String, state: Arc<SharedState>) {
    let socket_addr: SocketAddr = match addr.parse() {
        Ok(a) => a,
        Err(e) => {
            error!("Invalid metrics listen address '{}': {}", addr, e);
            return;
        }
    };

    let listener = match TcpListener::bind(socket_addr).await {
        Ok(l) => l,
        Err(e) => {
            error!("Failed to bind metrics server to {}: {}", socket_addr, e);
            return;
        }
    };

    info!("Metrics server listening on http://{}/metrics", socket_addr);

    // Get shutdown receiver
    let mut shutdown_rx = state.shutdown_tx.subscribe();

    loop {
        tokio::select! {
            accept_result = listener.accept() => {
                let (stream, remote_addr) = match accept_result {
                    Ok(conn) => conn,
                    Err(e) => {
                        warn!("Metrics server accept error: {}", e);
                        continue;
                    }
                };

                let io = TokioIo::new(stream);
                let state_clone = Arc::clone(&state);

                tokio::spawn(async move {
                    let service = service_fn(move |req| {
                        let state = Arc::clone(&state_clone);
                        async move { handle_request(req, state).await }
                    });

                    if let Err(e) = http1::Builder::new()
                        .serve_connection(io, service)
                        .await
                    {
                        // Connection errors are expected when clients disconnect
                        if !e.is_incomplete_message() {
                            warn!("Metrics connection error from {}: {}", remote_addr, e);
                        }
                    }
                });
            }

            _ = shutdown_rx.recv() => {
                info!("Metrics server shutting down");
                break;
            }
        }
    }
}

/// Handle an HTTP request
async fn handle_request(
    req: Request<hyper::body::Incoming>,
    state: Arc<SharedState>,
) -> Result<Response<Full<Bytes>>, Infallible> {
    match (req.method(), req.uri().path()) {
        (&Method::GET, "/metrics") => Ok(serve_metrics(state).await),
        (&Method::GET, "/") => Ok(serve_index()),
        (&Method::GET, "/health") => Ok(serve_health()),
        _ => Ok(not_found()),
    }
}

/// Serve the metrics endpoint
async fn serve_metrics(state: Arc<SharedState>) -> Response<Full<Bytes>> {
    // Collect gauge values
    let mut gauges = GaugeSnapshot {
        active_connections: state.registry.client_count() as u64,
        ..Default::default()
    };

    // Get session and pane counts
    {
        let session_manager = state.session_manager.read().await;
        gauges.active_sessions = session_manager.session_count() as u64;

        // Count all panes
        let mut pane_count = 0u64;
        for session in session_manager.list_sessions() {
            for window in session.windows() {
                pane_count += window.pane_count() as u64;
            }
        }
        gauges.active_panes = pane_count;
    }

    // Collect process metrics (Linux only)
    gauges.collect_process_metrics();

    // Generate Prometheus output
    let metrics = Metrics::global();
    let body = metrics.to_prometheus(&gauges);

    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/plain; version=0.0.4; charset=utf-8")
        .body(Full::new(Bytes::from(body)))
        .unwrap()
}

/// Serve a simple index page
fn serve_index() -> Response<Full<Bytes>> {
    let body = r#"<!DOCTYPE html>
<html>
<head><title>ccmux Metrics</title></head>
<body>
<h1>ccmux Metrics Server</h1>
<p><a href="/metrics">Metrics</a></p>
<p><a href="/health">Health Check</a></p>
</body>
</html>
"#;

    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/html; charset=utf-8")
        .body(Full::new(Bytes::from(body)))
        .unwrap()
}

/// Serve a health check endpoint
fn serve_health() -> Response<Full<Bytes>> {
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/plain")
        .body(Full::new(Bytes::from("OK")))
        .unwrap()
}

/// Return 404 Not Found
fn not_found() -> Response<Full<Bytes>> {
    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body(Full::new(Bytes::from("Not Found")))
        .unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gauge_snapshot_default() {
        let gauges = GaugeSnapshot::default();
        assert_eq!(gauges.active_connections, 0);
        assert_eq!(gauges.active_sessions, 0);
        assert_eq!(gauges.active_panes, 0);
        assert!(gauges.process_memory_bytes.is_none());
        assert!(gauges.process_open_fds.is_none());
    }

    #[test]
    fn test_metrics_to_prometheus() {
        let metrics = Metrics::global();
        let gauges = GaugeSnapshot {
            active_connections: 5,
            active_sessions: 2,
            active_panes: 10,
            process_memory_bytes: Some(1024 * 1024),
            process_open_fds: Some(25),
        };

        let output = metrics.to_prometheus(&gauges);

        // Check for expected metric names
        assert!(output.contains("ccmux_command_count"));
        assert!(output.contains("ccmux_active_connections"));
        assert!(output.contains("ccmux_active_sessions"));
        assert!(output.contains("ccmux_active_panes"));
        assert!(output.contains("ccmux_process_memory_bytes"));
        assert!(output.contains("ccmux_process_open_fds"));

        // Check for expected values
        assert!(output.contains("ccmux_active_connections 5"));
        assert!(output.contains("ccmux_active_sessions 2"));
        assert!(output.contains("ccmux_active_panes 10"));
    }

    #[test]
    fn test_serve_index() {
        let response = serve_index();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[test]
    fn test_serve_health() {
        let response = serve_health();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[test]
    fn test_not_found() {
        let response = not_found();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}
