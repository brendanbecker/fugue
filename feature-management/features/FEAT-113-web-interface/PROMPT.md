# FEAT-113: Web Interface with Mobile Support

**Priority**: P2
**Component**: ccmux-web (new crate)
**Effort**: Large
**Status**: new

## Summary

Add a web interface to ccmux that allows remote browser-based access to the terminal multiplexer. The implementation will expose the existing TUI through a web frontend, with special optimizations for mobile devices including voice-to-text input.

## Motivation

- **Remote Access**: Users need to access ccmux sessions from anywhere without SSH
- **Mobile Monitoring**: Ability to monitor Claude agents and provide input from mobile devices
- **Voice Input**: Natural voice-to-text interaction with Claude on mobile devices
- **Unified Interface**: Same TUI experience across desktop SSH, desktop web, and mobile web

## Architecture Decision: TUI Streaming

**Chosen Approach**: Stream the existing ratatui/crossterm TUI through a web interface using xterm.js

**Rationale**:
- Single codebase - no separate web UI to maintain
- All existing features work immediately (keybinds, mouse support, visual mode)
- Proven pattern (ttyd, wetty, gotty use this approach)
- Copy/paste already uses Shift+drag pattern compatible with web terminals

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

## Implementation Plan

### New Crate: `ccmux-web`

**Dependencies**:
- `axum` - WebSocket server (matches existing ccmux patterns)
- `tokio-tungstenite` - WebSocket handling
- `portable-pty` - PTY management for spawning ccmux-client
- `tokio` - Async runtime

**Core Components**:
1. **WebSocket Server**: Handles browser connections
2. **PTY Bridge**: Spawns ccmux-client in PTY, bridges I/O to WebSocket
3. **Static File Server**: Serves HTML/CSS/JS assets
4. **Session Management**: Maps WebSocket connections to PTY instances

### Frontend (Static Assets)

**Files**:
- `index.html` - Main page
- `app.js` - xterm.js initialization and WebSocket handling
- `mobile.js` - Mobile-specific features (voice input, layout adaptation)
- `styles.css` - Responsive styling

**Key Libraries**:
- `xterm.js` - Terminal emulator
- `xterm-addon-fit` - Responsive terminal sizing
- `xterm-addon-webgl` - Optional GPU acceleration

**xterm.js Configuration**:
```javascript
const term = new Terminal({
  allowProposedApi: true,
  scrollback: 1000,
  // Shift+drag for text selection (matches native TUI)
  // Normal mouse events forwarded to ccmux
});
```

## Desktop Web Features

### Core Functionality
- Full TUI rendering via xterm.js
- WebSocket-based real-time communication
- Mouse support (clicking to select panes works)
- Copy/paste with Shift+drag selection
- All existing keybinds work (`Ctrl+b`, etc.)
- Scrollback navigation
- Visual mode for text selection

### User Experience
- Auto-connect to ccmux-server on page load
- Reconnection logic for dropped WebSocket connections
- Connection status indicator
- Responsive terminal sizing (fits browser window)

## Mobile Web Features

### Layout Adaptation

**Auto-detection**:
- Detect mobile viewport via media queries or user agent
- Switch to mobile-optimized layout automatically

**Mobile Layout**:
- **Single pane fullscreen** - one pane visible at a time
- **Pane switcher** - bottom navigation or swipe gestures
- **Larger text** - readable without zooming
- **Touch-friendly controls** - buttons instead of keyboard shortcuts

### Voice-to-Text Input

**Implementation** (Web Speech API):
```javascript
const recognition = new webkitSpeechRecognition();
recognition.continuous = false;
recognition.interimResults = false;

recognition.onresult = (event) => {
  const transcript = event.results[0][0].transcript;
  // Send to active pane via WebSocket
  sendInput(transcript + '\n');
};
```

**UX Flow**:
1. User taps microphone button (floating action button)
2. Visual feedback while listening
3. Speak command naturally
4. Transcript appears, auto-sends to terminal
5. Can edit before sending (optional mode)

**Features**:
- Hold-to-talk or tap-to-toggle modes
- Visual waveform/listening indicator
- "Cancel" gesture for mistakes
- Auto-submit on pause (configurable)
- Manual edit before submit option

### Mobile-Specific UI

**Controls**:
- Microphone - Voice Input (main input method)
- Arrow buttons - Navigate between panes
- Keyboard icon - Toggle software keyboard
- Settings icon - Quick settings (font size, theme)

**Gestures**:
- Swipe left/right - Switch panes
- Two-finger tap - Toggle keyboard
- Long press - Context menu (copy, paste, etc.)

**Reading Optimizations**:
- Auto-scroll to bottom on new output
- Highlight new content briefly
- Optional text-to-speech for Claude responses
- Pinch-to-zoom support
- High contrast mode for outdoor readability

## Technical Implementation Details

### WebSocket Protocol

**Message Types**:
- `input` - User input (keyboard or voice) → PTY
- `output` - PTY output → Terminal display
- `resize` - Terminal size change
- `pane_switch` - Mobile pane navigation

**Sample Message Format**:
```json
{
  "type": "input",
  "data": "ls -la\n"
}

{
  "type": "output",
  "data": "\u001b[32mfile.txt\u001b[0m\n"
}

{
  "type": "resize",
  "cols": 80,
  "rows": 24
}
```

### PTY Management

