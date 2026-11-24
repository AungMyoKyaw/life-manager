# The CLI and the MCP server

Both interfaces call the same Rust domain as the desktop app. There is one database, one set of rules and one audit trail.

## The command line

Global flags: `--db <PATH>` (or `LIFE_DB`), `--actor <ACTOR>` (default `user`), `--json`.

```bash
life today --json
life dashboard --json
life stats --json
life task create --title "Prepare the review" --priority 2
life task list --json
life project|goal|event|note|area <list|get|create|update|complete|delete|restore>
life areas
life calendar list|conflicts|propose --date 2026-05-01
life habit list|create|log|delete
life focus list|start|stop
life agent history
life backup create|verify|restore
life search <query>
life export --out <path>
life call <action> [--input <file> | --payload <json>]
life db status|encrypt|decrypt
life mcp serve --stdio
life mcp tools
life mcp remote --port 0
life reminder check|preview|install|uninstall|status
```

With `--json`, commands print `{"ok": true, "data": ...}` to stdout; without `--json`, they print raw pretty-printed JSON (`life mcp tools` always prints a raw JSON array, and `life export --out <path>` prints only the output path). Errors always print `{"ok": false, "error": {"code": ..., "message": ...}}` to **stderr**:

| Code | Meaning |
|---|---|
| `VALIDATION_ERROR` | The arguments were wrong |
| `NOT_FOUND` | The record does not exist |
| `CONFLICT` | The change collided with newer work |
| `DATABASE_ERROR` | SQLite refused the operation |
| `IO_ERROR` | A file could not be read or written |
| `JSON_ERROR` | The payload was not valid JSON |
| `UNAUTHORIZED` | Missing or invalid remote MCP bearer token (HTTP 401) |
| `FORBIDDEN` | Revoked token or blocked remote host (HTTP 403) |
| `INVALID_JSON` | Malformed JSON body on `POST /mcp` (HTTP 400) |

## The MCP server

`life mcp serve --stdio` speaks the Model Context Protocol over standard input and output. `life mcp remote --port 0` serves the same tool catalog over HTTP on the loopback interface.

The catalog is generated from one table, `crates/life-cli/src/action_spec.rs` (263 actions; `Store::dispatch` also handles `goals.delete` and `projects.delete` directly), which holds every action, its description, its parameters and its required fields. Adding an action there adds it to the CLI and to the MCP server together.

### The tool contract

The tool names published by earlier releases keep working. `crates/life-cli/tests/mcp_contract.rs` pins every previously published name and proves that:

- every published name still resolves,
- no two actions publish the same name,
- every tool declares a description and an object input schema,
- no tool declares an empty parameter set,
- every required parameter is also declared,
- and every write action requires at least one parameter.

### The remote server

- Exposes `GET /health` and `POST /mcp`.
- Binds `127.0.0.1` by default (`--host` to override). A non-loopback bind requires `--allow-remote`, because the transport is plain HTTP.
- Requires a bearer token. Tokens are stored only as hashes (`remote_mcp_clients`) and can be revoked.
- Refuses an unknown or revoked token, and a request with no token.
- Validates every argument against the action specification before running it.
- For encrypted databases, set `LIFE_DB_PASSPHRASE` or `LIFE_DB_KEY` (`mcp remote` does not accept `--passphrase`).
- Writes `security.audit.log` entries with the client name for `.create`, `.update`, `.delete`, `.add`, `.apply`, `.set`, `.log`, `.run`, and `.complete` action suffixes.

## Idempotency and attribution

Any mutation accepts an optional `request_id`. Replaying the same request with the same payload returns the original result instead of writing twice; reusing the key with a different payload is refused.

Every mutation records the actor. The agent permission model can refuse an action pattern for an actor, and an agent can propose a change that a human then approves or rejects with a recorded reason.

## The database

```bash
life db status                 # how the database is protected
life db encrypt --passphrase  # a verified plain backup is taken first
life db decrypt --confirm      # the database becomes readable on disk
```

Headless use reads `LIFE_DB_PASSPHRASE` or `LIFE_DB_KEY`. Neither is ever written to the database, the state file or the repository.
