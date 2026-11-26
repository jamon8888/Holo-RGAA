#!/usr/bin/env python3
"""Package the rgaa-chrome extension into a distributable zip.

Produces a Chrome-compatible archive (`rgaa-chrome-<version>.zip`) containing
only the runtime files (manifest + scripts + panel). Install it with
chrome://extensions → mode développeur → "Charger l'extension non empaquetée"
after unzipping, or publish it to the Chrome Web Store.
"""
import json
import sys
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent
MANIFEST = ROOT / "manifest.json"

RUNTIME_FILES = [
    "manifest.json",
    "background.js",
    "lib.js",
    "panel.js",
    "panel.html",
]


def main() -> int:
    with MANIFEST.open("r", encoding="utf-8") as fh:
        manifest = json.load(fh)
    version = manifest.get("version", "0.0.0")
    out_dir = ROOT / "dist"
    out_dir.mkdir(parents=True, exist_ok=True)
    out_zip = out_dir / f"rgaa-chrome-{version}.zip"

    missing = [name for name in RUNTIME_FILES if not (ROOT / name).exists()]
    if missing:
        print(f"fichiers manquants : {', '.join(missing)}", file=sys.stderr)
        return 1

    with zipfile.ZipFile(out_zip, "w", zipfile.ZIP_DEFLATED) as zf:
        for name in RUNTIME_FILES:
            zf.write(ROOT / name, name)

    print(f"{out_zip} ({out_zip.stat().st_size} octets)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
