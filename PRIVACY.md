# Privacy

Life Manager is a local-first desktop app for a single person. There is no account, no subscription and no server that belongs to the project.

## What stays on your Mac

Everything you enter — tasks, projects, goals, calendar events, habits, notes, journal entries, focus sessions, wellbeing reflections, financial reminders, attachments and time records — is written to one SQLite database on your own machine:

    ~/Library/Application Support/LifeManager/life.sqlite

The app does not upload data by default. It works with the network disconnected.

## The network

Life Manager makes no network request of its own. Two optional features can:

- **External calendar and repository integrations.** These are off until you configure them. When enabled, data is exchanged with the provider you selected. Google Calendar and GitHub read their client credentials from environment variables at run time and store the resulting access token in your macOS Keychain, never in the database. Apple Calendar reads the calendar database that macOS already exposes to your user. CalDAV reads the server you point it at.
- **The remote MCP server.** It is off until you start it, it binds to the loopback interface by default, and it requires a bearer token that is stored only as a SHA-256 hash.

## What the app reads from the system

- macOS Notification Center permission state, to tell you honestly whether reminders can be delivered.
- EventKit, only if you grant access to Apple Calendar.
- Your own launchd user agent directory, to install the optional background reminder worker.

## Telemetry

There is none. No analytics, no crash reporting, no update check.

## Exports and deletion

Every record is in your database and in your exports. `life export` writes a JSON copy, and the app can restore from a verified SQLite backup. Deleting the database file removes everything.

## Permissions the app requests

| Permission | When | Why |
|---|---|---|
| Notifications | You turn reminders on | Delivering a reminder you asked for |
| Calendar access | You connect Apple Calendar | Reading events you chose to show alongside your own |
| Files and folders | You attach a file | Storing the attachment next to your database |

## Contact

For issues and questions, use the issue tracker for the repository where you obtained this project.
