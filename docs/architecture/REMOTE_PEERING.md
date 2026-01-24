# fugue-peering-design.md

# fugue Peering Design

High-level architecture and flows for adding remote peering support to fugue, enabling a daemon to run on a remote host (e.g., gaming PC) while clients attach from local machines (e.g., laptop). Goal: hybrid orchestration where Mayor runs locally but polecats/agents execute remotely.

## Current Constraints
- Daemon ↔ client: Unix domain socket only (`~/.fugue/fugue.sock`, bincode-serialized messages).
- No TCP listener, no auth, no network transparency.
- Persistence (WAL, per-pane CLAUDE_CONFIG_DIR) is local filesystem-bound.
- MCP bridge and sideband protocol (`<fugue:spawn>`) assume local daemon.

## Design Goals
- Minimal disruption to local workflow (Unix socket remains default).
- Secure remote access (SSH tunnel first, optional TLS later).
- Support multi-client attach (e.g., laptop + secondary monitor machine).
- Preserve Claude state detection, MCP tools, PTY I/O across wire.
- Low latency for TUI redraws and pane navigation.

## Core Approach: Optional TCP Listener + SSH Tunnel
1. **Daemon configurable listen modes**:
   - Unix socket (default, backward compat).
   - TCP (optional, bind to 127.0.0.1:port or configurable addr).

2. **Primary remote flow**: SSH local port forward (no daemon changes needed beyond TCP).
   - Daemon on remote: `--listen tcp://127.0.0.1:9999`
   - Client on local: `ssh -L 9999:localhost:9999 remote-host` then `fugue-client --addr tcp://localhost:9999`

3. **Protocol**: Reuse existing bincode messages over TCP stream (framed if needed for delimiting).

4. **Phase 2 extensions** (post-MVP):
   - TLS (rustls) for direct TCP (no tunnel).
   - Simple auth (shared secret in handshake).
   - Reconnect/resume logic in client.
   - mDNS discovery for local networks.

## Flows

### Local (Unchanged)
```
laptop → fugue-client → unix://~/.fugue/fugue.sock → fugue-server (PTYs, WAL)
```

### Remote via SSH Tunnel (MVP)
```
laptop ── ssh -L 9999:localhost:9999 ── remote-host
       └─ fugue-client ── tcp://localhost:9999 ─┘
                             ↓
                       fugue-server (tcp://127.0.0.1:9999)
                             ↓
                       PTYs / WAL / MCP on remote
```

### Future Direct TCP + TLS
```
laptop ── fugue-client ── tls://remote-host:9999 ── fugue-server
```

## Tradeoffs
| Aspect              | Pro                                      | Con                                      |
|---------------------|------------------------------------------|------------------------------------------|
| SSH Tunnel (MVP)    | Zero new auth/TLS code; uses SSH keys    | Requires SSH setup; slight overhead      |
| Native TCP + TLS    | Direct connect; no tunnel needed         | Adds rustls dependency; cert management  |
| Bincode over TCP    | No protocol rewrite                      | Bincode not length-prefixed by default   |
| TUI over network    | Full ratatui experience                  | Latency-sensitive redraws (test early)   |
| Security            | Tunnel = encrypted/auth'd                | Exposed TCP without TLS = high risk      |

## Risks & Mitigations
- **Latency**: TUI redraws may stutter on high-ping links → test with 50–100ms simulated delay.
- **Reconnects**: Client should retry on drop → add exponential backoff.
- **State sync**: WAL on remote → laptop client must handle disconnect gracefully.
- **Security**: Never bind to 0.0.0.0 without TLS/auth → default 127.0.0.1.
