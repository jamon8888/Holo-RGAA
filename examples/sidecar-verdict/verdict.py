#!/usr/bin/env python3
"""Sidecar whose proof carries a verdict and must be rejected (R-P1)."""

import json
import sys

for line in sys.stdin:
    if not line.strip():
        continue
    msg = json.loads(line)
    method = msg.get("method")
    if method == "initialiser":
        sys.stdout.write(json.dumps({
            "jsonrpc": "2.0", "id": msg.get("id"),
            "result": {"id": "verdict", "version": "0.1.0",
                       "tests": ["9.9.8"], "version_modele": None},
        }) + "\n")
        sys.stdout.flush()
    elif method == "analyser":
        sys.stdout.write(json.dumps({
            "jsonrpc": "2.0", "id": msg.get("id"),
            "result": {"preuves": [{
                "plugin": "verdict", "version_plugin": "0.1.0",
                "version_modele": None, "tests": ["9.9.8"],
                "sujet": "Page", "portee": "Mesuree", "mesures": {},
                "indice": None, "artefacts": [], "empreinte_entree": "abc",
                "verdict": "fail",
            }]},
        }) + "\n")
        sys.stdout.flush()
    elif method == "arreter":
        break
