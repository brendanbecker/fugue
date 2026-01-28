---
from: orch-fugue
to: worker-feat-124
type: task
timestamp: 2026-01-28T15:40:00Z
needs_response: true
priority: normal
tags: [FEAT-124, mail-storage, documentation]
thread_id: feat-124-implementation
---

## Task Assignment: FEAT-124 Mail Storage Format

You are assigned to implement FEAT-124: Filesystem-Based Mail Storage Format.

**Deliverables**:
1. Finalize directory structure in `docs/design/MAIL_STORAGE.md`
2. Define YAML frontmatter schema with all required/optional fields
3. Document filename convention with examples
4. Document mailbox resolution rules (session name vs tag vs broadcast)
5. Document message lifecycle (created → unread → read → archived)
6. Create example message files

**Branch**: `FEAT-124-mail-storage`
**Worktree**: `/home/becker/projects/tools/fugue-FEAT-124`

**Dependencies**: None (this is foundation for FEAT-125 and FEAT-126)

Report status when starting work and when complete.
