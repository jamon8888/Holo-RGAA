#!/usr/bin/env python3
"""Extrait les identifiants de règles d'un `axe.min.js` et écrit
`rgaa-rs/crates/rgaa-core/data/rgaa-4.1.2/axe_rules_injected.json`, ou, avec
`--check`, vérifie que le fichier commité correspond exactement au bundle.

Pourquoi un fichier à part d'`axe_rules.json` : ce dernier est régénéré par
`rgaa-data` depuis la documentation de la branche `develop` d'axe-core (4.13+),
alors que l'audit injecte la version épinglée dans `rgaa-obscura`
(`AXE_CORE_CDN`). Le registre des mécanismes doit citer des règles que la version
**injectée** exécute (spec criteria-coverage-expansion §5, issue #261) : une règle
absente de cette version ne lève jamais de violation, donc ne peut jamais rien
décider.

Le script lit le fichier comme du texte, sans l'exécuter. Il parcourt le tableau
`rules:[…]` de la build minifiée en équilibrant accolades et crochets (chaînes et
échappements compris) : chaque élément de premier niveau doit être un objet, et
chaque objet doit porter à son propre niveau une clé `id:"…"` et une clé `tags:`.
Une règle dont l'objet contient des structures imbriquées (`metadata:{…}`,
`all:[…]`) est donc lue comme les autres — une expression régulière qui refuse les
accolades l'aurait omise sans le dire.

Usage :
    curl -fsSL -o /tmp/axe.min.js \\
      https://cdnjs.cloudflare.com/ajax/libs/axe-core/4.9.1/axe.min.js
    # régénérer le fichier
    python3 scripts/extract-axe-rule-ids.py /tmp/axe.min.js 4.9.1
    # vérifier le fichier commité (sha256 et règles) contre le bundle
    python3 scripts/extract-axe-rule-ids.py --check /tmp/axe.min.js
"""

import hashlib
import json
import re
import sys
from pathlib import Path

OUT = (
    Path(__file__).resolve().parent.parent
    / "rgaa-rs/crates/rgaa-core/data/rgaa-4.1.2/axe_rules_injected.json"
)

ANCHOR = "rules:[{id:"


class ExtractError(Exception):
    pass


def _skip_string(text: str, i: int) -> int:
    """`text[i]` ouvre une chaîne ; renvoie l'indice juste après sa fermeture."""
    quote = text[i]
    i += 1
    while i < len(text):
        c = text[i]
        if c == "\\":
            i += 2
            continue
        if c == quote:
            return i + 1
        i += 1
    raise ExtractError(f"chaîne {quote} non fermée")


def _top_level_keys(obj: str) -> dict:
    """Clés de premier niveau d'un littéral objet minifié `{k:v,…}` -> texte de la valeur
    (pour une chaîne, son contenu brut)."""
    keys = {}
    depth = 0
    i = 0
    expect_key = False
    while i < len(obj):
        c = obj[i]
        if c in "\"'`":
            end = _skip_string(obj, i)
            i = end
            continue
        if c in "{[(":
            depth += 1
            if depth == 1:
                expect_key = True
            i += 1
            continue
        if c in "}])":
            depth -= 1
            i += 1
            continue
        if depth == 1 and c == ",":
            expect_key = True
            i += 1
            continue
        if depth == 1 and expect_key:
            m = re.compile(r'([A-Za-z_$][\w$]*|"[^"]*"):').match(obj, i)
            if not m:
                raise ExtractError(f"clé illisible à {obj[i:i + 40]!r}")
            key = m.group(1).strip('"')
            i = m.end()
            value = ""
            if i < len(obj) and obj[i] in "\"'":
                end = _skip_string(obj, i)
                value = obj[i + 1:end - 1]  # brut : seul `id` est lu, et validé
            keys[key] = value
            expect_key = False
            continue
        i += 1
    return keys


