# FEAT-114: Web Mobile Responsive Layout

**Priority**: P2
**Component**: fugue-web
**Effort**: Small
**Status**: new
**Depends On**: FEAT-113

## Summary

Add mobile-optimized layout to the fugue web interface. Automatically detect mobile viewports and switch to a single-pane fullscreen mode with touch-friendly navigation controls.

## Related Features

- **FEAT-113**: Web Interface Core (prerequisite)
- **FEAT-115**: Voice Input (builds on this)

## Motivation

- **Mobile Monitoring**: Check on Claude agents from phone/tablet
- **Touch Interaction**: Provide input without physical keyboard
- **Readability**: Terminal text readable on small screens without zooming

## Mobile Layout Design

### Single Pane Mode

On mobile viewports, show one pane at a time instead of the full TUI layout:

```
┌─────────────────────────┐
│ ● worker-1 [working]    │  ← Pane indicator/selector
├─────────────────────────┤
│                         │
│  Terminal output here   │
│  (single pane only)     │
│                         │
│                         │
├─────────────────────────┤
│  ◄  │  ●●○○  │  ►  │ ⌨️ │  ← Navigation controls
└─────────────────────────┘
```

### Responsive Breakpoints

```css
/* Desktop - full TUI */
@media (min-width: 768px) {
  .terminal-container { /* normal xterm rendering */ }
  .mobile-controls { display: none; }
}

/* Tablet - compact TUI, larger touch targets */
@media (min-width: 600px) and (max-width: 767px) {
  .terminal-container { /* slightly larger font */ }
  .mobile-controls { display: none; }
}

/* Mobile - single pane mode */
@media (max-width: 599px) {
  .terminal-container { /* fullscreen single pane */ }
  .mobile-controls { display: flex; }
}
```

### Auto-Detection

```javascript
function isMobileViewport() {
  return window.innerWidth < 600;
}

// Also check for touch capability
function isTouchDevice() {
  return 'ontouchstart' in window || navigator.maxTouchPoints > 0;
}

// Switch modes on resize
window.addEventListener('resize', () => {
  if (isMobileViewport()) {
    enableSinglePaneMode();
  } else {
    enableFullTuiMode();
  }
});
```

## Implementation

### Frontend Changes (`fugue-web/static/`)

**New file**: `mobile.js`
```javascript
// Mobile layout controller
class MobileLayout {
  constructor(term, ws) {
    this.term = term;
    this.ws = ws;
    this.panes = [];
    this.activePaneIndex = 0;
  }

  enable() {
    document.body.classList.add('mobile-mode');
    this.showMobileControls();
    this.requestPaneList();
  }

  disable() {
    document.body.classList.remove('mobile-mode');
    this.hideMobileControls();
  }

  switchPane(direction) {
    // direction: -1 (prev) or +1 (next)
    const newIndex = this.activePaneIndex + direction;
    if (newIndex >= 0 && newIndex < this.panes.length) {
      this.activePaneIndex = newIndex;
      this.ws.send(JSON.stringify({
        type: 'pane_switch',
        pane_id: this.panes[newIndex].id
      }));
    }
  }
}
```

**New file**: `mobile.css`
```css
.mobile-mode .terminal-container {
  position: fixed;
  top: 40px;      /* pane indicator */
  bottom: 50px;   /* navigation controls */
  left: 0;
  right: 0;
}

.mobile-controls {
  position: fixed;
  bottom: 0;
  left: 0;
  right: 0;
  height: 50px;
  display: flex;
  justify-content: space-around;
  align-items: center;
  background: #1a1a1a;
  border-top: 1px solid #333;
}

.mobile-controls button {
  min-width: 48px;
  min-height: 48px;  /* Touch-friendly size */
  font-size: 20px;
  background: #333;
  border: none;
  border-radius: 8px;
  color: white;
}

.pane-indicator {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  height: 40px;
  background: #1a1a1a;
  display: flex;
  align-items: center;
  padding: 0 12px;
  font-family: monospace;
  color: #0f0;
}
```

### WebSocket Protocol Extension

**New message type**:
```typescript
// Client → Server
{ type: 'pane_switch', pane_id: string }
{ type: 'get_panes' }

// Server → Client
{ type: 'pane_list', panes: Array<{id: string, name: string, status: string}> }
{ type: 'pane_switched', pane_id: string }
```

### Server Changes (`fugue-web/src/`)

**Add to** `ws.rs`:
```rust
// Handle pane_switch message
async fn handle_pane_switch(&mut self, pane_id: Uuid) -> Result<()> {
    // For mobile mode, we redirect the PTY bridge to a specific pane
    // This may involve sending a focus command to fugue-client
    // or maintaining separate PTY connections per pane
}
```

## Navigation Controls

### Button Controls
- `◄` Previous pane
- `►` Next pane
- `●●○○` Pane position indicator (dots)
- `⌨️` Toggle software keyboard

### Gesture Support (Optional Enhancement)
- Swipe left/right - Switch panes
- Two-finger tap - Toggle keyboard

```javascript
// Swipe detection
let touchStartX = 0;
document.addEventListener('touchstart', (e) => {
  touchStartX = e.touches[0].clientX;
});

document.addEventListener('touchend', (e) => {
  const deltaX = e.changedTouches[0].clientX - touchStartX;
  if (Math.abs(deltaX) > 50) {  // Minimum swipe distance
    if (deltaX > 0) mobileLayout.switchPane(-1);  // Swipe right = prev
    else mobileLayout.switchPane(1);              // Swipe left = next
  }
});
```

## Terminal Adaptations

### Font Size
```javascript
// Larger font for mobile readability
if (isMobileViewport()) {
  term.options.fontSize = 16;  // vs 14 on desktop
}
```

### Scrollback
- Enable momentum scrolling on touch
- Pull-to-refresh gesture for reconnect (optional)

### Keyboard Handling
```javascript
// Toggle software keyboard
function toggleKeyboard() {
  const input = document.getElementById('hidden-input');
  if (document.activeElement === input) {
    input.blur();
  } else {
    input.focus();
  }
}
```

## Configuration

**Extend** `~/.fugue/config.toml`:
```toml
[web.mobile]
enabled = true                # Enable mobile layout
auto_detect = true            # Auto-switch based on viewport
single_pane_mode = true       # Use single pane on mobile
font_size = 16                # Mobile font size
gesture_navigation = true     # Enable swipe gestures
```

## Acceptance Criteria

- [ ] Mobile viewport detected automatically
- [ ] Single pane mode activates on small screens
- [ ] Pane navigation controls visible and functional
- [ ] Previous/next pane buttons work
- [ ] Pane indicator shows current pane name/status
- [ ] Font size larger on mobile
- [ ] Touch scrolling works smoothly
- [ ] Software keyboard toggle works
- [ ] Layout switches correctly on orientation change
- [ ] Desktop mode still works when resizing to larger viewport

## Testing

### Manual Testing
- [ ] iOS Safari (iPhone)
- [ ] iOS Safari (iPad)
- [ ] Android Chrome (phone)
- [ ] Android Chrome (tablet)
- [ ] Desktop browser resized to mobile width
- [ ] Orientation change (portrait ↔ landscape)

### Test Scenarios
1. Load page on mobile - should auto-detect
2. Switch between panes using buttons
3. Swipe to switch panes (if gestures enabled)
4. Toggle keyboard and type input
5. Scroll terminal output
6. Rotate device - layout should adapt

## Out of Scope

- Voice input (FEAT-115)
- Text-to-speech
- Pinch-to-zoom (use font size setting instead)
- Native mobile app
