# FEAT-134: Cross-Server Message Routing

**Priority**: P2
**Component**: orchestration
**Effort**: Large
**Status**: new
**Depends**: FEAT-130, FEAT-133

## Summary

Enable orchestration messages to route between sessions on different servers, allowing a nexus on one machine to coordinate workers across multiple machines.

## Problem

Current orchestration (`fugue_send_orchestration`, `fugue_poll_messages`) only works within a single server. With multi-server support, an orchestrator needs to:

1. Send tasks to workers on remote servers
2. Receive status updates from remote workers
3. Route watchdog alerts across servers

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                        Client                                │
│  ┌─────────────────────────────────────────────────────────┐ │
│  │              Message Router                              │ │
│  │  ┌─────────────┐                    ┌─────────────────┐  │ │
│  │  │local:nexus  │◄──────────────────►│polecats:worker-1│  │ │
│  │  │(orchestrator)│                    │    (worker)     │  │ │
│  │  └──────┬──────┘                    └────────┬────────┘  │ │
│  └─────────┼────────────────────────────────────┼───────────┘ │
│            │                                    │             │
└────────────┼────────────────────────────────────┼─────────────┘
             ▼                                    ▼
       ┌──────────┐                        ┌──────────┐
       │  local   │                        │ polecats │
       │  server  │                        │  server  │
       └──────────┘                        └──────────┘
```

## Message Routing

### Extended Target Syntax

```json
{
  "tool": "fugue_send_orchestration",
  "input": {
    "target": {
      "server": "polecats",
      "tag": "worker"
    },
    "msg_type": "task.assigned",
    "payload": { ... }
  }
}
```

Target options:

```json
// Specific session on specific server
{"server": "polecats", "session": "worker-1"}

// All sessions with tag on specific server
{"server": "polecats", "tag": "worker"}

// All sessions with tag across ALL servers
{"tag": "worker", "broadcast_servers": true}

// Specific server's orchestrator
{"server": "polecats", "tag": "orchestrator"}
```

### Client-Side Router

The client maintains the cross-server routing:

```rust
pub struct MessageRouter {
    manager: ConnectionManager,
    pending_deliveries: HashMap<MessageId, PendingDelivery>,
}

impl MessageRouter {
    pub async fn send(&self, target: &Target, msg: OrchestrationMessage) -> Result<()> {
        match target {
            Target::Session { server, session } => {
                // Direct delivery to specific session
                self.manager.send(server, DeliverMessage { session, msg }).await
            }

            Target::Tag { server, tag } => {
                // Deliver to all matching sessions on one server
                self.manager.send(server, BroadcastToTag { tag, msg }).await
            }

            Target::BroadcastTag { tag } => {
                // Fan out to all servers
                for (server_name, conn) in self.manager.connections() {
                    conn.send(BroadcastToTag { tag: tag.clone(), msg: msg.clone() }).await?;
                }
                Ok(())
            }
        }
    }

    pub async fn poll(&self, session_id: &SessionId) -> Vec<OrchestrationMessage> {
        // Poll the specific server for messages to this session
        self.manager
            .send(&session_id.server, PollMessages { session: &session_id.session })
            .await
            .unwrap_or_default()
    }
}
```

### Watchdog Cross-Server Monitoring

Watchdog can monitor workers across servers:

```yaml
# Watchdog checks workers on multiple servers
poll_targets:
  - server: local
    tag: worker
  - server: polecats
    tag: worker
  - server: workstation
    tag: worker
```

When alerting orchestrator:

```json
{
  "msg_type": "worker.stuck",
  "payload": {
    "server": "polecats",
    "session": "worker-feat-127",
    "reason": "No output for 5 minutes"
  }
}
```

## Implementation

### Key Files

| File | Changes |
|------|---------|
| `fugue-client/src/orchestration/router.rs` | New - cross-server routing |
| `fugue-server/src/mcp/bridge/handlers.rs` | Extended target parsing |
| `fugue-protocol/src/orchestration.rs` | Extended Target enum |

### Protocol Changes

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Target {
    // Existing
    Session(String),
    Tag(String),
    Broadcast,

    // New - cross-server
    RemoteSession { server: String, session: String },
    RemoteTag { server: String, tag: String },
    GlobalTag { tag: String },  // All servers
}
```

## Considerations

### Latency

Cross-server messages have higher latency (network round-trip). Consider:
- Async delivery with acknowledgment
- Message batching for efficiency
- Timeout handling for unreachable servers

### Ordering

Messages to the same target should maintain order. Within a server this is guaranteed; cross-server needs sequence numbers or timestamps.

### Failure Modes

- Server disconnected mid-delivery
- Message delivered but ack lost
- Target session died before processing

## Acceptance Criteria

- [ ] `fugue_send_orchestration` accepts `server` in target
- [ ] Messages route to correct remote server
- [ ] `broadcast_servers: true` fans out to all
- [ ] `fugue_poll_messages` works for remote sessions
- [ ] Watchdog can monitor cross-server workers
- [ ] Graceful handling of disconnected servers
- [ ] Clear error messages for routing failures

## Related

- FEAT-130: Multi-connection client (transport layer)
- FEAT-133: MCP server parameter (basic routing)
- FEAT-126: Watchdog mail checking (may use cross-server)
