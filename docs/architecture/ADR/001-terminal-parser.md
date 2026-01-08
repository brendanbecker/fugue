# ADR-001: Terminal Parser Selection

## Status

**Accepted** - 2026-01-07

## Context

ccmux needs to parse ANSI escape sequences from PTY output to maintain terminal state (cursor position, colors, scrollback, etc.). This state is used for:

1. Rendering pane contents via ratatui
2. Screen snapshots for crash recovery
3. Efficient incremental updates to clients

Two Rust crates are commonly used for this purpose:

### Option A: vt100

- Maintained by the same author as `portable-pty`
- Simple, focused API
- `contents_diff()` method for efficient incremental updates
- Lighter weight (~5K lines of code)
- Used by Claude and ChatGPT research recommendations

### Option B: alacritty_terminal

- Extracted from the Alacritty terminal emulator
- Battle-tested with complex real-world usage
- Better handling of edge cases and Unicode
- Full TrueColor support
- Heavier (~20K lines, designed for GPU rendering)
- Recommended by Gemini research

## Decision

**Start with vt100**, benchmark with Claude Code's actual output, and fall back to alacritty_terminal only if vt100 mishandles escape sequences.

## Rationale

### Why vt100 First

1. **Simpler integration**: vt100's API maps cleanly to our needs
   ```rust
   let mut parser = vt100::Parser::new(rows, cols, scrollback);
   parser.process(pty_output);
   let screen = parser.screen();
   ```

2. **Incremental updates**: `contents_diff()` provides exactly what we need for efficient client updates
   ```rust
   let diff = parser.screen().contents_diff(prev_screen);
   // Only send changed cells to client
   ```

3. **Ecosystem alignment**: Same author as `portable-pty` suggests good integration

4. **Lower risk**: Smaller codebase means fewer potential bugs and easier debugging

5. **Adequate for Claude**: Claude Code's output is primarily text with standard ANSI colors. It doesn't use advanced terminal features like:
   - Sixel graphics
   - Complex Unicode combining characters
   - Unusual cursor movements

### When to Reconsider

Switch to alacritty_terminal if we encounter:
- Rendering artifacts with Claude's output
- Poor handling of alternate screen buffer (vim/less in panes)
- Unicode alignment issues (CJK characters, emoji)
- Performance problems with large outputs

### Migration Path

If migration is needed:
1. Create `TerminalParser` trait abstracting the interface
2. Implement for both vt100 and alacritty_terminal
3. Make parser selectable via config or feature flag
4. Default to alacritty_terminal after validation

```rust
pub trait TerminalParser: Send + Sync {
    fn process(&mut self, data: &[u8]);
    fn screen_contents(&self) -> String;
    fn cursor_position(&self) -> (u16, u16);
    fn resize(&mut self, rows: u16, cols: u16);
}
```

## Consequences

### Positive

- Faster initial development with simpler API
- Lower memory footprint
- Easier debugging
- Good alignment with our primary dependency (portable-pty)

### Negative

- May need to migrate if edge cases emerge
- alacritty_terminal's robustness not immediately available
- Some advanced features (if needed) would require migration

### Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| vt100 mishandles Claude output | Low | Medium | Benchmark early; trait abstraction |
| Performance issues | Low | Medium | Profile; optimize hot paths |
| Unicode problems | Medium | Low | Test CJK/emoji; migrate if needed |

## Validation Plan

1. **Phase 1 (Implementation)**
   - Implement with vt100
   - Add comprehensive escape sequence tests
   - Benchmark with synthetic Claude-like output

2. **Phase 2 (Integration)**
   - Run real Claude Code sessions
   - Monitor for rendering issues
   - Collect problematic sequences

3. **Phase 3 (Decision Point)**
   - If issues found: implement trait and migrate
   - If stable: document known limitations

## References

- [vt100 crate](https://crates.io/crates/vt100)
- [alacritty_terminal crate](https://crates.io/crates/alacritty_terminal)
- [tui-term](https://crates.io/crates/tui-term) - vt100 to ratatui bridge
- Research: `docs/research/SYNTHESIS.md` Section 1.2
