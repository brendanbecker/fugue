# FEAT-037: tmux-Compatible Prefix Keybinds

**Priority**: P1
**Component**: ccmux-client
**Type**: enhancement
**Estimated Effort**: small (1-2 hours)
**Business Value**: high
**Status**: completed

## Overview

Align ccmux prefix keybindings with tmux defaults for muscle-memory compatibility. Users familiar with tmux should be able to use ccmux without relearning keybinds.

## Problem Statement

The original ccmux keybinds diverged from tmux conventions:
- `c` created a pane (tmux: creates window)
- `n/p` navigated panes (tmux: navigates windows)
- No keybinds for window selection by number (0-9)
- No keybind for closing windows (&)

This caused confusion for tmux users and broke muscle memory.

## Solution

Realign all prefix keybinds to match tmux defaults:

### Window Management (tmux defaults)
| Key | Action | tmux |
|-----|--------|------|
| `c` | CreateWindow | same |
| `&` | CloseWindow | same |
| `n` | NextWindow | same |
| `p` | PreviousWindow | same |
| `w` | ListWindows | same |
| `0-9` | SelectWindow(n) | same |

### Pane Management (tmux defaults)
| Key | Action | tmux |
|-----|--------|------|
| `x` | ClosePane | same |
| `%` | SplitVertical | same |
| `"` | SplitHorizontal | same |
| `o` | NextPane (cycle) | same |
| `;` | PreviousPane | same |
| `←↓↑→` | Directional navigation | same |

### Extensions (vim-style, common in tmux configs)
| Key | Action | Notes |
|-----|--------|-------|
| `h/j/k/l` | Directional pane navigation | Common tmux config extension |

### Session/Mode Commands (unchanged)
| Key | Action |
|-----|--------|
| `d` | Detach |
| `s` | ListSessions (session picker) |
| `:` | Command mode |
| `[` | Copy/scroll mode |
| `z` | Zoom pane |
| `?` | Help |

## Implementation

### Files Modified
- `ccmux-client/src/input/mod.rs` - Updated `handle_prefix_key()` match block

### Key Changes
1. Changed `c` from CreatePane to CreateWindow
2. Changed `n` from NextPane to NextWindow
3. Changed `p` from PreviousPane to PreviousWindow
4. Added `&` for CloseWindow
5. Added `0-9` for SelectWindow by index
6. Added `o` for NextPane (tmux pane cycling)
7. Added `;` for PreviousPane (tmux last-pane)
8. Separated arrow keys from vim keys for clarity

## Implementation Tasks

### Section 1: Window Keybind Changes
- [x] Change `c` to CreateWindow
- [x] Change `n` to NextWindow
- [x] Change `p` to PreviousWindow
- [x] Add `&` for CloseWindow
- [x] Add `0-9` for SelectWindow

### Section 2: Pane Keybind Additions
- [x] Add `o` for NextPane (cycle)
- [x] Add `;` for PreviousPane

### Section 3: Code Organization
- [x] Group keybinds by category (window, pane, session, modes)
- [x] Add comments explaining tmux compatibility
- [x] Separate vim extensions from tmux defaults

### Section 4: Testing
- [x] Update test_prefix_then_command to expect CreateWindow
- [x] Verify all 269 client tests pass

## Acceptance Criteria

- [x] `Ctrl+b c` creates new window (not pane)
- [x] `Ctrl+b n/p` navigates windows (not panes)
- [x] `Ctrl+b o` cycles through panes
- [x] `Ctrl+b ;` goes to previous pane
- [x] `Ctrl+b 0-9` selects window by index
- [x] `Ctrl+b &` closes window
- [x] All existing keybinds preserved where compatible
- [x] Vim-style navigation still works (h/j/k/l)
- [x] All tests pass

## Dependencies

- **FEAT-010**: Client Input handling (base infrastructure)

## Commits

- `a15a767` - feat(client): align prefix keybinds with tmux defaults
- `4a15c13` - feat(client): implement Prefix+s to return to session selection
