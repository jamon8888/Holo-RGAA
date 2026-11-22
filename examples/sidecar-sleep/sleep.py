#!/usr/bin/env python3
"""Sidecar that answers initialiser, then sleeps forever on analyser."""

import json
import sys
import time

for line in sys.stdin:
    if not line.strip():
        continue
    msg = json.loads(line)
    if msg.get("method") == "initialiser":
        sys.stdout.write(json.dumps({
            "jsonrpc": "2.0", "id": msg.get("id"),
            "result": {"id": "sleep", "version": "0.1.0",
                       "tests": ["9.9.9"], "version_modele": None},
        }) + "\n")
        sys.stdout.flush()
    elif msg.get("method") == "analyser":
        time.sleep(30)
    elif msg.get("method") == "arreter":
        break
