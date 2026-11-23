#!/usr/bin/env python3
"""Disposable SQLite + release CLI/MCP regression for Life Manager schema v9."""
from __future__ import annotations
import os,json,base64,subprocess,tempfile,pathlib
ROOT=pathlib.Path(__file__).resolve().parents[1]
BIN=os.environ.get("LIFE_CLI",str(ROOT/"dist/life"))
passed=[]
with tempfile.TemporaryDirectory(prefix="life-v9-cli-") as tmp:
    db=str(pathlib.Path(tmp)/"life.sqlite")
    def op(action,p=None,actor="qa"):
        args=[BIN,"--db",db,"--json","--actor",actor,"call",action,"--payload",json.dumps(p or {})]
        result=subprocess.run(args,check=False,capture_output=True,text=True,timeout=35)
        if result.returncode:raise AssertionError(f"{action} exit={result.returncode}\n{result.stdout}\n{result.stderr}")
        response=json.loads(result.stdout)
        assert response["ok"],response
        return response["data"]
    def check(message,condition):
        assert condition,message
        passed.append(message)
    note=op("notes.create",{"title":"Portable engineering reference"})
    check("created permanent local note",bool(note["id"]))
    payload=b"FigCraft verified attachment bytes.\n"
    encoded=base64.b64encode(payload).decode("ascii")
    request={"owner_kind":"notes","owner_id":note["id"],"filename":"source.txt","mime_type":"text/plain","data_base64":encoded}
    attached=op("attachments.create",request)
    check("attachment SHA256 persisted",len(attached["sha256"])==64)
    check("binary content absent from metadata", "data_base64" not in attached)
    same=op("attachments.create",request)
    check("attachment retries deduplicate by hash",same["id"]==attached["id"])
    result=op("attachments.get",{"id":attached["id"]})
    check("attachment read yields exactly original bytes",base64.b64decode(result["data_base64"])==payload)
    op("attachments.delete",{"id":attached["id"]})
    check("deleted attachment is hidden",len(op("attachments.list",{"owner_kind":"notes","owner_id":note["id"]}))==0)
    op("attachments.restore",{"id":attached["id"]})
    check("archived attachment restored",len(op("attachments.list",{"owner_kind":"notes","owner_id":note["id"]}))==1)
    career=op("career.create",{"employer":"QA Studio","role":"Staff Engineer","stage":"applied"})
    op("career.update",{"id":career["id"],"stage":"interview","expected_revision":1})
    check("career stage advances with optimistic revision",op("career.get",{"id":career["id"]})["stage"]=="interview")
    learning=op("learning.create",{"title":"Technical architecture","status":"active","target_hours":30})
    log=op("learning.log",{"track_id":learning["id"],"minutes":45,"day":"2026-11-14"})
    check("study history linked to course",op("learning.sessions",{"track_id":learning["id"]})[0]["id"]==log["id"])
    finance=op("finance.create",{"title":"Yearly renewal","currency":"SGD","amount_minor":19950,"due_date":"2026-12-01"})
    check("financial integer minor units preserved",finance["amount_minor"]==19950)
    wellbeing=op("wellbeing.log",{"day":"2026-11-10","energy":3,"mood":4,"sleep_hours":7})
    check("optional wellbeing check-in stored",wellbeing["energy"]==3)
    dash=op("life_domains.dashboard")
    check("new life-area dashboard available",len(dash["career"])>0 and len(dash["learning"])>0)
    content=(ROOT/"qa/fixtures/calendar-sample.ics").read_text()
    prev=op("calendar.import.preview",{"source":"test.ics","content":content})
    check("calendar importer parses two valid VEVENTs",len(prev["valid"])==2)
    apply=op("calendar.import.apply",{"source":"test.ics","content":content,"confirm":True})
    check("calendar import inserts two protected events",len(apply["created"])==2)
    check("calendar import is idempotent by UID",len(op("calendar.import.preview",{"source":"test.ics","content":content})["duplicates"])==2)
    events=op("events.list")
    check("imported calendar events locked",all(e["locked"]==1 for e in events))
    exported=op("calendar.export_ics")
    check("ICS exporter emits RFC5545 date-time syntax","DTSTART:20261112T090000" in exported["content"])
    check("ICS exporter emits all-day DATE","DTSTART;VALUE=DATE:20261114" in exported["content"])
    check("calendar export includes standard UID","UID:" in exported["content"])
    archive=op("export")
    check("JSON overview includes current schema",archive["schema_version"]>=9)
    check("JSON overview contains metadata but not binary",len(archive["managed_attachments"])==1 and "content" not in archive["managed_attachments"][0])
    check("JSON overview includes domain table",len(archive["career_applications"])==1)
    requests=[
        {"jsonrpc":"2.0","id":1,"method":"initialize","params":{}},
        {"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}},
        {"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"life_calendar_import_preview","arguments":{"source":"agent-demo.ics","content":content}}},
        {"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"life_career_list","arguments":{}}},
    ]
    process=subprocess.run([BIN,"--db",db,"mcp","serve","--stdio"],input="\n".join(json.dumps(r) for r in requests)+"\n",capture_output=True,text=True,timeout=30)
    assert process.returncode==0,process.stderr
    replies=[json.loads(line) for line in process.stdout.splitlines() if line.strip()]
    toolnames={tool["name"] for tool in replies[1]["result"]["tools"]}
    check("MCP catalog exposes ICS and domain tools", {"life_calendar_import_preview","life_calendar_import_apply","life_career_update","life_attachments_get"}<=toolnames)
    check("MCP three-part action mapping works", "Review priorities" in replies[2]["result"]["content"][0]["text"])
    check("MCP domain read maps correctly","QA Studio" in replies[3]["result"]["content"][0]["text"])
print(json.dumps({"status":"PASS","count":len(passed),"checks":passed},indent=2))
