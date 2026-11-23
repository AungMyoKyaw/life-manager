# Test plan

## The suites

| Suite | Command | Scope |
|---|---|---|
| Rust domain | `cargo test --workspace --offline` | The store, the migrations, GPUI views, and every action |
| MCP contract | `cargo test -p life-cli` | The published tool catalog and its schemas |
| GPUI Desktop | `cargo test -p life-gpui` | Headless GPUI rendering across all 18 views and SQLCipher unlock |
| CLI and MCP | `python3 qa/*_e2e.py` | The release binary against disposable databases |
| Design | `python3 design/scripts/validate_design.py` | The FigCraft master and its exports |

## What the CLI and MCP suites cover

| Script | Checks |
|---|---|
| `full_e2e.py` | The base flows: tasks, projects, events, habits, focus, backups, restore, the MCP handshake |
| `figcraft_parity_e2e.py` | The feature extensions: templates, saved views, key results, links, availability, import, reports |
| `v9_extensions_e2e.py` | The schema history: attachments, domain records, ICS round trips, the export |
| `v11_reminders_habits_e2e.py` | Reminder claiming and delivery, snoozing, flexible habit cadences |
| `parity_v15_e2e.py` | Recurrence exceptions, planning rules, focus modes, executable views, reports, agent policy, automation schedules |
| `release_e2e.py` | The life-area trackers, the workflow gaps, CSV import, shortcuts, diagnostics, the security audit, the remote MCP server |
| `encryption_e2e.py` | Encryption end to end: the backup, the ciphertext, the wrong passphrase, the encrypted backup, the decrypt |

Every one of them runs against a disposable SQLite file through the compiled release binary, so it exercises the same code path the desktop app uses.

## The design gate

`validate_design.py` must report `PASS`. It checks the frame count against the inventory, the exported preview for every board, one unique preview per board, the token bindings, the font availability, the layer containment and the alignment of every visual group.

## The rules

- A new action needs a Rust test, and a CLI or MCP assertion if it changes behaviour.
- A new screen needs a headless GPUI test in `crates/life-gpui/tests/gpui_workspace.rs` that exercises the interaction.
- A new board needs the master rebuilt, the previews re-rendered and the design validated.
- A test that only checks that a page loads is not a test.

## What is not covered

There is no VoiceOver session and no full 224-board pixel diff. The browser suite runs in Chromium, not Safari. The reminder delivery path is verified through the CLI, but macOS Notification Center authorisation is not exercised. These are the open edges, and they are listed as such in `docs/FIGCRAFT_PARITY_AUDIT.md`.
