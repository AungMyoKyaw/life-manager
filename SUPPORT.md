# Support

## Before asking

Most questions are answered by the documentation in this repository:

| Question | Where |
|---|---|
| How do I install it? | `README.md` |
| What can the CLI do? | `docs/CLI_AND_MCP.md` |
| How does it work under the hood? | `docs/ARCHITECTURE.md` |
| How is the data stored? | `docs/DATA_MODEL.md` |
| How do I encrypt the database? | `SECURITY.md` and `life db --help` |
| Why does macOS say the app is damaged? | `RELEASE_CHECKLIST.md` — the build is not notarized yet |

## Common problems

**The app will not open on another Mac.** The build is ad-hoc signed and Gatekeeper rejects it. Right-click the app and choose Open, or build a signed and notarized copy yourself with `./scripts/release.sh all`.

**The CLI says it needs a passphrase.** The database is encrypted. Set `LIFE_DB_PASSPHRASE` or pass `--passphrase`.

**The reminder worker is not sending anything.** Check that reminders are enabled, that macOS has allowed notifications for Life Manager in System Settings, and that the current time is outside your quiet hours. `life reminder status` shows whether the worker is installed.

**The MCP server refuses a tool call.** The argument does not match the tool schema. `life mcp tools` prints every tool with its parameters and its required fields.

## Asking a question

Open an issue with what you did, what you expected, and what happened, and include the output of:

```bash
life --json call system.status --payload '{}'
```

Do not paste the contents of your database, a passphrase or a token.
