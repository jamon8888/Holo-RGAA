#!/usr/bin/env python3
"""P-00 JSON-RPC stdio echo sidecar — mirrors crates/rgaa-analysis echo_proof."""

import hashlib
import json
import sys


def send(payload):
    sys.stdout.write(json.dumps(payload, separators=(",", ":")) + "\n")
    sys.stdout.flush()


def sha256(text):
    return hashlib.sha256(text.encode("utf-8")).hexdigest()


def main():
    options = {}
    for line in sys.stdin:
        if not line.strip():
            continue
        message = json.loads(line)
        method = message.get("method")
        request_id = message.get("id")

        if method == "initialiser":
            params = message.get("params", {})
            options = params.get("options") or {}
            send({"jsonrpc": "2.0", "method": "journal",
                  "params": {"niveau": "info", "message": "echo sidecar prêt"}})
            send({"jsonrpc": "2.0", "id": request_id, "result": {
                "id": "echo-sidecar", "version": "0.1.0",
                "tests": ["1.1.1"], "version_modele": None}})

        elif method == "analyser":
            url = message.get("params", {}).get("url", "")
            echo = options["message"] if "message" in options else "echo"
            artefact = sha256("echo:" + url + ":" + echo)
            proof = {
                "plugin": "echo", "version_plugin": "0.1.0", "version_modele": None,
                "tests": ["1.1.1"], "sujet": "Page", "portee": "Mesuree",
                "mesures": {"echo": {"url": url, "message": echo}},
                "indice": None,
                "artefacts": [{"nom": "echo.txt", "media_type": "text/plain",
                               "chemin_bundle": "analysis/echo/" + artefact + ".txt",
                               "sha256": artefact}],
                "empreinte_entree": sha256("echo\0" + url + "\0" + echo),
            }
            send({"jsonrpc": "2.0", "id": request_id, "result": {"preuves": [proof]}})

        elif method == "arreter":
            break


if __name__ == "__main__":
    main()
