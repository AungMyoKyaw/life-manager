# The CLI and the MCP server

Both interfaces call the same Rust domain as the desktop app. There is one database, one set of rules and one audit trail.

## The command line

```bash
life today --json
life task create --title "Prepare the review" --priority 2
life task list --json
life calendar propose --date 2026-05-01
life db status
life db encrypt --passphrase "..."
life mcp serve --stdio
life mcp remote --port 0
life reminder status
```

Every command prints a JSON envelope with `ok` and `data`, or `ok: false` and a stable error code:

| Code | Meaning |
|---|---|
| `VALIDATION_ERROR` | The arguments were wrong |
| `NOT_FOUND` | The record does not exist |
| `CONFLICT` | The change collided with newer work |
| `DATABASE_ERROR` | SQLite refused the operation |
| `IO_ERROR` | A file could not be read or written |
| `JSON_ERROR` | The payload was not valid JSON |

## The MCP server

`life mcp serve --stdio` speaks the Model Context Protocol over standard input and output. `life mcp remote --port 0` serves the same tool catalog over HTTP on the loopback interface.

The catalog is generated from one table, `crates/life-cli/src/action_spec.rs`, which holds every action, its description, its parameters and its required fields. Adding an action there adds it to the CLI and to the MCP server together.

### The tool contract

The tool names published by earlier releases keep working. `crates/life-cli/tests/mcp_contract.rs` pins every previously published name and proves that:

- every published name still resolves,
- no two actions publish the same name,
- every tool declares a description and an object input schema,
- no tool declares an empty parameter set,
- every required parameter is also declared,
- and every write action requires at least one parameter.

### The remote server

- It binds the loopback interface. A non-loopback bind needs `--allow-remote`, because the transport is plain HTTP.
- It requires a bearer token. Tokens are stored only as hashes and can be revoked.
- It refuses an unknown or revoked token, and a request with no token.
- It validates every argument against the action specification before running it.
- Every remote write is recorded in the security audit with the client name.

## Idempotency and attribution

Any mutation accepts an optional `request_id`. Replaying the same request with the same payload returns the original result instead of writing twice; reusing the key with a different payload is refused.

Every mutation records the actor. The agent permission model can refuse an action pattern for an actor, and an agent can propose a change that a human then approves or rejects with a recorded reason.

## The database

```bash
life db status                 # how the database is protected
life db encrypt --passphrase  # a verified plain backup is taken first
life db decrypt --confirm      # the database becomes readable on disk
```

Headless use reads the passphrase from `LIFE_DB_PASSPHRASE`. The passphrase is never written to the database, the state file or the repository.
