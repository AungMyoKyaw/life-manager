# Changelog

All notable changes to Life Manager are recorded here. The format follows Keep a Changelog and the project uses semantic versioning.

## [Unreleased]

### Added

- Schema v16: interview events, savings goals, certification deadlines, rest intentions, personal administration items, keyboard shortcuts, a security audit log, integration accounts, remote MCP clients, notification authorization state and an import ledger.
- Per-occurrence recurrence exceptions: skip or move a single occurrence without changing the series, and expand a recurring task into concrete dates.
- Planning rules: protected time, weekly time budgets with an overrun report, carryover of unfinished work, travel and buffer minutes, and a daily planning session that records the day's three priorities.
- Focus modes: pomodoro presets, manual entries for work that already happened, interruption reasons, and focus trends against planned estimates.
- Executable saved views and saved searches, bidirectional backlinks between notes and tasks, and a capture parser that turns natural language into a task draft.
- Four explainable insight reports: plan versus actual, goal achievement, area time distribution and a journal export. Every report names the local records it came from.
- Agent policy: per-actor action permissions enforced inside the domain, a proposal and approval workflow, and daily or weekly automation schedules.
- A remote MCP server over HTTP with bearer-token authentication, bound to the loopback interface by default.
- CSV import with header mapping, preview, duplicate detection and partial-failure reporting.
- The remaining personal life-area trackers: interviews, savings goals, certification deadlines, sleep and rest intentions, and personal administration.
- A live accessibility inspection of the app's own focus, contrast, motion and keyboard behaviour.
- A command palette that runs named commands as well as searching.

### Changed

- The MCP catalog is generated from one action specification table. Every tool, schema, required parameter and the tool-name mapping now come from the same source, and every previously published tool name still resolves.
- The command palette searches every entity and runs named commands.

### Fixed

- The daily schedule proposal is deterministic when a reference time is supplied, so a proposal no longer depends on the wall clock.

## [0.1.0]

- The first local release: tasks, inbox, calendar, projects, goals, focus, habits, routines, notes, reviews, agents, automations, settings, and a standalone CLI with a local MCP server.
