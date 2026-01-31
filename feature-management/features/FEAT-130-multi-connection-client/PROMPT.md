# FEAT-130: Multi-Connection Client Architecture

**Priority**: P1
**Component**: fugue-client
**Effort**: Large
**Status**: new
**Depends**: FEAT-127, FEAT-128

## Summary

Refactor fugue client to maintain simultaneous connections to multiple servers, enabling cross-machine orchestration from a single TUI.

## Problem

Current client has a single `Connection` struct connecting to one server. For multi-machine orchestration, users need to see and control sessions across multiple servers from one interface.

## Current Architecture

```
┌─────────────┐     ┌─────────────┐
│ fugue client│────▶│ fugue server│
│ (single)    │     │ (single)    │
└─────────────┘     └─────────────┘
```

## Proposed Architecture

```
┌──────────────────────────────────────┐
│           fugue client               │
│  ┌────────────────────────────────┐  │
│  │      ConnectionManager         │  │
│  │  ┌──────────┬──────────┬────┐  │  │
│  │  │ local    │ polecats │... │  │  │
│  │  │Connection│Connection│    │  │  │
│  │  └──────────┴──────────┴────┘  │  │
│  └────────────────────────────────┘  │
└──────────────────────────────────────┘
          │           │
          ▼           ▼
    ┌──────────┐ ┌──────────┐
    │  local   │ │ polecats │
    │  server  │ │  server  │
    └──────────┘ └──────────┘
```

## Implementation

### Key Files

| File | Changes |
|------|---------|
| `fugue-client/src/connection/manager.rs` | New - ConnectionManager |
| `fugue-client/src/connection/client.rs` | Refactor to be per-server |
| `fugue-client/src/ui/app.rs` | Use ConnectionManager |
| `fugue-client/src/ui/session_list.rs` | Show sessions from all servers |

### ConnectionManager

```rust
pub struct ConnectionManager {
    connections: HashMap<String, Connection>,  // server_name -> Connection
    config: Config,
}

impl ConnectionManager {
    /// Connect to a server by name
    pub async fn connect(&mut self, name: &str) -> Result<()> {
        if self.connections.contains_key(name) {
            return Ok(());  // Already connected
        }

        let server_config = self.config.servers.get(name)
            .ok_or_else(|| anyhow!("Unknown server: {}", name))?;

        // Handle SSH tunnel if needed (FEAT-129)
        if server_config.ssh_tunnel {
            SshTunnel::establish(server_config)?;
        }

        let conn = Connection::with_addr(server_config.addr.clone());
        conn.connect().await?;

        self.connections.insert(name.to_string(), conn);
        Ok(())
    }

    /// Connect to all configured servers
    pub async fn connect_all(&mut self) -> Result<()> {
        for name in self.config.servers.keys() {
            if let Err(e) = self.connect(name).await {
                warn!("Failed to connect to {}: {}", name, e);
            }
        }
        Ok(())
    }

    /// Get connection by server name
    pub fn get(&self, name: &str) -> Option<&Connection> {
        self.connections.get(name)
    }

    /// List all sessions across all connected servers
    pub async fn list_all_sessions(&self) -> Vec<(String, SessionInfo)> {
        let mut all = Vec::new();
        for (server, conn) in &self.connections {
            if let Ok(sessions) = conn.list_sessions().await {
                for session in sessions {
                    all.push((server.clone(), session));
                }
            }
        }
        all
    }

    /// Send message to specific server
    pub async fn send(&self, server: &str, msg: ClientMessage) -> Result<ServerMessage> {
        let conn = self.connections.get(server)
            .ok_or_else(|| anyhow!("Not connected to {}", server))?;
        conn.send(msg).await
    }
}
```

### Startup Modes

```bash
# Connect to default server only
fugue

# Connect to specific server
fugue -s polecats

# Connect to all configured servers
fugue --all-servers
fugue -a

# Connect to multiple specific servers
fugue -s local -s polecats
```

## Message Routing

All requests must specify which server:

```rust
// Before
conn.send(CreateSession { name: "worker-1" }).await

// After
manager.send("polecats", CreateSession { name: "worker-1" }).await
```

## Acceptance Criteria

- [ ] ConnectionManager handles multiple connections
- [ ] Can connect to subset or all servers
- [ ] Failed connections don't block others
- [ ] Graceful handling of mid-session disconnects
- [ ] Session operations routed to correct server
- [ ] Clean shutdown of all connections

## Related

- FEAT-131: Namespaced sessions (display format)
- FEAT-132: TUI server awareness (UI changes)
- FEAT-133: MCP server parameter (MCP routing)
