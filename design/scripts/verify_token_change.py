"""Prove that one token change repaints the whole design master.

The master is copied, one design token is changed, and the result is
compared with the original.  A token that only affected a few layers would
leave most boards untouched; the design system's promise is that a token
change reaches every board that uses it.
"""
import json
import pathlib
import subprocess
import sys
import tempfile

HERE = pathlib.Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import tokens

ROOT = HERE.parents[1]
MASTER = ROOT / "design" / "life-manager-master.figcraft"
INVENTORY = ROOT / "design" / "screen-inventory.json"


def main():
    token_name = sys.argv[1] if len(sys.argv) > 1 else "accent"
    new_value = sys.argv[2] if len(sys.argv) > 2 else "#FF00FF"

    # The variable id for the token.
    names = list(tokens.COLORS)
    index = names.index(token_name) + 1
    var_id = f"VariableID:{index}"

    work = pathlib.Path(tempfile.mkdtemp())
    base = work / "base.figcraft"
    base.write_bytes(MASTER.read_bytes())

    script = work / "set.json"
    script.write_text(json.dumps([
        {"command": "variable.set",
         "params": {"variable": var_id, "mode": "Light", "value": new_value}},
    ]))
    out = work / "changed.figcraft"
    r = subprocess.run(
        ["figcraft-cli", "run", str(script), "--in", str(base), "--out", str(out),
         "--automation-read-root", str(work), "--automation-write-root", str(work)],
        capture_output=True, text=True,
    )
    print(r.stdout.strip())
    if r.returncode != 0:
        print(r.stderr)
        return 1

    inventory = json.loads(INVENTORY.read_text())
    changed_boards = 0
    unchanged = []
    for entry in inventory:
        node_id = entry["node_id"]
        before = work / "b.png"
        after = work / "a.png"
        for target, source in ((before, base), (after, out)):
            subprocess.run(
                ["figcraft-cli", "export", str(source), "--node", str(node_id),
                 "--out", str(target), "--scale", "0.25"],
                capture_output=True, text=True, check=True,
            )
        from PIL import Image, ImageChops
        a = Image.open(before).convert("RGB")
        b = Image.open(after).convert("RGB")
        if a.size != b.size:
            unchanged.append(entry["title"])
            continue
        if ImageChops.difference(a, b).getbbox() is not None:
            changed_boards += 1
        else:
            unchanged.append(entry["title"])

    print(f"token {token_name!r} -> {new_value} ({var_id})")
    print(f"boards changed: {changed_boards} of {len(inventory)}")
    if unchanged:
        print(f"boards unchanged: {len(unchanged)}")
        for t in unchanged[:10]:
            print("   ", t)
    return 0


if __name__ == "__main__":
    sys.exit(main())
