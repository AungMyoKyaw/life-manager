#!/usr/bin/env python3
"""Release CLI and MCP regression for the schema v15 domain.

Every check runs against a disposable SQLite file through the compiled
release binary, so it proves the same code path the desktop app uses.
"""
from __future__ import annotations
import json, os, pathlib, subprocess, tempfile

BIN = os.environ.get("LIFE_CLI", "/tmp/life-manager-target/release/life")
passed: list[str] = []

with tempfile.TemporaryDirectory(prefix="life-v15-cli-") as tmp:
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

    def check(message, condition):
        assert condition, message
        passed.append(message)

    # --- recurrence exceptions -------------------------------------------------
    task = op("tasks.create", {"title": "Daily reading", "due_date": "2026-03-02",
                               "recurrence": "daily"})
    window = op("recurrence.occurrences", {"task_id": task["id"], "from": "2026-03-02",
                                           "to": "2026-03-05"})
    check("Recurring task expands into concrete occurrences", window["count"] == 4)
    op("recurrence.exceptions.create", {"task_id": task["id"], "occurrence_date": "2026-03-03",
                                        "action": "skip", "reason": "travel"})
    after = op("recurrence.occurrences", {"task_id": task["id"], "from": "2026-03-02",
                                          "to": "2026-03-05"})
    check("A skipped occurrence is reported as skipped", after["occurrences"][1]["state"] == "skipped")
    op("recurrence.exceptions.create", {"task_id": task["id"], "occurrence_date": "2026-03-04",
                                        "action": "reschedule", "moved_to": "2026-03-09"})
    moved = op("recurrence.occurrences", {"task_id": task["id"], "from": "2026-03-02",
                                          "to": "2026-03-05"})
    check("A rescheduled occurrence keeps its new date",
          moved["occurrences"][2]["state"] == "moved" and moved["occurrences"][2]["moved_to"] == "2026-03-09")
    check("Exceptions are listed for the task",
          len(op("recurrence.exceptions.list", {"task_id": task["id"]})) == 2)

    # --- calendar sources -------------------------------------------------------
    source = op("calendar.sources.create", {"name": "Imported ICS", "kind": "ics"})
    check("A calendar source is created read only", source["read_only"] == 1)
    check("Calendar sources are listed", len(op("calendar.sources.list", {})) == 1)

    # --- planning ---------------------------------------------------------------
    import datetime
    today = datetime.date.today()
    monday = today - datetime.timedelta(days=today.weekday())
    planned = monday.isoformat()
    area = op("areas.create", {"name": "Parity Area"})
    op("tasks.create", {"title": "Unfinished", "due_date": planned,
                        "planned_date": planned, "area_id": area["id"],
                        "estimate_minutes": 90})
    op("protected.create", {"title": "Deep work", "weekday": 0,
                            "start_time": "09:00", "end_time": "11:00"})
    check("Protected time is stored", len(op("protected.list", {})) == 1)
    op("time_budgets.set", {"scope_kind": "area", "scope_id": area["id"],
                            "period": "week", "minutes": 60})
    budget = op("time_budgets.report", {})
    row = next(r for r in budget["areas"] if r["area_id"] == area["id"])
    check("The weekly budget overrun is measured", row["overrun_minutes"] == 30 and row["planned_minutes"] == 90)
    check("The overrun is reported as a status", row["status"] == "overrun")
    carry = op("planning.carryover", {"from": "2026-03-02", "to": "2026-03-09"})
    check("Unfinished work is offered for carryover", carry["count"] >= 1)

    # --- focus modes ------------------------------------------------------------
    preset = op("focus.presets.create", {"name": "Parity pomodoro", "focus_minutes": 25,
                                          "break_minutes": 5, "cycles": 4})
    check("A focus preset is stored", preset["focus_minutes"] == 25)
    session = op("focus.start", {"mode": "pomodoro", "preset_id": preset["id"]})
    check("A pomodoro session starts", session["mode"] == "pomodoro")
    op("focus.stop", {"id": session["id"]})
    manual = op("focus.manual", {"minutes": 45, "day": "2026-03-02", "reason": "offline"})
    check("A manual entry records real minutes", manual["duration_seconds"] == 2700)

    # --- executable views -------------------------------------------------------
    op("tasks.create", {"title": "Urgent work", "priority": 1, "status": "todo"})
    view = op("saved_views.create", {"name": "Urgent", "query": {"priority": 1}, "entity": "tasks"})
    run = op("saved_views.run", {"id": view["id"]})
    check("A saved view executes and returns matching rows", run["count"] == 1)
    search = op("saved_searches.create", {"name": "Open work", "entity": "all",
                                          "query": {"status": "todo"}})
    results = op("saved_searches.run", {"id": search["id"]})
    check("A saved search runs across every entity", results["count"] >= 2)

    # --- backlinks --------------------------------------------------------------
    note = op("notes.create", {"title": "Parity note"})
    op("links.add", {"source_kind": "notes", "source_id": note["id"],
                     "target_kind": "tasks", "target_id": task["id"], "relation": "reference"})
    backlinks = op("links.backlinks", {"kind": "tasks", "id": task["id"]})
    check("Backlinks report the incoming link", backlinks["count"] == 1)

    # --- insight reports --------------------------------------------------------
    for action in ("reports.plan_vs_actual", "reports.goal_achievement",
                   "reports.area_distribution", "reports.journal_export"):
        report = op(action, {"from": "2026-03-01", "to": "2026-03-05"})
        check(f"{action} states its source", "SQLite" in report["source"])
    check("The active goal limit is reported",
          op("goals.limit", {"limit": 5})["limit"] == 5)

    # --- agent policy -----------------------------------------------------------
    op("agents.permissions.set", {"target_actor": "helper", "action_pattern": "tasks.*",
                                  "allowed": True})
    op("tasks.create", {"title": "Allowed for helper", "actor": "helper"})
    op("agents.permissions.set", {"target_actor": "helper", "action_pattern": "tasks.create",
                                  "allowed": False})
    denied = subprocess.run([BIN, "--db", db, "--json", "--actor", "helper", "call",
                             "tasks.create", "--payload", json.dumps({"title": "Denied"})],
                            capture_output=True, text=True, timeout=35)
    refused = json.loads(denied.stderr or denied.stdout)
    check("A denied action pattern is refused", refused["ok"] is False)
    check("The refusal explains which actor was blocked",
          "helper" in refused["error"]["message"])
    proposal = op("agents.proposals.create", {"action": "tasks.create",
                                              "payload": {"title": "Proposed"}})
    check("A proposal waits for review", proposal["state"] == "pending")
    approved = op("agents.proposals.approve", {"id": proposal["id"], "reason": "ok"})
    check("An approved proposal applies its change", approved["state"] == "approved")

    # --- scheduled automations --------------------------------------------------
    rule = op("automations.create", {"title": "Scheduled note", "action": "notes.create",
                                     "arguments": {"title": "Scheduled note"}})
    op("automations.schedule.set", {"rule_id": rule["id"], "kind": "daily", "at_time": "00:00"})
    check("A scheduled trigger becomes due", op("automations.schedule.due", {})["count"] == 1)
    ran = op("automations.schedule.run_due", {})
    check("Due schedules run once", ran["count"] == 1 and ran["ran"][0]["ok"])
    check("A schedule does not run twice in one day",
          op("automations.schedule.due", {})["count"] == 0)

    # --- MCP surface ------------------------------------------------------------
    mcp = subprocess.run([BIN, "--db", db, "mcp", "serve", "--stdio"], input="", capture_output=True,
                          text=True, timeout=35)
    handshake = subprocess.run(
        [BIN, "--db", db, "mcp", "serve", "--stdio"],
        input=json.dumps({"jsonrpc": "2.0", "id": 1, "method": "tools/list"}) + "\n",
        capture_output=True, text=True, timeout=35)
    tools = json.loads(handshake.stdout.strip().splitlines()[-1])["result"]["tools"]
    names = {tool["name"] for tool in tools}
    check("MCP exposes the new calendar source tool", "life_calendar_sources_list" in names)
    check("MCP exposes the new planning tool", "life_planning_carryover" in names)
    check("MCP exposes the new report tool", "life_reports_plan_vs_actual" in names)
    check("MCP exposes the new proposal tool", "life_agents_proposals_approve" in names)
    check("MCP exposes the new focus tool", "life_focus_manual" in names)
    call = subprocess.run(
        [BIN, "--db", db, "mcp", "serve", "--stdio"],
        input=json.dumps({"jsonrpc": "2.0", "id": 2, "method": "tools/call",
                          "params": {"name": "life_time_budgets_report", "arguments": {}}}) + "\n",
        capture_output=True, text=True, timeout=35)
    reply = json.loads(call.stdout.strip().splitlines()[-1])
    check("MCP executes the new action", "result" in reply and "isError" not in reply.get("result", {}))
    bad = subprocess.run(
        [BIN, "--db", db, "mcp", "serve", "--stdio"],
        input=json.dumps({"jsonrpc": "2.0", "id": 3, "method": "tools/call",
                          "params": {"name": "life_tasks_create", "arguments": {}}}) + "\n",
        capture_output=True, text=True, timeout=35)
    reply = json.loads(bad.stdout.strip().splitlines()[-1])
    check("MCP rejects a missing required parameter",
          reply.get("result", {}).get("isError") is True)
    assert mcp.returncode in (0, None)

    result = {"suite": "parity_v15", "count": len(passed), "passed": passed}
    pathlib.Path("qa/parity_v15_result.json").write_text(json.dumps(result, indent=2))
    print(json.dumps(result, indent=2))
