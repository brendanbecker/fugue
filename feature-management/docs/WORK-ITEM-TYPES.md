# Work Item Types

This document formally defines the work item types used in the featmgmt system. Each type has a specific purpose, structure, lifecycle, and relationship to other work items.

## Overview

| Type | Prefix | Purpose | Primary Actor | Output |
|------|--------|---------|---------------|--------|
| **BUG** | `BUG-XXX` | Fix defects in existing functionality | Agent | Code changes |
| **FEAT** | `FEAT-XXX` | Add new functionality or enhance existing | Agent | Code changes |
| **INQ** | `INQ-XXX` | Structured deliberation for complex decisions | Multi-agent | FEAT work item(s) |
| **ACTION** | `ACTION-XXX` | Tasks requiring human intervention | Human | Varies |

## Type Hierarchy and Relationships

```
INQ-XXX (deliberation)
    └── spawns → FEAT-XXX (implementation)
                     └── may spawn → BUG-XXX (defects found during implementation)
                     └── may spawn → ACTION-XXX (human intervention needed)

BUG-XXX (defect fix)
    └── may spawn → FEAT-XXX (if fix requires new capability)
    └── may spawn → ACTION-XXX (human intervention needed)
```

---

## BUG - Bug/Defect

### Definition

A **BUG** represents a defect in existing functionality where the software does not behave as specified or expected. Bugs describe something that is broken and needs to be fixed.

### When to Use

Use BUG when:
- Existing functionality produces incorrect results
- Expected behavior differs from actual behavior
- A regression has been introduced
- An error or exception occurs unexpectedly
- Performance has degraded below acceptable thresholds

Do NOT use BUG when:
- Adding new functionality (use FEAT)
- The behavior is working as designed but needs improvement (use FEAT)
- You're unsure what the correct behavior should be (use INQ)
- Human intervention is required (use ACTION)

### Required Fields

| Field | Type | Description |
|-------|------|-------------|
| `bug_id` | string | Unique identifier (pattern: `BUG-\d{3,}`) |
| `title` | string | Concise description (5-200 chars) |
| `component` | string | Affected component/module |
| `severity` | enum | `critical`, `high`, `medium`, `low` |
| `priority` | enum | `P0`, `P1`, `P2`, `P3` |
| `status` | enum | See lifecycle below |
| `reported_date` | date | When bug was first reported |
| `updated_date` | date | Last modification date |
| `description` | string | Detailed explanation (10+ chars) |
| `steps_to_reproduce` | array | Steps to reproduce (min 1) |
| `expected_behavior` | string | What should happen (5+ chars) |
| `actual_behavior` | string | What actually happens (5+ chars) |

### Optional Fields

| Field | Type | Description |
|-------|------|-------------|
| `assigned_to` | string/null | Agent/person assigned |
| `tags` | array | Categorization tags |
| `affected_versions` | array | Versions where bug occurs |
| `environment` | string | Environment (production, staging, etc.) |
| `reproducibility` | enum | `always`, `sometimes`, `rare`, `unknown` |
| `evidence` | object | Supporting evidence (logs, screenshots) |
| `root_cause` | string | Identified root cause |
| `impact` | string | Business/user impact |

### Lifecycle (Status States)

```
new → in_progress → blocked → resolved → verified → closed
                 ↘                    ↗
                   → wont_fix ───────→
```

| Status | Description |
|--------|-------------|
| `new` | Bug reported, not yet started |
| `in_progress` | Actively being worked on |
| `blocked` | Waiting on external dependency |
| `resolved` | Fix implemented, awaiting verification |
| `verified` | Fix confirmed working |
| `closed` | Bug fully resolved and archived |
| `wont_fix` | Decision made not to fix |

### Required Files

| File | Purpose |
|------|---------|
| `bug_report.json` | Structured metadata |
| `PROMPT.md` | Self-executing implementation instructions |
| `PLAN.md` | Implementation plan and architecture |
| `TASKS.md` | Task breakdown with progress tracking |

### Directory Structure

```
bugs/BUG-XXX-descriptive-slug/
├── bug_report.json     # Required: Metadata
├── PROMPT.md           # Required: Implementation instructions
├── PLAN.md             # Required: Implementation plan
├── TASKS.md            # Required: Task breakdown
└── comments.md         # Optional: Progress notes
```

