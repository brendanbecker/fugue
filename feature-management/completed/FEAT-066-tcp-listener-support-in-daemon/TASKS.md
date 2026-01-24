# Task Breakdown: FEAT-066

**Work Item**: [FEAT-066: TCP listener support in daemon](PROMPT.md)
**Status**: Not Started
**Last Updated**: 2026-01-13

## Prerequisites

- [ ] Read and understand PROMPT.md
- [ ] Review PLAN.md architecture decisions
- [ ] Review fugue-server/src/server/mod.rs (existing Unix socket implementation)
- [ ] Review fugue-tcp-implementation-plan.md (overall strategy)

## Section 1: Configuration Support

- [ ] Add `listen_tcp` field to server config struct
- [ ] Add `--listen-tcp` CLI flag to clap argument parser
- [ ] Implement TCP address parsing (host:port format)
- [ ] Add validation for TCP addresses (reject invalid formats)
- [ ] Set default to empty/None (disabled) for backward compatibility
- [ ] Add config example to config.toml comments
- [ ] Test config parsing with valid/invalid addresses

## Section 2: TCP Listener Implementation

- [ ] Add `tokio_util` dependency to Cargo.toml (for `LengthDelimitedCodec`)
- [ ] Create `accept_tcp_loop()` function in server/mod.rs
- [ ] Implement `tokio::net::TcpListener` binding
- [ ] Set `SO_REUSEADDR` socket option for quick restarts
- [ ] Set `TCP_NODELAY` socket option for low latency
- [ ] Spawn separate tokio task for TCP accept loop
- [ ] Add error handling for TCP bind failures
- [ ] Add logging for TCP listener start/stop
- [ ] Ensure concurrent Unix + TCP listeners work

## Section 3: Connection Handling

- [ ] Identify existing connection handler code
- [ ] Make handler generic over `AsyncRead + AsyncWrite` trait
- [ ] Implement length-delimited framing using `LengthDelimitedCodec`
- [ ] Test raw bincode serialization over TCP stream
- [ ] Implement message read/write with proper framing
- [ ] Ensure same message handling logic for both Unix and TCP
- [ ] Add TCP-specific error handling (connection reset, broken pipe)
- [ ] Add logging for TCP connection events (connect/disconnect)
- [ ] Test that Unix socket handling is unchanged

## Section 4: Testing

- [ ] Write unit test for TCP address config parsing
- [ ] Write unit test for message framing (encode/decode)
- [ ] Write integration test: TCP-only connection
- [ ] Write integration test: Unix + TCP concurrent connections
- [ ] Write integration test: message round-trip over TCP
- [ ] Write integration test: daemon restart with TCP enabled
- [ ] Manual test: Connect with netcat to verify TCP listener
- [ ] Manual test: Use tcpdump to inspect TCP traffic
- [ ] Manual test: Test connection reset handling
- [ ] Run full test suite to verify no regressions

## Section 5: Documentation

- [ ] Document `listen_tcp` config option in config.toml
- [ ] Add security notes to config.toml (localhost-only default)
- [ ] Add example configurations (localhost, remote)
- [ ] Update README.md with TCP listener feature
- [ ] Create SECURITY.md section on TCP listener risks
- [ ] Document firewall requirements
- [ ] Document SSH tunnel setup (reference to Phase 2)
- [ ] Add troubleshooting section (connection failures, etc.)

## Verification Tasks

- [ ] Verify daemon accepts `--listen-tcp` CLI flag
- [ ] Verify daemon reads `listen_tcp` from config.toml
- [ ] Verify TCP listener binds to specified address
- [ ] Verify clients can connect via TCP (with netcat)
- [ ] Verify bincode messages work over TCP
- [ ] Verify Unix socket continues to work unchanged
- [ ] Verify default is localhost-only (security check)
- [ ] Verify feature_request.json status updated
- [ ] Document completion in comments.md

## Completion Checklist

- [ ] All configuration tasks complete
- [ ] All implementation tasks complete
- [ ] All testing tasks complete (unit, integration, manual)
- [ ] All documentation tasks complete
- [ ] PLAN.md updated with final implementation details
- [ ] No regressions in existing Unix socket functionality
- [ ] Security review passed (localhost-only default)
- [ ] Ready for review/merge

## Notes

**Security Reminder**: Default to 127.0.0.1 binding. Never bind to 0.0.0.0 by default.

**Testing Priority**: Integration tests are more important than unit tests for this feature. Focus on end-to-end workflows.

**Blocking**: This feature blocks FEAT-067 (client TCP support). Once this is complete, clients can be updated to connect via TCP.

---
*Check off tasks as you complete them. Update status field above.*
