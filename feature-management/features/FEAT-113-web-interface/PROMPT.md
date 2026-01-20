# FEAT-113: Web Interface Core (Desktop MVP)

**Priority**: P2
**Component**: ccmux-web (new crate)
**Effort**: Medium
**Status**: new

## Summary

Add a web interface to ccmux that allows browser-based access to the terminal multiplexer. Stream the existing TUI through xterm.js over WebSocket, enabling remote access without SSH.

## Related Features

- **FEAT-114**: Mobile Responsive Layout (depends on this)
- **FEAT-115**: Voice Input (depends on this)
- **FEAT-116**: Web Security/Auth (depends on this)

## Motivation

- **Remote Access**: Users need to access ccmux sessions from anywhere without SSH
- **Unified Interface**: Same TUI experience in browser as native terminal
- **Foundation**: Enables mobile and voice features in subsequent work

## Architecture: TUI Streaming

**Approach**: Stream the existing ratatui/crossterm TUI through xterm.js

**Rationale**:
- Single codebase - no separate web UI to maintain
- All existing features work immediately (keybinds, mouse support, visual mode)
- Proven pattern (ttyd, wetty, gotty use this approach)

**Architecture**:
```
Browser (xterm.js)
    ↕ WebSocket
ccmux-web server
    ↕ PTY
ccmux-client (existing TUI)
    ↕ Unix socket
ccmux-server (existing daemon)
```

## Implementation

### New Crate: `ccmux-web`

**Location**: `ccmux-web/` alongside existing crates

**Dependencies**:
```toml
[dependencies]
axum = { version = "0.7", features = ["ws"] }
tokio = { version = "1", features = ["full"] }
tokio-tungstenite = "0.21"
portable-pty = "0.8"
tower-http = { version = "0.5", features = ["fs"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tracing = "0.1"
```

**Core Components**:

1. **WebSocket Server** (`src/ws.rs`)
   - Accept browser connections
   - Handle message routing
   - Connection lifecycle management

2. **PTY Bridge** (`src/pty_bridge.rs`)
   - Spawn `ccmux-client` in PTY
   - Bridge PTY I/O to WebSocket bidirectionally
   - Handle resize events

3. **Static File Server** (`src/static_files.rs`)
   - Serve HTML/CSS/JS assets
   - Embedded or filesystem-based

4. **Session Management** (`src/sessions.rs`)
   - Map WebSocket connections to PTY instances
   - Handle reconnection
   - Cleanup on disconnect

### Frontend (Static Assets)

**Location**: `ccmux-web/static/`

**Files**:
- `index.html` - Main page with xterm container
- `app.js` - xterm.js initialization and WebSocket handling
- `styles.css` - Basic terminal styling

**Dependencies** (CDN or bundled):
- `xterm.js` - Terminal emulator
- `xterm-addon-fit` - Responsive terminal sizing

**xterm.js Setup**:
```javascript
import { Terminal } from 'xterm';
import { FitAddon } from 'xterm-addon-fit';

const term = new Terminal({
  scrollback: 1000,
  cursorBlink: true,
});

const fitAddon = new FitAddon();
term.loadAddon(fitAddon);
term.open(document.getElementById('terminal'));
fitAddon.fit();

// WebSocket connection
const ws = new WebSocket(`ws://${location.host}/ws`);

ws.onmessage = (event) => {
  term.write(event.data);
};

term.onData((data) => {
  ws.send(JSON.stringify({ type: 'input', data }));
});

// Handle resize
window.addEventListener('resize', () => {
  fitAddon.fit();
  ws.send(JSON.stringify({
    type: 'resize',
    cols: term.cols,
    rows: term.rows
  }));
});
```

### WebSocket Protocol

**Message Types**:

```typescript
// Client → Server
{ type: 'input', data: string }      // Keyboard input
{ type: 'resize', cols: number, rows: number }  // Terminal resize

// Server → Client
// Raw terminal output (binary/text frames)
```

### PTY Bridge Flow

```
1. Browser connects to /ws
2. Server spawns: ccmux-client --connect <socket>
3. PTY stdout → WebSocket send
4. WebSocket recv → PTY stdin
5. On WebSocket close:
   - Keep PTY alive for reconnect_timeout (default 30s)
   - After timeout, SIGHUP to ccmux-client
```

## Configuration

**New section in** `~/.ccmux/config.toml`:
```toml
[web]
enabled = false          # Disabled by default
host = "127.0.0.1"       # Localhost only by default
port = 8080
reconnect_timeout = 30   # Seconds to keep PTY alive after disconnect
```

## Desktop Web Features (This MVP)

- [x] Full TUI rendering via xterm.js
- [x] WebSocket-based real-time communication
- [x] Mouse support (clicking to select panes)
- [x] Copy/paste with Shift+drag selection
- [x] All existing keybinds work (`Ctrl+b`, etc.)
- [x] Terminal resizes with browser window
- [x] Auto-reconnection on WebSocket drop
- [x] Connection status indicator

## Acceptance Criteria

- [ ] New `ccmux-web` crate compiles and runs
- [ ] `ccmux-web` binary starts HTTP/WebSocket server
- [ ] Browser at `http://localhost:8080` shows ccmux TUI
- [ ] Keyboard input works (all keybinds functional)
- [ ] Mouse events forwarded correctly (pane selection works)
- [ ] Copy/paste with Shift+drag works
- [ ] Terminal resizes with browser window
- [ ] Reconnection works after brief network drop
- [ ] PTY cleaned up after reconnect timeout
- [ ] Configuration loaded from config.toml

## Testing

### Unit Tests
- WebSocket message parsing
- PTY spawn and cleanup
- Session management

### Integration Tests
- End-to-end: browser → WebSocket → PTY → ccmux-client → ccmux-server
- Reconnection after disconnect
- Multiple simultaneous connections

### Manual Testing
- [ ] Chrome
- [ ] Firefox
- [ ] Safari
- [ ] Copy/paste with Shift+drag
- [ ] Mouse pane selection
- [ ] All keybinds (Ctrl+b, etc.)
- [ ] Window resize

## Out of Scope (See Related Features)

- Mobile-specific layout (FEAT-114)
- Voice input (FEAT-115)
- Authentication/TLS (FEAT-116)
- WebGL rendering optimization
- Text-to-speech

## Prior Art

- **ttyd**: C-based, minimal - good reference for PTY bridging
- **wetty**: Node.js - good reference for xterm.js integration
- **gotty**: Go - good reference for WebSocket protocol