---

## FEAT - Feature/Enhancement

### Definition

A **FEAT** represents new functionality to be added or improvements to existing functionality. Features describe something that should be built or enhanced.

### When to Use

Use FEAT when:
- Adding new capabilities to the system
- Enhancing existing functionality
- Improving user experience
- Refactoring for better architecture
- Optimizing performance (as a planned improvement)

Do NOT use FEAT when:
- Fixing broken functionality (use BUG)
- The design/approach is unclear (use INQ first)
- Human intervention is required (use ACTION)

### Required Fields

| Field | Type | Description |
|-------|------|-------------|
| `feature_id` | string | Unique identifier (pattern: `FEAT-\d{3,}`) |
| `title` | string | Concise description (5-200 chars) |
| `component` | string | Target component/module |
| `priority` | enum | `P0`, `P1`, `P2`, `P3` |
| `status` | enum | See lifecycle below |
| `type` | enum | `enhancement`, `new_feature`, `improvement` |
| `created_date` | date | When feature was requested |
| `updated_date` | date | Last modification date |
| `description` | string | Detailed explanation (10+ chars) |
| `estimated_effort` | enum | `small`, `medium`, `large`, `xl` |
| `business_value` | enum | `high`, `medium`, `low` |

### Optional Fields

| Field | Type | Description |
|-------|------|-------------|
| `assigned_to` | string/null | Agent/person assigned |
| `tags` | array | Categorization tags |
| `dependencies` | array | Other work items this depends on |
| `technical_complexity` | enum | `high`, `medium`, `low` |
| `user_impact` | string | How users are affected |

### Lifecycle (Status States)

```
new → in_progress → blocked → completed → verified → closed
                 ↘                     ↗
                   → wont_do ─────────→
```

| Status | Description |
|--------|-------------|
| `new` | Feature requested, not yet started |
| `in_progress` | Actively being implemented |
| `blocked` | Waiting on external dependency |
| `completed` | Implementation finished, awaiting verification |
| `verified` | Feature confirmed working |
| `closed` | Feature fully complete and archived |
| `wont_do` | Decision made not to implement |

### Required Files

| File | Purpose |
|------|---------|
| `feature_request.json` | Structured metadata |
| `PROMPT.md` | Self-executing implementation instructions |
| `PLAN.md` | Implementation plan and architecture |
| `TASKS.md` | Task breakdown with progress tracking |

### Directory Structure

```
features/FEAT-XXX-descriptive-slug/
├── feature_request.json  # Required: Metadata
├── PROMPT.md             # Required: Implementation instructions
├── PLAN.md               # Required: Implementation plan
├── TASKS.md              # Required: Task breakdown
└── comments.md           # Optional: Progress notes
```

---

## INQ - Inquiry (Deliberation Process)

### Definition

An **INQ** represents a structured deliberation process for reaching consensus on complex decisions. Inquiries are used when the approach, design, or solution is unclear and requires multi-perspective analysis before implementation can begin.

### When to Use

Use INQ when:
- Multiple valid approaches exist and the best choice is unclear
- The problem requires deep exploration before solution design
- Stakeholders have conflicting perspectives that need resolution
- Architectural decisions have long-term implications
- The team needs to build shared understanding before proceeding
- Trade-offs need formal analysis and documentation

Do NOT use INQ when:
- The solution is clear and well-understood (use FEAT directly)
- You're fixing a known defect (use BUG)
- You need human intervention (use ACTION)
- The decision is trivial or easily reversible

### Inquiry Phases

An INQ progresses through four mandatory phases:

#### Phase 1: Independent Research

**Purpose**: Gather diverse perspectives through parallel, independent exploration.

**Process**:
- Multiple agents explore the problem space independently
- Each agent produces findings without seeing others' work
- Prevents groupthink and ensures diverse viewpoints
- Outputs: Individual research reports (`research/agent-N.md`)

**Completion Criteria**:
- All assigned agents have submitted research reports
- Each report covers: problem analysis, potential approaches, evidence gathered

#### Phase 2: Synthesis

**Purpose**: Consolidate findings into a unified understanding.

**Process**:
- A synthesis agent reads all research reports
- Identifies common themes, conflicts, and unique insights
- Creates a consolidated synthesis document
- Highlights areas of agreement and disagreement
- Output: `SYNTHESIS.md`

