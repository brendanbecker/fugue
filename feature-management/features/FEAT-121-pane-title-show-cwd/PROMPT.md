# FEAT-121: Show cwd in pane title instead of "pane"

**Priority**: P2
**Component**: ccmux-client
**Type**: enhancement
**Estimated Effort**: tiny
**Business Value**: medium

## Overview

Change the default pane title from the literal string "pane" to show the current working directory (cwd) of the terminal in that pane.

## Problem Statement

Currently, when a pane has no explicit title set, the border shows "pane" as a placeholder. This provides no useful information to the user. The cwd is already tracked per-pane and would be much more informative.

## Current Behavior

```
┌─pane────────────────────────────────┐
│ $ ls                                │
│ file1.txt  file2.txt                │
└─────────────────────────────────────┘
```

## Desired Behavior

```
┌─~/projects/ccmux──────────────────────┐
│ $ ls                                  │
│ file1.txt  file2.txt                  │
└───────────────────────────────────────┘
```

Or with home directory abbreviation:

```
┌─/home/user/projects/ccmux────────────┐
```

## Implementation

### Location

**File**: `ccmux-client/src/ui/pane.rs`

**Function**: `display_title()` (line 384)

### Current Code

```rust
pub fn display_title(&self) -> String {
    let base_title = self.title.as_deref().unwrap_or("pane");
    // ...
}
```

### Proposed Change

```rust
pub fn display_title(&self) -> String {
    let base_title = self.title.as_deref()
        .or(self.cwd.as_deref())
        .unwrap_or("pane");
    // ...
}
```

This uses the fallback chain: `title` -> `cwd` -> "pane"

### Optional Enhancement: Abbreviate Home Directory

For better readability, abbreviate the home directory with `~`:

```rust
fn abbreviate_home(path: &str) -> String {
    if let Some(home) = dirs::home_dir() {
        if let Some(home_str) = home.to_str() {
            if path.starts_with(home_str) {
                return path.replacen(home_str, "~", 1);
            }
        }
    }
    path.to_string()
}

pub fn display_title(&self) -> String {
    let base_title = self.title.as_deref()
        .map(|s| s.to_string())
        .or_else(|| self.cwd.as_ref().map(|cwd| abbreviate_home(cwd)))
        .unwrap_or_else(|| "pane".to_string());
    // ...
}
```

## Implementation Tasks

### Section 1: Basic Implementation

- [ ] Edit `ccmux-client/src/ui/pane.rs`
- [ ] Update `display_title()` to use cwd as fallback
- [ ] Test that explicit title still takes precedence

### Section 2: Home Directory Abbreviation (Optional)

- [ ] Add `abbreviate_home()` helper function
- [ ] Apply abbreviation to cwd display
- [ ] Test with paths inside and outside home directory

### Section 3: Testing

- [ ] Manual test: pane with no title shows cwd
- [ ] Manual test: pane with explicit title still shows title
- [ ] Manual test: cwd updates when `cd` is run (if cwd tracking works)

## Files to Modify

| File | Changes |
|------|---------|
| `ccmux-client/src/ui/pane.rs` | Update `display_title()` fallback logic |

## Acceptance Criteria

- [ ] Panes without explicit title show cwd instead of "pane"
- [ ] Panes with explicit title still show the title
- [ ] (Optional) Home directory abbreviated to `~`
- [ ] No performance impact (cwd already tracked)

## Notes

### CWD Tracking

The `Pane` struct already has a `cwd: Option<String>` field (line 141) and `set_cwd()` method (line 203). The daemon should be sending cwd updates via pane state messages.

If cwd is not being populated, that would be a separate issue to investigate.

### Truncation

Long paths may need truncation to fit in the border. Consider showing the last N path components if the full path is too long for the pane width. This could be a follow-up enhancement.
