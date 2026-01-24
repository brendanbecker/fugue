# FEAT-055: tmux Keybinding Parity in TUI

**Priority**: P2
**Component**: fugue-client (TUI)
**Type**: enhancement
**Estimated Effort**: medium
**Business Value**: high

## Overview

Implement tmux-compatible keybindings in the fugue TUI client so users familiar with tmux can navigate windows and panes using the same keyboard shortcuts.

### Context

- Users expect `Ctrl-b` prefix followed by navigation keys to work like tmux
- Window switching via `Ctrl-b n/p/w` does not currently work
- This is independent of MCP tools - it's about TUI keyboard handling

## Keybindings to Implement

### Window Navigation
| Keybinding | Action |
|------------|--------|
| `Ctrl-b n` | Next window |
| `Ctrl-b p` | Previous window |
| `Ctrl-b w` | List windows (interactive picker) |
| `Ctrl-b 0-9` | Switch to window by number |
| `Ctrl-b l` | Last (previously active) window |

### Pane Navigation
| Keybinding | Action |
|------------|--------|
| `Ctrl-b o` | Next pane |
| `Ctrl-b ;` | Last (previously active) pane |
| `Ctrl-b Up/Down/Left/Right` | Move to pane in direction |
| `Ctrl-b q` | Show pane numbers (briefly) |

### Pane Management
| Keybinding | Action |
|------------|--------|
| `Ctrl-b %` | Split vertical (side-by-side) |
| `Ctrl-b "` | Split horizontal (stacked) |
| `Ctrl-b x` | Kill current pane (with confirmation) |
| `Ctrl-b z` | Toggle pane zoom |

### Window Management
| Keybinding | Action |
|------------|--------|
| `Ctrl-b c` | Create new window |
| `Ctrl-b &` | Kill current window (with confirmation) |
| `Ctrl-b ,` | Rename current window |

### Session
| Keybinding | Action |
|------------|--------|
| `Ctrl-b d` | Detach from session |
| `Ctrl-b s` | List sessions (interactive picker) |
| `Ctrl-b $` | Rename current session |

## Implementation Notes

- The TUI already handles some keybindings - this extends that system
- Prefix key (`Ctrl-b`) should be configurable (like tmux)
- Some keybindings may already be partially implemented but not working correctly

## Acceptance Criteria

- [ ] `Ctrl-b n` switches to next window
- [ ] `Ctrl-b p` switches to previous window
- [ ] `Ctrl-b w` shows window picker
- [ ] `Ctrl-b 0-9` switches to numbered window
- [ ] `Ctrl-b arrow` navigates between panes
- [ ] `Ctrl-b %` and `Ctrl-b "` split panes
- [ ] `Ctrl-b d` detaches cleanly

## Related

- FEAT-049: tmux-compatible CLI wrapper (command-line, not TUI)
- BUG-026: Focus management broken (MCP layer - separate issue)
