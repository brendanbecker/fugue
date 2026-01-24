# FEAT-101: Codex CLI Agent Detection

## Goal

Extend the agent detection system to recognize OpenAI Codex CLI alongside Claude and Gemini. Implement `CodexAgentDetector` following the existing `AgentDetector` trait pattern with presence detection, activity state tracking, and debounced state broadcasts.

## Background

Codex CLI is OpenAI's terminal-based coding agent. Like Claude Code and Gemini CLI, it runs in a TUI and has distinct visual patterns for presence and activity states. Adding detection enables fugue to provide the same orchestration awareness for Codex sessions.

## Detection Patterns

### Presence Detection (Strong - 100% confidence)

| Pattern | Context |
|---------|---------|
| `OpenAI Codex` | Welcome box header |
| `(v0.` or `(v1.` | Version string in welcome |
| `gpt-5` or `gpt-4` followed by `-codex` | Model indicator |
| `/model to change` | Slash command hint |
| `codex-agent` | MCP server name |

### Presence Detection (Medium - 70% confidence)

| Pattern | Context |
|---------|---------|
| `Codex` alone | Generic reference |
| `context left` with percentage | Context indicator |

### Activity State Detection

| State | Pattern |
|-------|---------|
| **Processing** | `•` bullet with "Working", "Preparing", or other activity text |
| **Processing** | `(Xs • esc to interrupt)` timer pattern |
| **ToolUse** | Activity text contains "tool", "executing", "running" |
| **Generating** | Activity text contains "writing", "generating", "creating" |
| **Idle** | `›` prompt character at start of line |
| **AwaitingConfirmation** | `[Y/n]`, `[y/N]`, "Continue?", "confirm" |

### Visual Markers

- `•` (filled bullet) - Active operation in progress
- `◦` (hollow bullet) - Transitional state
- `›` - Input prompt (idle)

## Implementation

### File Structure

```
fugue-server/src/agents/
├── mod.rs          # Add `pub mod codex;` and register in with_defaults()
└── codex/
    └── mod.rs      # CodexAgentDetector implementation
```

### CodexAgentDetector Fields

```rust
pub struct CodexAgentDetector {
    is_active: bool,
    confidence: u8,
    current_activity: AgentActivity,
    last_broadcast_activity: AgentActivity,
    last_state_broadcast: Option<Instant>,
    broadcast_debounce: Duration,
    model: Option<String>,      // e.g., "gpt-5.2-codex medium"
    version: Option<String>,    // e.g., "0.87.0"
}
```

### Metadata Extraction

- **model**: Extract from "model: gpt-5.2-codex medium" or similar
- **version**: Extract from "(v0.87.0)" pattern
- **context_percent**: Extract from "100% context left" if visible

### Registry Integration

Update `DetectorRegistry::with_defaults()` in `fugue-server/src/agents/mod.rs`:

```rust
pub fn with_defaults() -> Self {
    let mut registry = Self::new();
    registry.register(Box::new(claude::ClaudeAgentDetector::new()));
    registry.register(Box::new(gemini::GeminiAgentDetector::new()));
    registry.register(Box::new(codex::CodexAgentDetector::new()));  // Add this
    registry
}
```

## Testing Requirements

1. **Presence detection tests**
   - Detect "OpenAI Codex" welcome message
   - Detect model indicator pattern
   - Detect version pattern
   - Lower confidence for "Codex" alone

2. **Activity detection tests**
   - Detect Processing from "Working" with bullet
   - Detect Processing from timer pattern
   - Detect ToolUse when activity mentions tools
   - Detect Idle from `›` prompt
   - Detect AwaitingConfirmation from `[Y/n]`

3. **Metadata extraction tests**
   - Extract model name
   - Extract version

4. **Debounce tests**
   - Initial detection not debounced
   - Rapid state changes debounced
   - State changes after debounce window pass through

5. **Registry integration tests**
   - Registry detects Codex sessions
   - `mark_as_active("codex")` works

## Acceptance Criteria

- [ ] `CodexAgentDetector` implements `AgentDetector` trait
- [ ] Presence detection works for all strong patterns
- [ ] Activity state detection matches Codex UI behavior
- [ ] Debouncing prevents TUI flicker from rapid updates
- [ ] Registered in `DetectorRegistry::with_defaults()`
- [ ] All tests pass
- [ ] `cargo clippy` passes
