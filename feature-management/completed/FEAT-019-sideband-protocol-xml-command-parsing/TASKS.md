# Task Breakdown: FEAT-019

**Work Item**: [FEAT-019: Sideband Protocol - XML Command Parsing from Claude Output](PROMPT.md)
**Status**: Not Started
**Last Updated**: 2026-01-08

## Prerequisites

- [ ] Read and understand PROMPT.md
- [ ] Review PLAN.md and update if needed
- [ ] Verify FEAT-014 dependency status
- [ ] Review existing PTY output handling code

## Design Tasks

- [ ] Design SidebandCommand enum with all variants
- [ ] Design parser state machine
- [ ] Design output filtering strategy
- [ ] Plan event emission architecture
- [ ] Document XML grammar for sideband commands

## Implementation Tasks

### Command Types (commands.rs)
- [ ] Create SidebandCommand enum
- [ ] Implement SpawnCommand struct with cmd, cwd, env fields
- [ ] Implement InputCommand struct with target, text fields
- [ ] Implement ControlCommand struct with action, target fields
- [ ] Implement ControlAction enum (focus, close, etc.)
- [ ] Implement CanvasCommand struct with type, path fields
- [ ] Implement CanvasType enum (diff, image, etc.)
- [ ] Add validation methods for each command type
- [ ] Implement Display trait for debugging

### Parser Core (parser.rs)
- [ ] Create SidebandParser struct with state and buffer
- [ ] Implement ParserState enum (Passthrough, TagStart, etc.)
- [ ] Implement process_byte() method for streaming parsing
- [ ] Implement process_chunk() for bulk processing
- [ ] Implement tag detection (`<fugue:` prefix)
- [ ] Implement attribute parsing (key="value" pairs)
- [ ] Implement self-closing tag handling (`/>`)
- [ ] Implement content extraction for tags with body
- [ ] Implement closing tag matching (`</fugue:*>`)

### Error Handling
- [ ] Define ParseError enum
- [ ] Handle malformed opening tags
- [ ] Handle mismatched closing tags
- [ ] Handle invalid attribute values
- [ ] Handle unknown command types
- [ ] Implement graceful degradation (pass through on error)

### Output Filtering
- [ ] Track non-command output separately
- [ ] Implement filtered output extraction
- [ ] Handle partial commands at chunk boundaries
- [ ] Add timeout for stale partial commands
- [ ] Preserve exact byte sequence for non-command output

### Event Emission
- [ ] Define SidebandEvent enum
- [ ] Create event channel (mpsc sender/receiver)
- [ ] Emit CommandParsed events
- [ ] Emit ParseError events for debugging
- [ ] Document event consumer interface

### Module Integration (mod.rs)
- [ ] Export public types
- [ ] Create SidebandProcessor facade
- [ ] Add configuration options (enable/disable, rate limit)
- [ ] Document module API

## Testing Tasks

### Unit Tests
- [ ] Test SpawnCommand parsing with all attributes
- [ ] Test SpawnCommand parsing with minimal attributes
- [ ] Test InputCommand parsing with text content
- [ ] Test InputCommand with special characters in text
- [ ] Test ControlCommand parsing with various actions
- [ ] Test CanvasCommand parsing with type and path
- [ ] Test unknown command type handling
- [ ] Test malformed XML handling
- [ ] Test attribute edge cases (quotes, escapes)

### Integration Tests
- [ ] Test parser with PTY output stream
- [ ] Test output filtering preserves non-command content
- [ ] Test partial command buffering across reads
- [ ] Test multiple commands in single chunk
- [ ] Test interleaved commands and regular output

### Edge Case Tests
- [ ] Test empty command body
- [ ] Test very long command attributes
- [ ] Test deeply nested content (should not happen)
- [ ] Test rapid command sequence
- [ ] Test command at exact buffer boundary

## Documentation Tasks

- [ ] Document supported command syntax
- [ ] Document attribute requirements per command
- [ ] Document error handling behavior
- [ ] Add usage examples for Claude integration
- [ ] Document security considerations

## Verification Tasks

- [ ] All acceptance criteria from PROMPT.md met
- [ ] Tests passing
- [ ] Update feature_request.json status
- [ ] Document completion in PLAN.md

## Completion Checklist

- [ ] All implementation tasks complete
- [ ] All tests passing
- [ ] Documentation updated
- [ ] PLAN.md reflects final implementation
- [ ] Ready for review/merge

---
*Check off tasks as you complete them. Update status field above.*
