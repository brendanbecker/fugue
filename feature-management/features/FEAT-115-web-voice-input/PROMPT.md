# FEAT-115: Web Voice Input

**Priority**: P3
**Component**: fugue-web
**Effort**: Small
**Status**: new
**Depends On**: FEAT-113 (FEAT-114 recommended)

## Summary

Add voice-to-text input for the fugue web interface using the Web Speech API. Users can speak commands naturally, which are transcribed and sent to the active terminal pane. Primary use case is mobile interaction with Claude agents.

## Related Features

- **FEAT-113**: Web Interface Core (prerequisite)
- **FEAT-114**: Mobile Layout (recommended - provides mobile UI context)

## Motivation

- **Hands-Free Input**: Interact with Claude while doing other tasks
- **Mobile Convenience**: Faster than typing on phone keyboard
- **Accessibility**: Alternative input method for users who prefer voice
- **Natural Interaction**: Speak to Claude naturally

## Web Speech API

### Browser Support

The Web Speech API is supported in:
- Chrome (desktop & Android) - Full support
- Safari (desktop & iOS) - Full support
- Edge - Full support
- Firefox - Limited/behind flag

### Basic Implementation

```javascript
// Check for support
const SpeechRecognition = window.SpeechRecognition || window.webkitSpeechRecognition;

if (!SpeechRecognition) {
  console.warn('Speech recognition not supported');
  hideVoiceButton();
}

// Initialize
const recognition = new SpeechRecognition();
recognition.continuous = false;      // Stop after one phrase
recognition.interimResults = true;   // Show partial results
recognition.lang = 'en-US';          // Language

// Events
recognition.onresult = (event) => {
  const transcript = event.results[0][0].transcript;
  const isFinal = event.results[0].isFinal;

  if (isFinal) {
    sendToTerminal(transcript);
  } else {
    showInterimTranscript(transcript);
  }
};

recognition.onerror = (event) => {
  console.error('Speech error:', event.error);
  showError(event.error);
};

recognition.onend = () => {
  updateUI('idle');
};
```

## User Interface

### Voice Button (Floating Action Button)

```
┌─────────────────────────┐
│                         │
│  Terminal output...     │
│                         │
│                         │
│                    🎤   │  ← FAB in bottom-right
│                   ╱  ╲  │
├─────────────────────────┤
│  ◄  │  ●●○○  │  ►  │ ⌨️ │
└─────────────────────────┘
```

### Voice Input States

```
┌──────────────────────────────────────┐
│ State: IDLE                          │
│ Button: 🎤 (gray)                    │
│ Action: Tap to start listening       │
└──────────────────────────────────────┘
           │
           ▼ tap
┌──────────────────────────────────────┐
│ State: LISTENING                     │
│ Button: 🎤 (red, pulsing)            │
│ Visual: Waveform/pulse animation     │
│ Text: "Listening..."                 │
│ Action: Tap to stop, or wait         │
└──────────────────────────────────────┘
           │
           ▼ speech detected
┌──────────────────────────────────────┐
│ State: TRANSCRIBING                  │
│ Button: 🎤 (orange)                  │
│ Text: "check on workers..." (live)   │
│ Action: Shows interim results        │
└──────────────────────────────────────┘
           │
           ▼ final result
┌──────────────────────────────────────┐
│ State: CONFIRMING (optional mode)    │
│ Text: "check on workers"             │
│ Buttons: [Send] [Edit] [Cancel]      │
│ Action: User confirms or edits       │
└──────────────────────────────────────┘
           │
           ▼ send (or auto-send)
┌──────────────────────────────────────┐
│ State: SENT                          │
│ Text: "Sent!" (brief flash)          │
│ Then returns to IDLE                 │
└──────────────────────────────────────┘
```

## Implementation

### Voice Controller (`voice.js`)

