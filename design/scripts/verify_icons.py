"""Check that every Lucide icon the app imports can be resolved to geometry."""
import pathlib
import re
import sys

HERE = pathlib.Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import lucide

APP = HERE.parents[1] / "app" / "src"


def app_imports():
    names = set()
    for f in APP.rglob("*.svelte"):
        src = f.read_text()
        for block in re.findall(r"import\s*\{([^}]*)\}\s*from\s*'lucide-svelte'", src):
            for n in block.split(","):
                n = n.strip()
                if n:
                    names.add(n)
    return names


def main():
    names = sorted(app_imports())
    failed = []
    for n in names:
        try:
            path = lucide.icon_path(n)
            if not path:
                failed.append((n, "empty path"))
        except Exception as exc:
            failed.append((n, str(exc)))
    print("app imports:", len(names), "failed:", len(failed))
    for n, e in failed:
        print("  ERR", n, "->", e)
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