def extract_rule_ids(text: str) -> list:
    if text.count(ANCHOR) != 1:
        raise ExtractError(f"tableau `{ANCHOR}` introuvable ou ambigu")
    i = text.index(ANCHOR) + len("rules:")
    assert text[i] == "["
    i += 1
    depth = 1
    objects = []
    obj_start = None
    while depth > 0:
        if i >= len(text):
            raise ExtractError("tableau `rules` non fermé")
        c = text[i]
        if c in "\"'`":
            if depth == 1:
                raise ExtractError("élément du tableau `rules` qui n'est pas un objet")
            i = _skip_string(text, i)
            continue
        if c in "{[(":
            if depth == 1:
                if c != "{":
                    raise ExtractError("élément du tableau `rules` qui n'est pas un objet")
                obj_start = i
            depth += 1
        elif c in "}])":
            depth -= 1
            if depth == 1:
                objects.append(text[obj_start:i + 1])
                obj_start = None
        elif depth == 1 and c not in ", \n\t":
            raise ExtractError(f"élément inattendu dans `rules` : {text[i:i + 40]!r}")
        i += 1
    ids = []
    for obj in objects:
        keys = _top_level_keys(obj)
        if "id" not in keys or "tags" not in keys:
            raise ExtractError(f"objet de règle sans `id` ou sans `tags` : {obj[:80]!r}")
        if not re.fullmatch(r"[a-z0-9-]+", keys["id"]):
            raise ExtractError(f"identifiant de règle inattendu : {keys['id']!r}")
        ids.append(keys["id"])
    if len(ids) != len(set(ids)):
        raise ExtractError("identifiants de règles en double")
    if len(ids) < 50:
        raise ExtractError(f"seulement {len(ids)} règles extraites : extraction suspecte")
    return sorted(ids)


def read_bundle(path: Path):
    raw = path.read_bytes()
    text = raw.decode("utf-8")
    banner = re.match(r"/\*! axe v(\S+)", text)
    if not banner:
        raise ExtractError("bannière `/*! axe v…` absente")
    return raw, text, banner.group(1)


def build(raw: bytes, text: str, version: str) -> dict:
    return {
        "axe_version": version,
        "source": f"https://cdnjs.cloudflare.com/ajax/libs/axe-core/{version}/axe.min.js",
        "sha256": hashlib.sha256(raw).hexdigest(),
        "generated_by": "scripts/extract-axe-rule-ids.py",
        "rules": extract_rule_ids(text),
    }


def check(path: Path) -> int:
    committed = json.loads(OUT.read_text(encoding="utf-8"))
    raw, text, version = read_bundle(path)
    fresh = build(raw, text, version)
    problems = []
    for field in ("axe_version", "source", "sha256"):
        if committed.get(field) != fresh[field]:
            problems.append(f"{field}: commité {committed.get(field)!r}, bundle {fresh[field]!r}")
    extra = sorted(set(committed.get("rules", [])) - set(fresh["rules"]))
    lacking = sorted(set(fresh["rules"]) - set(committed.get("rules", [])))
    if extra:
        problems.append(f"règles commitées absentes du bundle : {extra}")
    if lacking:
        problems.append(f"règles du bundle absentes du fichier commité : {lacking}")
    if committed.get("rules") != fresh["rules"]:
        if not extra and not lacking:
            problems.append("liste de règles commitée non triée ou avec doublons")
    if problems:
        print(f"{OUT} ne correspond pas à {path} :", file=sys.stderr)
        for p in problems:
            print(f"  - {p}", file=sys.stderr)
        return 1
    print(f"OK : {len(fresh['rules'])} règles axe {version}, sha256 {fresh['sha256']}")
    return 0


def main() -> int:
    args = sys.argv[1:]
    try:
        if len(args) == 2 and args[0] == "--check":
            return check(Path(args[1]))
        if len(args) == 2:
            path, version = Path(args[0]), args[1]
            raw, text, banner = read_bundle(path)
            if banner != version:
                print(f"bannière axe {banner!r} != {version!r}", file=sys.stderr)
                return 1
            data = build(raw, text, version)
            OUT.write_text(json.dumps(data, indent=2) + "\n", encoding="utf-8")
            print(f"{len(data['rules'])} règles axe {version} -> {OUT}")
            return 0
    except ExtractError as e:
        print(f"extraction impossible : {e}", file=sys.stderr)
        return 1
    print(__doc__, file=sys.stderr)
    return 2


if __name__ == "__main__":
    sys.exit(main())
