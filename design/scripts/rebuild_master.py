#!/usr/bin/env python3
"""Regenerate the FigCraft master from the design sources.

Runs the whole chain: build the boards, bind the design tokens, convert to
native FigCraft, then re-resolve the token bindings so the stored colours
match the tokens.
"""
import json
import pathlib
import subprocess
import sys

HERE = pathlib.Path(__file__).resolve().parent
ROOT = HERE.parents[1]
DESIGN = ROOT / "design"
WORK = DESIGN / ".build"


def run(cmd, **kw):
    r = subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True, **kw)
    if r.returncode != 0:
        sys.stderr.write(r.stdout + r.stderr)
        raise SystemExit("failed: " + " ".join(str(c) for c in cmd))
    return r.stdout


def main():
    WORK.mkdir(parents=True, exist_ok=True)
    master_json = DESIGN / "life-manager-master.json"
    master = DESIGN / "life-manager-master.figcraft"

    print(run([sys.executable, str(HERE / "build_figcraft.py")]).strip())
    print(run([sys.executable, str(HERE / "bind_tokens.py")]).strip())
    # Keep the brand page in the canonical regenerated master.
    print(run([sys.executable, str(HERE / "build_brand_identity.py")]).strip())

    run(["figcraft-cli", "convert", str(master_json), str(master)])

    # Re-resolve every binding so the stored literals match the tokens.
    # A page uses the default mode; a frame that asks for a mode keeps it,
    # so the dark board stays dark.
    doc = json.loads(master_json.read_text())
    modes = {}
    for pid in doc["pages"]:
        modes[pid] = "VariableCollectionId:1/mode1"
    for node in doc["nodes"].values():
        for collection, mode in (node.get("var_modes") or {}).items():
            if collection == "VariableCollectionId:1":
                modes[node["id"]] = mode
    steps = []
    for pid in doc["pages"]:
        steps.append({"command": "variable.setMode",
                      "params": {"ids": [pid], "collection": "VariableCollectionId:1",
                                 "mode": "Light"}})
    for node_id, mode in modes.items():
        if mode == "VariableCollectionId:1/mode1":
            continue
        steps.append({"command": "variable.setMode",
                      "params": {"ids": [node_id], "collection": "VariableCollectionId:1",
                                 "mode": "Dark"}})
    script = WORK / "resolve.json"
    script.write_text(json.dumps(steps))
    out = WORK / "resolved.figcraft"
    run(["figcraft-cli", "run", str(script), "--in", str(master),
         "--out", str(out),
         "--automation-read-root", str(DESIGN),
         "--automation-write-root", str(WORK)])
    out.replace(master)

    info = run(["figcraft-cli", "info", str(master), "--json"])
    d = json.loads(info)
    d["file"] = master.relative_to(ROOT).as_posix()
    (DESIGN / "figcraft-info.json").write_text(json.dumps(d, indent=2) + "\n")
    print(f"OK pages={len(d['pages'])} nodes={d['nodes']} "
          f"variables={d['variables']} missingFonts={d['missingFonts']}")


if __name__ == "__main__":
    main()