**Desktop Flow**:
1. Browser connects via WebSocket
2. Server spawns `ccmux-client` in new PTY
3. PTY I/O bridged to WebSocket bidirectionally
4. On disconnect, PTY kept alive briefly for reconnection
5. After timeout, clean shutdown

**Mobile Flow** (single pane mode):
1. Browser connects, requests mobile mode
2. Server spawns ccmux-client with specific session
3. Server tracks active pane for this connection
4. Pane switch commands change active PTY routing
5. Voice input routed to active pane only

### Responsive Design

**Breakpoints**:
- Desktop: `>= 768px` - Full TUI layout
- Tablet: `600-767px` - Compact TUI, larger touch targets
- Mobile: `< 600px` - Single pane mode, voice controls

**CSS Strategy**:
```css
/* Desktop - full TUI */
@media (min-width: 768px) {
  .terminal-container { /* normal layout */ }
  .mobile-controls { display: none; }
}

/* Mobile - single pane */
@media (max-width: 767px) {
  .terminal-container { /* fullscreen single pane */ }
  .mobile-controls { display: flex; }
  .voice-button { /* prominent FAB */ }
}
```

## Configuration

**New config options** (`~/.ccmux/config.toml`):
```toml
[web]
enabled = true
host = "0.0.0.0"
port = 8080
tls_cert = ""  # Optional HTTPS
tls_key = ""

[web.mobile]
voice_input_enabled = true
single_pane_mode = true  # Force single pane on mobile
auto_detect_mobile = true
```

## Security Considerations

### Authentication
- **Phase 1**: No auth (localhost only, trust network layer)
- **Phase 2**: Optional token-based auth
- **Phase 3**: Integration with existing auth systems

### HTTPS/TLS
- Support for TLS certificates
- Recommend reverse proxy (nginx/caddy) for production
- WebSocket over WSS when using HTTPS

### Origin Validation
- CORS headers to restrict allowed origins
- WebSocket origin checking
- CSRF token for session initiation

## Testing Strategy

### Unit Tests
- WebSocket message handling
- PTY spawning and cleanup
- Voice transcript processing
- Pane switching logic

### Integration Tests
- End-to-end browser → WebSocket → PTY → ccmux-server
- Mobile layout switching
- Voice input flow
- Reconnection logic

### Manual Testing Checklist
- [ ] Desktop browser (Chrome, Firefox, Safari)
- [ ] Mobile browser (iOS Safari, Android Chrome)
- [ ] Voice input on mobile
- [ ] Copy/paste with Shift+drag
- [ ] Mouse pane selection
- [ ] Reconnection after network drop
- [ ] Multiple simultaneous connections

## Phased Implementation

### Phase 1: MVP (Desktop Web)
- Basic xterm.js rendering
- WebSocket PTY bridge
- Static file serving
- Copy/paste support
- Mouse events forwarded to TUI

### Phase 2: Mobile Foundation
- Responsive layout detection
- Single pane mode for mobile
- Touch-friendly navigation
- Larger text on small screens

### Phase 3: Voice Input
- Web Speech API integration
- Voice button UI
- Transcript handling
- Error/fallback handling

### Phase 4: Polish
- Text-to-speech for Claude responses (optional)
- Advanced gestures (swipe, pinch-zoom)
- Offline detection and graceful degradation
- Performance optimizations (WebGL rendering)

## Acceptance Criteria

### Phase 1 (MVP)
- [ ] New `ccmux-web` crate builds and runs
- [ ] Browser can connect and see ccmux TUI via xterm.js
- [ ] Keyboard input works (all keybinds functional)
- [ ] Mouse events forwarded correctly (pane selection works)
- [ ] Copy/paste with Shift+drag works
- [ ] Terminal resizes with browser window
- [ ] Reconnection on WebSocket drop

### Phase 2 (Mobile)
- [ ] Mobile viewport detected automatically
- [ ] Single pane mode on mobile
- [ ] Pane navigation controls work
- [ ] Text readable without zooming
- [ ] Touch targets appropriately sized

### Phase 3 (Voice)
- [ ] Voice input button visible on mobile
- [ ] Speech recognition captures input
- [ ] Transcript sent to active pane
- [ ] Visual feedback during listening
- [ ] Graceful fallback if speech API unavailable

## Open Questions

1. **Authentication**: When/how to add user authentication?
2. **Multi-user**: Should one server support multiple user sessions simultaneously?
3. **Port conflict**: How to handle port already in use?
4. **PTY cleanup**: Timeout duration for abandoned WebSocket connections?
5. **Mobile keyboard**: When should software keyboard auto-show vs. stay hidden?
6. **Voice language**: Multi-language support for speech recognition?

## Related Work / Prior Art

**Existing Projects**:
- **ttyd**: Shares terminal over web, minimal UI
- **wetty**: Web-based terminal emulator (Node.js)
- **gotty**: Terminal sharing (Go)
- **Butterfly**: Web terminal with auth (Python)

**Differentiators for ccmux-web**:
- Native integration with ccmux's TUI and MCP features
- Mobile-first voice input (not found in other terminal web UIs)
- Agent state awareness could be exposed in web UI
- Pane/window/session orchestration via web controls

## Future Enhancements (Out of Scope)

- Native mobile apps (iOS/Android) using same backend
- Collaborative sessions (multiple users in same session)
- Session recording/playback via web
- Web-based configuration editor
- Browser notifications for agent state changes
- Integration with MCP tools via web UI (click to run tools)
- File upload/download through web interface
- Split terminal mode on larger tablets
