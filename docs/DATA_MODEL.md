# The data model

One SQLite database holds everything. The schema version is in `schema_migrations` and the live value is reported by `life --json call system.status --payload '{}'`.

## Identity and time

Every record has a UUIDv7 primary key. Timestamps are stored as RFC 3339 strings in UTC. A calendar event keeps its wall-clock time and its IANA zone, so travel and daylight-saving transitions stay meaningful. A date without a time, such as an all-day event, is a floating date.

## Optimistic concurrency

Mutable records carry a `revision` counter. An update may state the revision it expects; a mismatch is a conflict rather than a silent overwrite. The agent's undo uses the same mechanism, so an undo is refused when the record has moved on.

## Deletion and archiving

Tasks are soft-deleted with `deleted_at` and can be restored (`tasks.restore`). `areas`, `habits`, `routines`, and `task_templates` use an `archived` integer flag (`project_templates` uses `archived_at`). `projects`, `goals`, `events`, `notes`, and `wellbeing_checkins` perform hard `DELETE`.

## The main tables

Schema v16 defines 46 tables plus 1 FTS5 virtual table (`task_search`):

| Table(s) | Holds |
|---|---|
| `schema_migrations`, `idempotency_keys` | Applied schema versions and request idempotency cache |
| `areas` | Life areas (`archived` flag) |
| `projects`, `project_templates`, `goals`, `key_results`, `goal_progress_history` | Projects, templates, goals, OKRs, and progress snapshots |
| `tasks`, `task_dependencies`, `task_templates`, `milestones`, `task_search` | Tasks, dependencies, templates, milestones, and FTS5 index |
| `recurrence_exceptions` | Single occurrences of a recurring task, skipped or moved |
| `events`, `calendar_sources`, `imported_records` | Calendar events, sources, and external import tracking |
| `availability_rules`, `protected_blocks`, `time_budgets` | Working hours, protected time, and weekly area budgets |
| `habits`, `habit_logs`, `routines`, `routine_steps` | Habits, daily logs, and ordered routines |
| `focus_sessions`, `focus_pauses`, `focus_presets` | Focus sessions, pauses, and timer presets |
| `notes`, `entity_links` | Markdown notes and links between records |
| `reviews`, `guided_reviews`, `goal_cycles`, `goal_cycle_items` | Reviews, guided review drafts, and goal cycles |
| `saved_filters`, `saved_searches` | Reusable task filters and saved search queries |
| `managed_attachments` | Managed file attachments with SHA-256 checksums and inlined `content BLOB` (8 MiB limit) |
| `action_history`, `action_changes` | Activity ledger and undo ledger |
| `agent_permissions`, `agent_proposals`, `security_audit` | Actor permissions, reviewable proposals, and security audit log |
| `automation_rules`, `automation_runs`, `automation_schedules` | Local automation triggers, runs, and schedules |
| `career_applications`, `interview_events`, `learning_tracks`, `learning_sessions`, `certifications` | Career pipeline, interviews, learning tracks, sessions, and certifications |
| `finance_commitments`, `savings_goals`, `wellbeing_checkins`, `rest_intentions`, `administration_items` | Finance commitments, savings, wellbeing, rest, and personal admin |
| `reminder_delivery`, `notification_authorization` | Reminder delivery leases and notification permission state |
| `integration_accounts`, `remote_mcp_clients` | Integration account status and hashed remote MCP bearer tokens |
| `settings`, `keyboard_shortcuts` | Preferences and shortcut bindings |

## Search

`task_search` is an FTS5 virtual table inside the database (encrypted with everything else) updated on task create, update, and hard delete. `Store::search` currently queries records using SQL `LIKE` rather than FTS5 `MATCH`.

## Migrations

Each migration is guarded by the schema version and is additive. A database from an earlier version opens on a newer build and is migrated in place inside a transaction. The backup and restore paths use the SQLCipher export path when the database is encrypted, because the online backup API cannot read an encrypted database.

## Protection

The database is plain SQLite by default. `life db encrypt` converts it to a SQLCipher database whose key is derived from a passphrase with Argon2id (or supplied directly via `LIFE_DB_KEY`). The `<db_filename>.encryption.json` state file beside the database records `protection`, `salt`, `params`, `cipher`, `page_size`, `hmac`, and `kdf`, never the key and never the passphrase.