**Completion Criteria**:
- Synthesis document captures all perspectives
- Key decision points are clearly identified
- Areas requiring debate are enumerated

#### Phase 3: Debate

**Purpose**: Resolve conflicts through structured argumentation.

**Process**:
- Adversarial agents argue different perspectives
- Each position must be defended with evidence
- Counter-arguments are formally documented
- Output: `DEBATE.md` with structured arguments

**Debate Structure**:
```markdown
## Decision Point: [Topic]
### Position A: [Stance]
- Argument 1
- Argument 2
- Evidence

### Position B: [Stance]
- Argument 1
- Argument 2
- Evidence

### Resolution
[Which position prevails and why]
```

**Completion Criteria**:
- All key decision points have been debated
- Each position has been given fair representation
- Resolutions are documented with rationale

#### Phase 4: Consensus

**Purpose**: Formalize the agreed-upon approach and spawn implementation work.

**Process**:
- Create `CONSENSUS.md` documenting final decisions
- Create FEAT work item(s) for implementation
- Link FEAT back to INQ for traceability
- Output: `CONSENSUS.md` + spawned FEAT(s)

**Completion Criteria**:
- All decision points have clear resolutions
- At least one FEAT has been created
- FEAT includes reference to source INQ

### Required Fields

| Field | Type | Description |
|-------|------|-------------|
| `inquiry_id` | string | Unique identifier (pattern: `INQ-\d{3,}`) |
| `title` | string | Concise description (5-200 chars) |
| `component` | string | Primary affected component |
| `priority` | enum | `P0`, `P1`, `P2`, `P3` |
| `status` | enum | See lifecycle below |
| `phase` | enum | `research`, `synthesis`, `debate`, `consensus`, `completed` |
| `created_date` | date | When inquiry was opened |
| `updated_date` | date | Last modification date |
| `question` | string | The core question to be answered (10+ chars) |
| `context` | string | Background and why this decision matters (20+ chars) |
| `constraints` | array | Non-negotiable requirements (min 1) |
| `research_agents` | integer | Number of independent research agents (min 2) |

### Optional Fields

| Field | Type | Description |
|-------|------|-------------|
| `assigned_to` | string/null | Lead facilitator |
| `tags` | array | Categorization tags |
| `stakeholders` | array | Interested parties |
| `deadline` | date | Decision deadline if any |
| `scope` | string | Boundaries of the inquiry |
| `spawned_features` | array | FEAT IDs created from this inquiry |
| `alternatives_considered` | array | Summary of rejected approaches |

### Lifecycle (Status States)

```
new → research → synthesis → debate → consensus → completed
   ↘                                           ↗
     → cancelled ─────────────────────────────→
```

| Status | Description |
|--------|-------------|
| `new` | Inquiry created, not yet started |
| `research` | Phase 1: Independent research in progress |
| `synthesis` | Phase 2: Consolidating findings |
| `debate` | Phase 3: Adversarial argumentation |
| `consensus` | Phase 4: Formalizing decisions |
| `completed` | Consensus reached, FEAT(s) spawned |
| `cancelled` | Inquiry abandoned (document why) |

### Required Files

| File | Purpose |
|------|---------|
| `inquiry_report.json` | Structured metadata |
| `QUESTION.md` | Formal problem statement and context |
| `research/` | Directory for research phase outputs |
| `SYNTHESIS.md` | Phase 2 consolidated findings |
| `DEBATE.md` | Phase 3 structured arguments |
| `CONSENSUS.md` | Phase 4 final decisions |

### Directory Structure

```
inquiries/INQ-XXX-descriptive-slug/
├── inquiry_report.json   # Required: Metadata
├── QUESTION.md           # Required: Problem statement
├── research/             # Required: Research outputs
│   ├── agent-1.md        # Independent research report
│   ├── agent-2.md        # Independent research report
│   └── agent-N.md        # Additional agents as configured
├── SYNTHESIS.md          # Required: Consolidated findings
├── DEBATE.md             # Required: Structured arguments
├── CONSENSUS.md          # Required: Final decisions
└── comments.md           # Optional: Process notes
```

### Spawning Features

When an INQ reaches the `consensus` phase, it spawns one or more FEAT work items:

