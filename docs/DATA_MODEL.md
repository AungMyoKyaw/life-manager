# The data model

One SQLite database holds everything. The schema version is in `schema_migrations` and the live value is reported by `life --json call system.status --payload '{}'`.

## Identity and time

Every record has a UUIDv7 primary key. Timestamps are stored as RFC 3339 strings in UTC. A calendar event keeps its wall-clock time and its IANA zone, so travel and daylight-saving transitions stay meaningful. A date without a time, such as an all-day event, is a floating date.

## Optimistic concurrency

Mutable records carry a `revision` counter. An update may state the revision it expects; a mismatch is a conflict rather than a silent overwrite. The agent's undo uses the same mechanism, so an undo is refused when the record has moved on.

## Soft deletion

Tasks, areas and the personal life-area records are archived with `deleted_at` rather than erased. The archive is reachable, and a restore brings the record back with its history intact.

## The main tables

| Table | Holds |
|---|---|
| `areas` | Life areas |
| `projects`, `goals`, `key_results` | Projects, goals and their measurable outcomes |
| `tasks`, `milestones` | Tasks with subtasks, dependencies, recurrence and milestones |
| `recurrence_exceptions` | A single occurrence of a recurring task, skipped or moved |
| `events`, `calendar_sources` | Calendar events and the sources they come from |
| `availability_rules`, `protected_blocks`, `time_budgets` | Working hours, protected time and weekly budgets |
| `habits`, `habit_logs`, `routines`, `routine_steps` | Habits and routines |
| `focus_sessions`, `focus_pauses`, `focus_presets` | Focus sessions, their pauses and their presets |
| `notes`, `entity_links` | Notes and the links between any two records |
| `reviews`, `guided_reviews`, `goal_cycles` | Reviews, guided review drafts and planning periods |
| `attachments` | Managed file attachments, checksummed and never inlined |
| `action_history`, `action_changes` | The activity ledger and the undo ledger |
| `agent_permissions`, `agent_proposals` | Who may do what, and what is waiting for review |
| `security_audit` | Privileged operations, in order |
| `settings`, `keyboard_shortcuts` | Preferences and the shortcut map |

## Search

Task titles and descriptions are indexed in an FTS5 virtual table and kept in step with the task table. The index lives inside the database, so it is encrypted with everything else.

## Migrations

Each migration is guarded by the schema version and is additive. A database from an earlier version opens on a newer build and is migrated in place inside a transaction. The backup and restore paths use the SQLCipher export path when the database is encrypted, because the online backup API cannot read an encrypted database.

## Protection

The database is plain SQLite by default. `life db encrypt` converts it to a SQLCipher database whose key is derived from a passphrase with Argon2id. The state file beside the database records the salt and the parameters, never the key and never the passphrase.
