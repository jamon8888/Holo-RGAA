#!/usr/bin/env python3
"""Extrait les identifiants de règles d'un `axe.min.js` et écrit
`rgaa-rs/crates/rgaa-core/data/rgaa-4.1.2/axe_rules_injected.json`.

Pourquoi un fichier à part d'`axe_rules.json` : ce dernier est régénéré par
`rgaa-data` depuis la documentation de la branche `develop` d'axe-core (4.13+),
alors que l'audit injecte la version épinglée dans `rgaa-obscura`
(`AXE_CORE_CDN`). Le registre des mécanismes doit citer des règles que la version
**injectée** exécute (spec criteria-coverage-expansion §5, issue #261) : une règle
absente de cette version ne lève jamais de violation, donc ne peut jamais rien
décider.

Le script lit le fichier comme du texte, sans l'exécuter : les règles sont les
objets `{id:"…", … tags:[…]}` du tableau `rules:[…]` de la build minifiée.

Usage :
    curl -fsSL -o /tmp/axe.min.js \\
      https://cdnjs.cloudflare.com/ajax/libs/axe-core/4.9.1/axe.min.js
    python3 scripts/extract-axe-rule-ids.py /tmp/axe.min.js 4.9.1
"""

import hashlib
import json
import os
import re
import sys
import tempfile
from pathlib import Path

OUT = (
    Path(__file__).resolve().parent.parent
    / "rgaa-rs/crates/rgaa-core/data/rgaa-4.1.2/axe_rules_injected.json"
)


def main() -> int:
    if len(sys.argv) != 3:
        print(__doc__, file=sys.stderr)
        return 2
    path, version = Path(sys.argv[1]), sys.argv[2]
    raw = path.read_bytes()
    text = raw.decode("utf-8")
    banner = re.match(r"/\*! axe v(\S+)", text)
    if not banner or banner.group(1) != version:
        print(f"bannière axe {banner and banner.group(1)!r} != {version!r}", file=sys.stderr)
        return 1
    start = text.find("rules:[{id:")
    if start < 0 or text.count("rules:[{id:") != 1:
        print("tableau `rules:[{id:` introuvable ou ambigu", file=sys.stderr)
        return 1
    ids = sorted(set(re.findall(r'\{id:"([a-z0-9-]+)",[^{}]*?tags:\[', text[start:])))
    if len(ids) < 50:
        print(f"seulement {len(ids)} règles extraites : extraction suspecte", file=sys.stderr)
        return 1
    data = {
        "axe_version": version,
        "source": f"https://cdnjs.cloudflare.com/ajax/libs/axe-core/{version}/axe.min.js",
        "sha256": hashlib.sha256(raw).hexdigest(),
        "generated_by": "scripts/extract-axe-rule-ids.py",
        "rules": ids,
    }
    # Never leave the checked-in invariant half-written: regeneration is often
    # run from a shell and can be interrupted at any point.  `replace` is atomic
    # when both paths live in the same directory.
    with tempfile.NamedTemporaryFile(
        mode="w", encoding="utf-8", dir=OUT.parent, delete=False
    ) as temporary:
        temporary.write(json.dumps(data, indent=2) + "\n")
        temporary.flush()
        os.fsync(temporary.fileno())
        temporary_path = Path(temporary.name)
    temporary_path.replace(OUT)
    print(f"{len(ids)} règles axe {version} -> {OUT}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