```json
{
  "feature_id": "FEAT-XXX",
  "source_inquiry": "INQ-YYY",
  "rationale": "Consensus decision from INQ-YYY Phase 4"
}
```

The spawned FEAT should reference the INQ in its description and link to the `CONSENSUS.md` for architectural context.

---

## ACTION - Human Action

### Definition

An **ACTION** represents a task that requires human intervention. Actions are created when autonomous agents cannot proceed without human input, approval, or execution.

### When to Use

Use ACTION when:
- Credentials or secrets need to be configured
- External service registration is required
- Manual verification is needed
- Approval from a stakeholder is required
- Physical or external system access is needed
- A decision requires human judgment beyond agent capability

Do NOT use ACTION when:
- The task can be automated (use BUG or FEAT)
- You need to explore options (use INQ)
- The work is purely technical (use BUG or FEAT)

### Required Fields

| Field | Type | Description |
|-------|------|-------------|
| `action_id` | string | Unique identifier (pattern: `ACTION-\d{3,}`) |
| `title` | string | Concise description (5-200 chars) |
| `component` | string | Related component/module |
| `urgency` | enum | `critical`, `high`, `medium`, `low` |
| `status` | enum | See lifecycle below |
| `created_date` | date | When action was created |
| `updated_date` | date | Last modification date |
| `description` | string | What needs to be done (10+ chars) |
| `reason` | string | Why human intervention is required (5+ chars) |

### Optional Fields

| Field | Type | Description |
|-------|------|-------------|
| `assigned_to` | string/null | Person assigned |
| `tags` | array | Categorization tags |
| `required_expertise` | string | Skills needed (security, devops, etc.) |
| `estimated_time` | string | Time estimate (30 minutes, 2 hours) |
| `blocking_items` | array | Work items blocked by this action |
| `evidence` | object | Context and supporting information |

### Lifecycle (Status States)

```
pending → in_progress → completed
                     ↘
                       → cancelled
                       → blocked
```

| Status | Description |
|--------|-------------|
| `pending` | Action created, waiting to start |
| `in_progress` | Human is working on it |
| `completed` | Action finished |
| `cancelled` | Action no longer needed |
| `blocked` | Cannot proceed, needs something else |

### Required Files

| File | Purpose |
|------|---------|
| `action_report.json` | Structured metadata |
| `INSTRUCTIONS.md` | Step-by-step instructions for human |

### Directory Structure

```
human-actions/ACTION-XXX-descriptive-slug/
├── action_report.json   # Required: Metadata
├── INSTRUCTIONS.md      # Required: Human instructions
└── comments.md          # Optional: Notes and updates
```

---

## Choosing the Right Type

```
                    ┌─────────────────────────────────────┐
                    │         Is something broken?         │
                    └──────────────────┬──────────────────┘
                                       │
                    ┌──────────────────┴──────────────────┐
                    │                                      │
                   YES                                    NO
                    │                                      │
                    ▼                                      ▼
              ┌─────────┐               ┌────────────────────────────┐
              │   BUG   │               │  Do you know how to        │
              └─────────┘               │  implement the solution?   │
                                        └────────────┬───────────────┘
                                                     │
                                    ┌────────────────┴────────────────┐
                                    │                                  │
                                   YES                                NO
                                    │                                  │
                                    ▼                                  ▼
                         ┌─────────────────────┐            ┌─────────────┐
                         │  Can agents do it   │            │     INQ     │
                         │  autonomously?      │            └─────────────┘
                         └──────────┬──────────┘
                                    │
                        ┌───────────┴───────────┐
                        │                       │
                       YES                     NO
                        │                       │
                        ▼                       ▼
                  ┌─────────┐            ┌──────────┐
                  │  FEAT   │            │  ACTION  │
                  └─────────┘            └──────────┘
```

## Summary Table Location

Each work item type has a summary table file:

| Type | Summary File | Location |
|------|--------------|----------|
| BUG | `bugs.md` | `bugs/bugs.md` |
| FEAT | `features.md` | `features/features.md` |
| INQ | `inquiries.md` | `inquiries/inquiries.md` |
| ACTION | `actions.md` | `human-actions/actions.md` |

## Version History

| Version | Date | Changes |
|---------|------|---------|
| 1.0.0 | 2025-01-19 | Initial formal definitions for BUG, FEAT, INQ, ACTION |
