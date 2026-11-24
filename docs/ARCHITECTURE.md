# Life Manager — Architecture

**Status:** the implemented system. This document describes what runs, not what was planned. The design is independent of any previous LifeOS or other project.

## Stack and boundaries

- Native Rust [GPUI](https://gpui.rs) (`crates/life-gpui` via `gpui-kit`) desktop shell, macOS-primary (CI also builds and tests `life` and `life-gpui` on Linux and Windows; Tauri/Svelte webview was retired).
- Rust workspace: `life-core` (domain models, SQLite/SQLCipher persistence, migrations, FTS5 table, scheduling, security policies), `life-cli` (standalone headless Rust CLI + local/remote MCP server), and `life-gpui` (native GPU-accelerated desktop app), with careful module boundaries rather than microservices.
- SQLite, one normalized primary database in the user's macOS Application Support directory, in WAL mode with foreign keys, busy timeout, versioned migrations, and verified snapshots.
- Optional small native reminder helper for scheduled notifications when the GUI is closed. Avoid a permanently running HTTP service for local core tasks.
- The GUI and CLI both call `life_core::Store` (`Store::call` / `BackendHandle`) for validation, authorization and scheduling policies. Neither writes arbitrary SQL.

## Primary dependency flow

Native GPUI UI (`life-gpui`) → in-process typed `BackendHandle` → `life-core` → SQLite / SQLCipher.

Standalone CLI (`life-cli`) → `life-core::Store` → SQLite / SQLCipher.

MCP stdio / remote HTTP (`life-cli`) → validated tool input → `life-core::Store` → SQLite / SQLCipher.

Event notifications flow outward from committed database revisions. A GUI opened after CLI changes queries current authoritative state, rather than trusting cached frontend state. All updates should carry revision and entity ID for deterministic refresh.

## Domain boundaries

1. Capture and tasks: task hierarchy, states, labels, recurrence, templates, dependencies, activity.
2. Projects and goals: projects, milestones, optional objectives/key results, progress, links.
3. Calendar: sources, recurrence/exceptions, events, availability rules, task blocks, conflict detection.
4. Focus and habits: sessions, intervals, skip rules, routines, actual duration and streak calculations.
5. Planning: capacity calculation, deliberate priority selection, time-budget constraints, deterministic candidate slots, preview/approval.
6. Notes and search: Markdown, backlinks, local attachments, FTS5 and smart filters.
7. Reviews and analytics: derived statistics, historical slices, explainable metrics, journal entries.
8. Agents and automation: typed tools, action provenance, idempotency, change sets, safe undo, event triggers and rule engine.
9. Platform: paths, backups, notification helper, imports/exports, external adapters, diagnostics.

## Scheduling rules

- All real-world instants stored in UTC; retain IANA zone IDs and local wall-time recurrence definitions for meaningful calendar rendering.
- Floating all-day dates are date values; do not convert them blindly into midnight UTC.
- Each task can have distinct due time, scheduled block(s), estimated effort, actual focus intervals and completion time. A time block references a task, never copies task content as an independent authoritative record.
- Availability is the union/subtraction of user-defined windows, external busy periods, buffers, and protected blocks. Detect collisions before persisting.
- Scheduling proposals are deterministic, reproducible and explainable with an input revision. Applying a proposal verifies the revision and lock status, then commits atomically.
- Recurrence uses explicit inclusion/exclusion dates and detached instance exceptions; DST ambiguous or nonexistent wall times need a documented resolution policy per calendar source.
- Do not silently shift an overdue item or overwrite an existing calendar commitment.

## CLI/MCP process behavior

- The CLI opens SQLite independently, with short writes, indexed reads and structured error statuses. It must not call into a running desktop process.
- MCP defaults to stdio (`life mcp serve --stdio`), launched by the agent host, and exposes constrained typed resource operations. Remote HTTP transport (`life mcp remote`) is always compiled and guarded at runtime by bearer tokens (`remote_mcp_clients`), binding `127.0.0.1` by default and requiring `--allow-remote` for non-loopback addresses.
- For every mutation: validate input, check domain invariants, enforce idempotency, record actor/source and before/after state, commit, emit revision notification.
- SQLite WAL permits readers while a writer commits; busy handling uses bounded retries and explicit BUSY errors, not unbounded spinning.
- Do not use low-level SQL, shell, arbitrary filesystem operations or unrestricted dynamic execution as agent task tools.

## Local storage and backups

- Managed attachments are stored in the `managed_attachments` table inside `life.sqlite` (`content BLOB NOT NULL`, 8 MiB limit) with SHA-256 checksums.
- Use SQLite's consistent online backup mechanism for a complete snapshot; verify integrity and retention after creation. Never copy the live WAL database file naively while it may be changing.
- Migration path: snapshot → validate current version → transactional migration → integrity check → advance version, with explicit failure recovery.
- When SQLCipher encryption is enabled, headless CLI/MCP access requires `LIFE_DB_PASSPHRASE` or `LIFE_DB_KEY` at process start (`Store::open` fails closed without them).
- Import workflow: parse → schema validation → duplicate detection → preview → bounded transactional apply → detailed report. On failure, report precisely what committed.

## Event, version and concurrency semantics

- Every user-visible mutable entity has a stable UUIDv7 and monotonically increasing revision number.
- All mutating API requests can carry an expected revision and idempotency key. Concurrent edits produce explicit stale-write conflicts rather than silent overwrites.
- Agent and automation actions are appended to an activity ledger with actor, source, request ID, timestamps, status and undo eligibility.
- Clients independently reconcile committed revision updates. Frontend and CLI never maintain competing sources of truth.

## Operational and performance budgets

- Target warm local CLI read below 100 ms for typical indexed queries, with measurable environment-specific p95 and p99 budgets.
- `task_search` (FTS5) is updated on task create, update, and hard delete; `Store::search` currently queries via `LIKE '%...%'` rather than FTS5 `MATCH`.
- Prefer lazy initialization and virtualized large tables. Background helpers must remain lightweight and recover across macOS sleep/restart.
- Release targets must be measured on macOS Apple Silicon, not inferred from tool benchmarks.

## Security model

- One macOS user boundary and filesystem permissions; no forced account login.
- Optional app lock and optional encryption are separate user decisions.
- External integrations (`integrations.configure`) currently check `LIFE_*` environment variables and record status in `integration_accounts`; macOS Keychain storage and live OAuth/CalDAV/EventKit sync are not yet implemented. Logs redact secrets and private note content by default.
- Destructive bulk operations require review; local MCP writes are scoped to supported resources and recorded by actor.
- Third-party content and agent prompts are data, not new system instructions. External integrations are explicit opt-in.

## Implementation status

The workspace implements Schema v16 across `crates/life-core`, `crates/life-cli`, and `crates/life-gpui` (18 `WorkspaceView` screens, 265 dispatched actions in `Store::dispatch`, 263 cataloged CLI/MCP actions in `action_spec.rs`).
