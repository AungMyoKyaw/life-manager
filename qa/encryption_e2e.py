#!/usr/bin/env python3
"""End-to-end check of the optional database encryption, through the release CLI.

The passphrase is never written to the repository. The check uses a disposable
database and a passphrase that exists only for the length of this script.
"""
from __future__ import annotations
import json, os, pathlib, subprocess, tempfile

BIN = os.environ.get("LIFE_CLI", "/tmp/life-manager-target/release/life")
passed: list[str] = []
PASSPHRASE = "correct horse battery staple"


def check(message, condition):
    assert condition, message
    passed.append(message)


def run(args, env=None):
    environment = dict(os.environ)
    environment.pop("LIFE_DB_PASSPHRASE", None)
    environment.pop("LIFE_DB_KEY", None)
    if env:
        environment.update(env)
    return subprocess.run(
        [BIN] + args, capture_output=True, text=True, timeout=60, env=environment
    )


def op(db, action, payload=None, env=None):
    result = run(
        ["--db", db, "--json", "call", action, "--payload", json.dumps(payload or {})],
        env,
    )
    if result.returncode:
        raise AssertionError(f"{action} exit={result.returncode}\n{result.stdout}\n{result.stderr}")
    response = json.loads(result.stdout)
    assert response["ok"], response
    return response["data"]


with tempfile.TemporaryDirectory(prefix="life-enc-") as tmp:
    db = str(pathlib.Path(tmp) / "life.sqlite")

    # 1. A plain database reports no protection.
    op(db, "tasks.create", {"title": "A private journal entry"})
    status = run(["--db", db, "--json", "db", "status"])
    document = json.loads(status.stdout)["data"]
    check("A plain database reports no protection", document["protection"] == "none")
    check("A plain database has a SQLite header", document["file_header"].startswith("SQLite format 3"))

    # 2. The passphrase is required.
    missing = run(["--db", db, "--json", "db", "encrypt"])
    check("Encrypting without a passphrase is refused", missing.returncode != 0)
    check(
        "The refusal explains what is needed",
        "passphrase" in (missing.stdout + missing.stderr).lower(),
    )

    # 3. Encrypting takes a verified plain backup first.
    encrypted = run(
        ["--db", db, "--json", "db", "encrypt", "--passphrase", PASSPHRASE]
    )
    assert encrypted.returncode == 0, encrypted.stdout + encrypted.stderr
    document = json.loads(encrypted.stdout)["data"]
    check("The database is encrypted", document["encrypted"] is True)
    check("The mode is a passphrase", document["mode"] == "passphrase")
    backup = pathlib.Path(document["pre_encryption_backup"])
    check("A pre-encryption backup was taken", backup.exists())

    # 4. The backup is a readable plain database with the same row.
    import sqlite3

    with sqlite3.connect(backup) as connection:
        rows = connection.execute("SELECT title FROM tasks").fetchall()
    check("The backup holds the original row", rows == [("A private journal entry",)])

    # 5. The live file is ciphertext.
    raw = pathlib.Path(db).read_bytes()
    check("The plaintext is not in the file", b"A private journal entry" not in raw)
    check("The file is not a plain SQLite file", not raw.startswith(b"SQLite format 3"))

    # 6. Without the passphrase the database cannot be read.
    blocked = run(["--db", db, "--json", "call", "tasks.list", "--payload", "{}"])
    check("The database cannot be read without the passphrase", blocked.returncode != 0)
    check(
        "The refusal names the missing passphrase",
        "passphrase" in (blocked.stdout + blocked.stderr).lower(),
    )

    # 7. With the passphrase it reads correctly.
    with_passphrase = {"LIFE_DB_PASSPHRASE": PASSPHRASE}
    tasks = op(db, "tasks.list", {}, with_passphrase)
    check("The database reads with the passphrase", len(tasks) == 1)
    check("The row survives encryption", tasks[0]["title"] == "A private journal entry")

    # 8. A wrong passphrase is refused.
    wrong_run = run(
        ["--db", db, "--json", "call", "tasks.list", "--payload", "{}"],
        {"LIFE_DB_PASSPHRASE": "not the passphrase"},
    )
    check("A wrong passphrase cannot open the database", wrong_run.returncode != 0)

    # 9. The protection state can be verified without opening the database.
    verified = op(db, "encryption.verify_passphrase", {"passphrase": PASSPHRASE}, with_passphrase)
    check("The correct passphrase verifies", verified["matches"] is True)
    unverified = op(
        db, "encryption.verify_passphrase", {"passphrase": "nope"}, with_passphrase
    )
    check("A wrong passphrase does not verify", unverified["matches"] is False)

    # 10. An encrypted backup is created and is itself encrypted.
    backup_result = op(db, "backup.create", {}, with_passphrase)
    check("The encrypted backup is marked encrypted", backup_result["encrypted"] is True)
    backup_bytes = pathlib.Path(backup_result["path"]).read_bytes()
    check("The encrypted backup is ciphertext", not backup_bytes.startswith(b"SQLite format 3"))

    # 11. Decrypting restores a readable database.
    decrypted = run(
        [
            "--db",
            db,
            "--json",
            "db",
            "decrypt",
            "--passphrase",
            PASSPHRASE,
            "--confirm",
        ]
    )
    assert decrypted.returncode == 0, decrypted.stdout + decrypted.stderr
    document = json.loads(decrypted.stdout)["data"]
    check("The database is decrypted", document["encrypted"] is False)
    raw = pathlib.Path(db).read_bytes()
    check("The file is a plain SQLite file again", raw.startswith(b"SQLite format 3"))
    tasks = op(db, "tasks.list", {})
    check("The row survives decryption", len(tasks) == 1)

    # 12. Decrypting without a confirmation is refused.
    again = run(["--db", db, "--json", "db", "decrypt", "--passphrase", PASSPHRASE])
    check("Decrypting without a confirmation is refused", again.returncode != 0)

result = {"suite": "encryption", "count": len(passed), "passed": passed}
pathlib.Path("qa/encryption_result.json").write_text(json.dumps(result, indent=2))
print(json.dumps(result, indent=2))
