# Contributing to Life Manager

Life Manager is a local-first app for one person. It runs entirely on your own machine, keeps one SQLite database, and exposes the same domain through a desktop app, a CLI and an MCP server.

## Before you start

Read these, in order:

- `README.md` — overview, quickstart, and build instructions
- `docs/ARCHITECTURE.md` — crate boundaries, SQLite concurrency, and security model
- `docs/DATA_MODEL.md` — SQLite schema v16 and encryption model
- `docs/CLI_AND_MCP.md` — the typed CLI and MCP catalog

The design master `design/life-manager-master.figcraft` is the source of truth for every screen. If you change a screen, change the master first, regenerate it, and then change the app.

## The rules that are not negotiable

1. **One source of truth.** The Rust domain in `crates/life-core` owns the data. The desktop app, the CLI and the MCP server all call the same `Store::call` facade. Never add a second place where a task, an event or a goal can be written.
2. **The database belongs to its operator.** Data stays local unless the operator enables an external integration. No telemetry, no crash reporting, no update check.
3. **Never fake a state.** If an integration is not configured, the interface says so. If a credential is missing, the error names it. If something is not implemented, the documentation says that.
4. **Every action is typed.** A new domain action goes into `crates/life-cli/src/action_spec.rs` with its parameters, its required fields and its description, and the MCP catalog picks it up from there.
5. **Every action is tested.** A new action needs a Rust test, and if it changes behaviour a GPUI workspace test and a CLI or MCP assertion.

## Running the checks

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --offline
python3 qa/full_e2e.py
python3 qa/figcraft_parity_e2e.py
python3 qa/v9_extensions_e2e.py
python3 qa/v11_reminders_habits_e2e.py
python3 qa/parity_v15_e2e.py
python3 qa/release_e2e.py
python3 qa/encryption_e2e.py
python3 qa/board_coverage.py
python3 design/scripts/validate_design.py
```

Or use the `Makefile` shortcuts (`make check`, `make test`, `make lint`, `make validate-design`).

## Changing the design

```bash
python3 design/scripts/build_figcraft.py     # rebuild the board JSON
python3 design/scripts/bind_tokens.py        # bind the design tokens
python3 design/scripts/rebuild_master.py     # write the native .figcraft
python3 design/scripts/render_previews.py    # export the previews and contact sheets
python3 design/scripts/validate_design.py    # the gate
```

`validate_design.py` must report `PASS`, with zero alignment problems and one unique preview per board.

## Style

Rust is formatted by `cargo fmt` and linted by Clippy with `-D warnings`. Comments explain why, not what.

## Reporting a problem

Open an issue with what you did, what you expected, and what happened. If it involves the database, include the schema version from `life --json call system.status --payload '{}'` and **never** paste the contents of the user's database.

## Security problems

Read `SECURITY.md`. Do not open a public issue for a vulnerability.