```javascript
class VoiceInput {
  constructor(options = {}) {
    this.recognition = new (window.SpeechRecognition || window.webkitSpeechRecognition)();
    this.recognition.continuous = false;
    this.recognition.interimResults = true;
    this.recognition.lang = options.lang || 'en-US';

    this.autoSubmit = options.autoSubmit ?? true;
    this.onTranscript = options.onTranscript || (() => {});
    this.onStateChange = options.onStateChange || (() => {});

    this.state = 'idle';
    this.setupEvents();
  }

  setupEvents() {
    this.recognition.onstart = () => {
      this.setState('listening');
    };

    this.recognition.onresult = (event) => {
      const result = event.results[0];
      const transcript = result[0].transcript;

      if (result.isFinal) {
        this.setState('complete');
        if (this.autoSubmit) {
          this.onTranscript(transcript, true);
          this.setState('idle');
        } else {
          this.pendingTranscript = transcript;
          this.setState('confirming');
        }
      } else {
        this.setState('transcribing');
        this.onTranscript(transcript, false);
      }
    };

    this.recognition.onerror = (event) => {
      console.error('Voice error:', event.error);
      this.setState('error', event.error);
      setTimeout(() => this.setState('idle'), 2000);
    };

    this.recognition.onend = () => {
      if (this.state === 'listening') {
        // No speech detected
        this.setState('idle');
      }
    };
  }

  start() {
    if (this.state === 'idle') {
      this.recognition.start();
    }
  }

  stop() {
    this.recognition.stop();
  }

  confirm() {
    if (this.state === 'confirming' && this.pendingTranscript) {
      this.onTranscript(this.pendingTranscript, true);
      this.pendingTranscript = null;
      this.setState('idle');
    }
  }

  cancel() {
    this.recognition.stop();
    this.pendingTranscript = null;
    this.setState('idle');
  }

  setState(state, data) {
    this.state = state;
    this.onStateChange(state, data);
  }
}
```

### Voice UI (`voice-ui.js`)

```javascript
class VoiceUI {
  constructor(voiceInput, container) {
    this.voice = voiceInput;
    this.container = container;

    this.voice.onStateChange = (state, data) => this.updateUI(state, data);
    this.createElements();
  }

  createElements() {
    // FAB button
    this.fab = document.createElement('button');
    this.fab.className = 'voice-fab';
    this.fab.innerHTML = '🎤';
    this.fab.onclick = () => this.toggle();

    // Transcript overlay
    this.overlay = document.createElement('div');
    this.overlay.className = 'voice-overlay hidden';
    this.overlay.innerHTML = `
      <div class="voice-transcript"></div>
      <div class="voice-actions hidden">
        <button class="voice-send">Send</button>
        <button class="voice-edit">Edit</button>
        <button class="voice-cancel">Cancel</button>
      </div>
    `;

    this.container.appendChild(this.fab);
    this.container.appendChild(this.overlay);

    // Wire up confirmation buttons
    this.overlay.querySelector('.voice-send').onclick = () => this.voice.confirm();
    this.overlay.querySelector('.voice-cancel').onclick = () => this.voice.cancel();
  }

  toggle() {
    if (this.voice.state === 'idle') {
      this.voice.start();
    } else {
      this.voice.stop();
    }
  }

  updateUI(state, data) {
    this.fab.className = `voice-fab voice-${state}`;

    switch (state) {
      case 'idle':
        this.overlay.classList.add('hidden');
        break;
      case 'listening':
        this.overlay.classList.remove('hidden');
        this.overlay.querySelector('.voice-transcript').textContent = 'Listening...';
        break;
      case 'transcribing':
        this.overlay.querySelector('.voice-transcript').textContent = data || '';
        break;
      case 'confirming':
        this.overlay.querySelector('.voice-actions').classList.remove('hidden');
        break;
      case 'error':
        this.overlay.querySelector('.voice-transcript').textContent = `Error: ${data}`;
        break;
    }
  }
}
```

### Voice Styles (`voice.css`)

