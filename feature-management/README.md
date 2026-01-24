# fugue Feature Management

This directory contains bug reports, feature requests, and task tracking for the fugue project.

## Directory Structure

- **bugs/** - Bug reports with implementation plans
- **features/** - Feature requests with implementation plans
- **completed/** - Archived completed items
- **deprecated/** - Deprecated or obsolete items
- **human-actions/** - Items requiring human intervention
- **agent_runs/** - Automated agent session reports and logs
- **schemas/** - JSON schemas for work item validation

## Workflow

This project uses the featmgmt pattern for automated bug resolution and feature implementation.

See [OVERPROMPT.md](./OVERPROMPT.md) for the complete automated workflow (when available).

## Key Files

- **OVERPROMPT.md** - Autonomous agent workflow definition (to be added)
- **.agent-config.json** - Project-specific configuration
- **bugs/bugs.md** - Bug summary file
- **features/features.md** - Feature summary file

## Pattern Version

Managed by featmgmt v1.0.0

Project Type: standard

## Project Components

fugue is organized into these logical components:

| Component | Description |
|-----------|-------------|
| `pty` | PTY spawning and management (portable-pty) |
| `terminal` | Terminal state parsing (ANSI, VT100) |
| `tui` | User interface rendering (ratatui, crossterm) |
| `session` | Session state and persistence |
| `claude` | Claude Code detection and integration |
| `config` | Configuration and hot-reload |
| `orchestration` | Pane spawning and session trees |
