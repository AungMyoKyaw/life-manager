# Security

## The threat model

Life Manager holds a single person's private records: tasks, calendar events, journal entries, wellbeing reflections, financial commitments, focus sessions and time records. They are stored in one SQLite file on the user's machine.

The realistic risks are:

- Another program or a person reading that file.
- A backup left in an unprotected place.
- An agent given more access than the user intended.
- A release build that a stranger cannot verify.

## What the app does

- **The database can be encrypted at rest.** `life db encrypt --passphrase "..."` converts it to a SQLCipher database. The key is derived from the passphrase with Argon2id and is never written to the database, to the state file or to the repository. Headless use reads `LIFE_DB_PASSPHRASE`.
- **The database is not encrypted by default.** A plain database is a plain SQLite file. Say so; do not imply otherwise.
- **Backups are verified.** Every backup is integrity checked before it is accepted, and a verified snapshot is taken before an encryption change or a restore.
- **Agent actions are attributable.** Every mutation records the actor, and the agent permission model can refuse an action pattern for an actor.
- **The remote MCP server is local by default.** It binds the loopback interface, requires a bearer token, validates every argument, and refuses a non-loopback bind unless `--allow-remote` is given. Its transport is plain HTTP, so it must not be exposed to a network you do not control.
- **The launchd worker is opt in.** It is only installed after the user has been told what it does.

## What the app does not do

- It does not encrypt the database by default.
- It does not sign the release build with a Developer ID identity yet, so the shipped artifact is ad-hoc signed and Gatekeeper rejects it on other machines. **Do not distribute it.** See `RELEASE_CHECKLIST.md`.
- It does not have end-to-end encryption, two-factor authentication or a sync service. There is no account.
- It does not send telemetry, crash reports or analytics.

## Reporting a vulnerability

If private vulnerability reporting is enabled on the hosting repository, use that channel. Otherwise, contact the maintainer through the repository's documented support channel. Include the version, platform, steps to reproduce, and impact.

Please do not open a public issue for an unfixed vulnerability.

## What not to include

Never paste the contents of a user's database, a passphrase, a master key, a bearer token, or a real journal entry into a report. Use a disposable database.