```css
.voice-fab {
  position: fixed;
  bottom: 70px;  /* Above mobile controls */
  right: 16px;
  width: 56px;
  height: 56px;
  border-radius: 50%;
  border: none;
  font-size: 24px;
  cursor: pointer;
  box-shadow: 0 2px 8px rgba(0,0,0,0.3);
  transition: all 0.2s;
}

.voice-fab.voice-idle {
  background: #333;
}

.voice-fab.voice-listening {
  background: #c00;
  animation: pulse 1s infinite;
}

.voice-fab.voice-transcribing {
  background: #f80;
}

@keyframes pulse {
  0%, 100% { transform: scale(1); }
  50% { transform: scale(1.1); }
}

.voice-overlay {
  position: fixed;
  bottom: 130px;
  left: 16px;
  right: 16px;
  background: rgba(0,0,0,0.9);
  border-radius: 12px;
  padding: 16px;
  color: white;
  font-family: system-ui;
}

.voice-overlay.hidden {
  display: none;
}

.voice-transcript {
  font-size: 18px;
  margin-bottom: 12px;
}

.voice-actions {
  display: flex;
  gap: 8px;
}

.voice-actions.hidden {
  display: none;
}

.voice-actions button {
  flex: 1;
  padding: 12px;
  border: none;
  border-radius: 8px;
  font-size: 16px;
}

.voice-send { background: #0a0; color: white; }
.voice-edit { background: #555; color: white; }
.voice-cancel { background: #a00; color: white; }
```

## Integration with Terminal

```javascript
// In app.js
const voiceInput = new VoiceInput({
  autoSubmit: config.voice.autoSubmit,
  lang: config.voice.lang,
  onTranscript: (text, isFinal) => {
    if (isFinal) {
      // Send to terminal with newline (submit)
      ws.send(JSON.stringify({
        type: 'input',
        data: text + '\n'
      }));
    }
  }
});

const voiceUI = new VoiceUI(voiceInput, document.body);
```

## Configuration

**Extend** `~/.fugue/config.toml`:
```toml
[web.voice]
enabled = true           # Enable voice input
auto_submit = true       # Send immediately on recognition complete
lang = "en-US"           # Recognition language
show_on_desktop = false  # Only show on mobile by default
```

## Error Handling

### Common Errors

| Error | Cause | User Message |
|-------|-------|--------------|
| `not-allowed` | Microphone permission denied | "Please allow microphone access" |
| `no-speech` | No speech detected | "No speech detected. Tap to try again." |
| `network` | Network error (some browsers) | "Network error. Check connection." |
| `aborted` | User cancelled | (silent, return to idle) |

### Fallback

```javascript
if (!SpeechRecognition) {
  // Hide voice button entirely
  // Or show "Voice not supported in this browser"
}
```

## Acceptance Criteria

- [ ] Voice button visible on mobile (configurable for desktop)
- [ ] Tap to start listening
- [ ] Visual feedback while listening (pulsing button)
- [ ] Interim results shown as user speaks
- [ ] Final transcript sent to terminal
- [ ] Tap again to cancel
- [ ] Error states handled gracefully
- [ ] Permission denied shows helpful message
- [ ] No speech timeout returns to idle
- [ ] Works in Chrome, Safari, Edge
- [ ] Graceful degradation in Firefox (button hidden)
- [ ] Optional confirmation mode before send

## Testing

### Manual Testing
- [ ] iOS Safari - voice recognition
- [ ] Android Chrome - voice recognition
- [ ] Desktop Chrome - voice recognition
- [ ] Permission flow (allow/deny)
- [ ] No speech timeout
- [ ] Long utterance
- [ ] Background noise handling
- [ ] Cancel mid-recognition

### Test Phrases
1. "check on workers" → sends "check on workers\n"
2. "list all panes" → sends "list all panes\n"
3. "yes" (for confirmations) → sends "yes\n"
4. Long command with punctuation

## Privacy Considerations

- Speech is processed by browser's speech service (Google for Chrome, Apple for Safari)
- Audio is not stored by fugue
- Consider noting this in UI: "Voice processed by [browser vendor]"
- No server-side speech processing in fugue

## Future Enhancements (Out of Scope)

- Text-to-speech for Claude responses
- Custom wake word ("Hey fugue")
- Multi-language support
- Offline speech recognition (when browsers support it)
- Voice commands for navigation ("next pane", "scroll up")
