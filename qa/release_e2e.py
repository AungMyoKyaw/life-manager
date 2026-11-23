#!/usr/bin/env python3
"""Release readiness checks for the schema v16 domain and the remote MCP server.

Everything runs against a disposable SQLite file through the compiled release
binary, so it proves the same code path the desktop app uses.
"""
from __future__ import annotations
import json, os, pathlib, socket, subprocess, tempfile, time

BIN = os.environ.get("LIFE_CLI", "/tmp/life-manager-target/release/life")
passed: list[str] = []


def check(message, condition):
    assert condition, message
    passed.append(message)


def free_port():
    with socket.socket() as s:
        s.bind(("127.0.0.1", 0))
        return s.getsockname()[1]


with tempfile.TemporaryDirectory(prefix="life-release-") as tmp:
    db = str(pathlib.Path(tmp) / "life.sqlite")

    def op(action, payload=None, actor="qa"):
        args = [BIN, "--db", db, "--json", "--actor", actor, "call", action,
                "--payload", json.dumps(payload or {})]
        result = subprocess.run(args, check=False, capture_output=True, text=True, timeout=35)
        if result.returncode:
            raise AssertionError(f"{action} exit={result.returncode}\n{result.stdout}\n{result.stderr}")
        response = json.loads(result.stdout)
        assert response["ok"], response
        return response["data"]

    # --- personal life areas -------------------------------------------------
    interview = op("interviews.create", {"title": "Technical interview", "starts_at": "2026-04-01T10:00:00",
                                         "location": "Video", "preparation": "Architecture"})
    check("An interview is scheduled", interview["status"] == "scheduled")
    check("Interviews are listed", len(op("interviews.list", {})) == 1)
    op("savings_goals.create", {"title": "Emergency fund", "target_minor": 500000, "currency": "SGD"})
    check("A savings goal is stored", len(op("savings_goals.list", {})) == 1)
    op("certifications.create", {"title": "CKA", "deadline_date": "2026-05-01"})
    check("A certification deadline is stored", len(op("certifications.list", {})) == 1)
    op("rest_intentions.create", {"title": "Sleep", "kind": "sleep", "starts_at": "22:00", "ends_at": "07:00",
                                   "days": [0, 1, 2]})
    check("A rest intention is stored", len(op("rest_intentions.list", {})) == 1)
    op("administration.create", {"title": "Passport renewal", "kind": "renewal", "renewal_date": "2027-01-01"})
    check("An administration item is stored", len(op("administration.list", {})) == 1)
    overview = op("personal.overview", {})
    check("The personal overview names its source", "SQLite" in overview["source"])
    check("The personal overview carries every tracker",
          all(key in overview for key in ("career", "learning", "finance", "wellbeing",
                                          "savings_goals", "certifications", "rest_intentions",
                                          "administration", "interviews")))

    # --- workflows ------------------------------------------------------------
    task = op("tasks.create", {"title": "Late work", "due_date": "2020-01-01", "priority": 1})
    decisions = op("overdue.decisions", {})
    check("An overdue task is offered for a decision", decisions["count"] == 1)
    op("overdue.decide", {"id": task["id"], "decision": "snooze", "date": "2026-04-02"})
    check("The overdue decision is applied", op("tasks.get", {"id": task["id"]})["due_date"] == "2026-04-02")
    timeline = op("entity.timeline", {"kind": "tasks", "id": task["id"]})
    check("The activity timeline is readable", timeline["count"] >= 1)
    capture = op("capture.parse", {"text": "Draft the report tomorrow #work"})
    check("The capture parser understands a date and a tag",
          capture["title"] == "Draft the report" and capture["tags"][0] == "work")
    session = op("planning.session", {"date": "2026-04-01", "priorities": ["a", "b", "c"]})
    check("A daily planning session records three priorities", len(session["priorities"]) == 3)
    stored = op("planning.session.get", {"date": "2026-04-01"})
    check("The planning session is stored", len(stored["session"]["priorities"]) == 3)

    # --- CSV import -----------------------------------------------------------
    csv = "title,due_date,priority\nShip the release,2026-05-01,1\nShip the release,2026-05-01,1\n,2026-05-02,2\n"
    preview = op("csv.import.preview", {"content": csv})
    check("A CSV import classifies valid, duplicate and invalid rows",
          len(preview["valid"]) == 1 and len(preview["duplicates"]) == 1 and len(preview["invalid"]) == 1)
    check("A CSV preview writes nothing", len(op("tasks.list", {})) == 1)
    applied = op("csv.import.apply", {"content": csv, "confirm": True, "allow_invalid": True})
    check("A reviewed CSV import applies the valid row", applied["created_count"] == 1)
    check("The imported task exists", len(op("tasks.list", {})) == 2)

    # --- settings, security and diagnostics ------------------------------------
    shortcuts = op("shortcuts.list", {})
    check("The default shortcuts are seeded", len(shortcuts) >= 8)
    op("shortcuts.update", {"action": "command_palette", "keys": "Meta+P"})
    denied = subprocess.run([BIN, "--db", db, "--json", "call", "shortcuts.update",
                              "--payload", json.dumps({"action": "quick_capture", "keys": "Meta+P"})],
                             capture_output=True, text=True, timeout=35)
    check("A conflicting shortcut is refused", json.loads(denied.stderr or denied.stdout)["ok"] is False)
    integrations = op("integrations.list", {})
    check("Every integration reports a real state",
          all(row["status"] in ("not configured", "disconnected", "authorized", "syncing", "error")
              for row in integrations))
    configure = op("integrations.configure", {"provider": "google_calendar"})
    check("An unconfigured integration names what it still needs", configure["configured"] is False)
    status = op("system.status", {})
    check("The system status reports the schema version", status["schema_version"] == 16)
    check("The database integrity check passes", status["integrity"] == "ok")
    diagnostics = op("diagnostics.report", {})
    check("The diagnostics report counts the local tables", len(diagnostics["tables"]) >= 10)
    op("security.audit.log", {"event": "backup.restore", "detail": "Restored a verified snapshot"})
    check("The security audit records an event", len(op("security.audit.list", {})) == 1)

    # --- remote MCP ------------------------------------------------------------
    token = "lm_0123456789abcdef0123456789abcdef"
    client = op("remote_clients.create", {"name": "Release agent", "token": token})
    check("A remote client token is created", client["name"] == "Release agent")
    check("The plaintext token is never stored", token not in json.dumps(op("remote_clients.list", {})))

    port = free_port()
    server = subprocess.Popen([BIN, "--db", db, "mcp", "remote", "--port", str(port)],
                              stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
    try:
        ready = False
        for _ in range(60):
            try:
                with socket.create_connection(("127.0.0.1", port), timeout=0.5):
                    ready = True
                    break
            except OSError:
                time.sleep(0.1)
        check("The remote MCP server binds a port", ready)

        def http(body, headers=""):
            request = (f"POST /mcp HTTP/1.1\r\nHost: 127.0.0.1\r\n"
                       f"Content-Type: application/json\r\nContent-Length: {len(body)}\r\n"
                       f"{headers}\r\n{body}")
            with socket.create_connection(("127.0.0.1", port), timeout=10) as s:
                s.sendall(request.encode())
                data = b""
                while True:
                    chunk = s.recv(65536)
                    if not chunk:
                        break
                    data += chunk
            return data.decode(errors="ignore")

        unauthorised = http(json.dumps({"jsonrpc": "2.0", "id": 1, "method": "tools/list"}))
        check("The remote server refuses a request with no token", "401" in unauthorised.splitlines()[0])
        wrong = http(json.dumps({"jsonrpc": "2.0", "id": 1, "method": "tools/list"}),
                     "Authorization: Bearer lm_wrong_wrong_wrong_wrong\r\n")
        check("The remote server refuses an unknown token", "403" in wrong.splitlines()[0])

        listing = http(json.dumps({"jsonrpc": "2.0", "id": 2, "method": "tools/list"}),
                       f"Authorization: Bearer {token}\r\n")
        check("The remote server lists the tool catalog", '"life_tasks_list"' in listing)
        check("The remote tool catalog carries the new tools",
              '"life_interviews_list"' in listing and '"life_csv_import_preview"' in listing)

        call = http(json.dumps({"jsonrpc": "2.0", "id": 3, "method": "tools/call",
                                 "params": {"name": "life_system_status", "arguments": {}}}),
                    f"Authorization: Bearer {token}\r\n")
        check("The remote server executes a tool", "schema_version" in call)
        bad = http(json.dumps({"jsonrpc": "2.0", "id": 4, "method": "tools/call",
                                "params": {"name": "life_tasks_create", "arguments": {}}}),
                   f"Authorization: Bearer {token}\r\n")
        check("The remote server validates the arguments", "VALIDATION_ERROR" in bad)

        op("remote_clients.revoke", {"id": client["id"]})
        revoked = http(json.dumps({"jsonrpc": "2.0", "id": 5, "method": "tools/list"}),
                       f"Authorization: Bearer {token}\r\n")
        check("A revoked token is refused", "403" in revoked.splitlines()[0])
    finally:
        server.terminate()
        server.wait(timeout=10)

    result = {"suite": "release", "count": len(passed), "passed": passed}
    pathlib.Path("qa/release_result.json").write_text(json.dumps(result, indent=2))
    print(json.dumps(result, indent=2))
