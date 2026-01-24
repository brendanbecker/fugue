# Task Breakdown: FEAT-067

**Work Item**: [FEAT-067: Client TCP connection support](PROMPT.md)
**Status**: Not Started
**Last Updated**: 2026-01-13

## Prerequisites

- [ ] Read and understand PROMPT.md
- [ ] Review PLAN.md and update if needed
- [ ] Verify FEAT-066 is completed or has testable TCP daemon
- [ ] Identify affected code paths in fugue-client

## Design Tasks

- [ ] Review existing UnixStream connection implementation
- [ ] Design ConnectionAddr enum (tcp vs unix)
- [ ] Design URL parsing logic (tcp://, unix://)
- [ ] Update PLAN.md with final approach
- [ ] Consider edge cases (IPv6, tilde expansion, etc.)

## CLI and Configuration Tasks

- [ ] Add `--addr` flag to clap Args struct in main.rs
- [ ] Add `FUGUE_ADDR` environment variable support
- [ ] Set default to `unix://~/.fugue/fugue.sock`
- [ ] Test CLI flag parsing with various inputs
- [ ] Verify environment variable takes precedence correctly

## URL Parsing Tasks

- [ ] Implement URL parsing function (tcp://host:port, unix://path)
- [ ] Handle `tcp://` scheme (extract host and port)
- [ ] Handle `unix://` scheme (extract path)
- [ ] Handle tilde expansion for Unix paths
- [ ] Add error handling for invalid URLs
- [ ] Write unit tests for URL parsing

## Connection Logic Tasks

- [ ] Create ConnectionAddr enum
- [ ] Implement `ConnectionAddr::parse()` method
- [ ] Implement `ConnectionAddr::connect()` method
- [ ] Add TCP connection using `tokio::net::TcpStream`
- [ ] Ensure both TCP and Unix return compatible stream types
- [ ] Add connection timeout handling
- [ ] Self-review changes

## Protocol Compatibility Tasks

- [ ] Test bincode message send/receive over TCP
- [ ] Verify message round-trip with TCP daemon
- [ ] Test concurrent connections (multiple clients)
- [ ] Add framing (LengthDelimitedCodec) if needed
- [ ] Ensure compatibility with FEAT-066 daemon

## Error Handling Tasks

- [ ] Add clear error messages for connection failures
- [ ] Handle "connection refused" specifically
- [ ] Handle "no such file" for Unix sockets
- [ ] Handle invalid URL format errors
- [ ] Test error message clarity

## Testing Tasks

- [ ] Test `tcp://localhost:9999` connection
- [ ] Test `unix://~/.fugue/fugue.sock` connection (backward compat)
- [ ] Test with explicit Unix path (no scheme)
- [ ] Test `FUGUE_ADDR` environment variable
- [ ] Test CLI flag override of environment variable
- [ ] Test connection to FEAT-066 TCP daemon
- [ ] Test error cases (daemon not running, wrong port)
- [ ] Run full test suite to ensure no regressions

## Documentation Tasks

- [ ] Update `--help` text for `--addr` flag
- [ ] Document `FUGUE_ADDR` environment variable
- [ ] Add usage examples (TCP and Unix)
- [ ] Add SSH tunnel workflow example
- [ ] Update comments in code where needed

## Verification Tasks

- [ ] All acceptance criteria from PROMPT.md met
- [ ] Tests passing (unit + integration)
- [ ] Backward compatibility verified (Unix socket default)
- [ ] Works with FEAT-066 TCP daemon
- [ ] Update feature_request.json status
- [ ] Document completion in comments.md

## Completion Checklist

- [ ] All implementation tasks complete
- [ ] All tests passing
- [ ] Documentation updated
- [ ] PLAN.md reflects final implementation
- [ ] No regressions in existing Unix socket behavior
- [ ] Ready for review/merge

---
*Check off tasks as you complete them. Update status field above.*
