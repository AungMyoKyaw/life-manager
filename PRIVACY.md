# Privacy

Life Manager is a local-first desktop app for a single person. There is no account, no subscription and no server that belongs to the project.

## What stays on your Mac

Everything you enter — tasks, projects, goals, calendar events, habits, notes, journal entries, focus sessions, wellbeing reflections, financial reminders, attachments and time records — is written to one SQLite database on your own machine:

    ~/Library/Application Support/LifeManager/life.sqlite

(Override the database path with `--db <PATH>` or `LIFE_DB`.)

The app does not upload data by default. It works with the network disconnected. Optional encryption at rest is available via `life db encrypt` (see `SECURITY.md`).

## The network

Life Manager makes no network request of its own. Two areas relate to external connectivity:

- **External calendar and repository integrations.** Off by default. Currently `integrations.configure` only checks `LIFE_*` environment variables and records configuration status in `integration_accounts`; macOS Keychain token storage, OAuth token exchange, EventKit calendar reading, and live CalDAV/Google/GitHub network sync are not yet implemented.
- **The remote MCP server.** Off until you start it (`life mcp remote`), binds to `127.0.0.1` by default (requiring `--allow-remote` for non-loopback addresses), and requires a bearer token stored only as a SHA-256 hash.

## What the app reads from the system

- macOS Notification Center permission state, to report whether reminders can be delivered.
- Your own `launchd` user agent directory, to install the optional background reminder worker.

## Telemetry

There is none. No analytics, no crash reporting, no update check.

## Exports and deletion

`life export` writes a JSON snapshot of the 16 core tables (`areas`, `projects`, `goals`, `tasks`, `events`, `habits`, `habit_logs`, `notes`, `focus_sessions`, `settings`, `managed_attachments` [metadata only, without `content` BLOBs], `career_applications`, `learning_tracks`, `learning_sessions`, `finance_commitments`, `wellbeing_checkins`, `reminder_delivery`); newer v12–v16 tables (such as `milestones`, `routines`, `reviews`, `goal_cycles`, `savings_goals`, `certifications`, `rest_intentions`, `administration_items`) are included in full SQLite backups (`life backup create`) rather than `life export`. Deleting the database file removes everything.

## Permissions the app requests

| Permission | When | Why |
|---|---|---|
| Notifications | You turn reminders on | Delivering a reminder you asked for |
| Calendar access | You connect Apple Calendar | Reading events you chose to show alongside your own |
| Files and folders | You attach a file | Storing the attachment next to your database |

## Contact

For issues and questions, use the issue tracker for the repository where you obtained this project.
