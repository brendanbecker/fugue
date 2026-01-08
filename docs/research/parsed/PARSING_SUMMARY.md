# Document Parsing Summary

> Generated: 2026-01-07
> Stage 3 of ccmux development pipeline

## Documents Processed

| Source | Format | Size | Est. Tokens | Sections |
|--------|--------|------|-------------|----------|
| claude_research.md | Markdown | 24KB | ~4,500 | 7 major + intro/conclusion |
| gemini_research.md | Markdown | 36KB | ~8,500 | 10 major sections |
| chatgpt_research.pdf | PDF | 351KB | ~12,000 | 5 major sections |

**Total estimated tokens across sources:** ~25,000

## Output Files Generated

### Per-Source Files (12 total)

| File Type | Claude | Gemini | ChatGPT |
|-----------|--------|--------|---------|
| `*_structure.json` | ✓ | ✓ | ✓ |
| `*_section_map.md` | ✓ | ✓ | ✓ |
| `*_abstracts.md` | ✓ | ✓ | ✓ |
| `*_metadata.json` | ✓ | ✓ | ✓ |

### File Descriptions

1. **`*_structure.json`** - Hierarchical section structure with:
   - Section IDs, titles, levels
   - Line/page ranges
   - Token estimates per section
   - Nested children for subsections

2. **`*_section_map.md`** - Visual navigation tree showing:
   - ASCII tree of document structure
   - Token counts per section
   - Quick reference tables by topic
   - Content markers (CODE, TABLE, etc.)

3. **`*_abstracts.md`** - Per-section summaries:
   - 100-200 token abstracts
   - Key insights condensed
   - Section location references

4. **`*_metadata.json`** - Extracted technical details:
   - Crate recommendations with purposes
   - Architectural patterns
   - Claude Code detection strategies
   - Code block references
   - Key term definitions
   - External links and references

## Parsing Approach

### Claude Research (Markdown)
- Parsed via structural analysis of headings
- 7 major numbered sections identified
- Code blocks catalogued with line ranges
- Crates extracted with version numbers

### Gemini Research (Markdown)
- Parsed via heading hierarchy
- 10 major sections including deliverables
- Deeper subsection nesting (up to level 4)
- 48 references in Works Cited section

### ChatGPT Research (PDF)
- Parsed via page-based analysis
- 5 major thematic sections
- More detailed subsection breakdown
- Code snippets with page references

## Key Observations

### Consensus Areas
All three sources agree on:
- **Core stack**: `portable-pty` + terminal parser + `ratatui`
- **Architecture**: Client-server model (daemon holds PTYs)
- **Claude integration**: `--output-format stream-json` for structured output
- **Session resume**: `claude --resume <session_id>`
- **Config reload**: `notify` crate with debouncing

### Divergence Points
- **Terminal parser**: Claude recommends `vt100`; Gemini prefers `alacritty_terminal`
- **Config access**: Claude uses `ArcSwap`; others less specific
- **MCP integration**: Gemini emphasizes MCP server approach; others focus on sideband protocol

### Unique Contributions
- **Claude**: XML-like namespaced protocol (`<ccmux:spawn>`), SKILL.md definition, okaywal WAL
- **Gemini**: Visual telemetry states (Channelling, Synthesizing, Discombobulating), `arc_swap` pattern
- **ChatGPT**: Detailed CLI flag reference, concurrent session isolation via HOME directories

## Chunk Size Analysis

Target chunk size: 400-900 tokens

| Source | Sections in Range | Oversized | Undersized |
|--------|-------------------|-----------|------------|
| Claude | 18/24 (75%) | 4 | 2 |
| Gemini | 22/28 (79%) | 3 | 3 |
| ChatGPT | 15/22 (68%) | 5 | 2 |

Most sections fall within the target retrieval window for efficient context injection.

## Next Steps

1. **SYNTHESIS.md** - Cross-source unified analysis with:
   - Consolidated crate recommendations
   - Merged architectural decisions
   - Conflict resolution notes
   - Implementation priority queue

2. **HANDOFF.md Update** - Stage 4 (Architecture Generation) preparation
