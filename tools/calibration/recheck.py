#!/usr/bin/env python3
"""Re-vérification à l'aveugle : un sous-ensemble de cartes re-soumis à l'étiqueteur.

Elle mesure ce qu'une seule passe ne peut pas dire : la **cohérence test-retest** de
l'étiqueteur, qui plafonne ce qu'un modèle peut atteindre, et si un écart avec la règle est un
glissement ponctuel ou une préférence stable.

Les cartes sont ré-identifiées et mélangées ; la page n'indique ni lesquelles sont revues pour
une raison particulière, ni le verdict précédent. La justification reste obligatoire.

    python3 recheck.py build   --lot-dir lot --labels export.json [--extra g03,g19] [--n 14] [--seed 7]
    python3 recheck.py compare --lot-dir lot --labels export.json --labels2 export2.json
"""
import argparse, collections, hashlib, json, math, os, random, sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from sheet import build as build_sheet

VERDICTS = ("pertinent", "non_pertinent")


def wilson(k, n, z=1.96):
    if not n:
        return (0.0, 0.0)
    p, d = k / n, 1 + z * z / n
    c = (p + z * z / (2 * n)) / d
    m = z * math.sqrt(p * (1 - p) / n + z * z / (4 * n * n)) / d
    return (max(0.0, c - m), min(1.0, c + m))


def load(lot):
    items = {x["qid"]: x for x in json.load(open(os.path.join(lot, "items.json"), encoding="utf-8"))}
    key = json.load(open(os.path.join(lot, "key.json"), encoding="utf-8"))
    return items, key


def cmd_build(a):
    items, key = load(a.lot_dir)
    labels = {l["qid"]: l for l in json.load(open(a.labels, encoding="utf-8"))["labels"]}
    rng = random.Random(a.seed)
    flagged = [q for q in key if labels[q]["verdict"] in VERDICTS
               and (labels[q]["verdict"] == VERDICTS[0]) != (key[q]["pred"] == "P")]   # humain != prédiction
    flagged += [q for q in (a.extra.split(",") if a.extra else []) if q in key and q not in flagged]
    rest = [q for q in key if q not in flagged and not key[q].get("anchor")]
    # compléter à n en gardant l'équilibre pertinent / non pertinent de l'étiquetage initial
    by_v = {v: [q for q in rest if labels[q]["verdict"] == v] for v in VERDICTS}
    for v in by_v:
        rng.shuffle(by_v[v])
    need = max(0, a.n - len(flagged))
    pick = []
    while len(pick) < need and (by_v[VERDICTS[0]] or by_v[VERDICTS[1]]):
        for v in VERDICTS:
            if by_v[v] and len(pick) < need:
                pick.append(by_v[v].pop())
    chosen = flagged + pick
    rng.shuffle(chosen)

    new_items, new_key = [], {}
    for n, orig in enumerate(chosen, 1):
        r = f"h{n:02d}"
        x = {k: v for k, v in items[orig].items() if k not in ("qid", "id")}
        x.update(qid=r, id=f"H{n:03d}")
        new_items.append(x)
        new_key[r] = {"id": f"H{n:03d}", "orig": orig, "flagged": orig in flagged,
                      "first": labels[orig]["verdict"], "pred": key[orig]["pred"],
                      "kind": key[orig].get("kind", ""), "anchor": bool(key[orig].get("anchor"))}

    out = os.path.join(a.lot_dir, "recheck")
    os.makedirs(os.path.join(out, "serve"), exist_ok=True)
    json.dump(new_items, open(os.path.join(out, "items.json"), "w", encoding="utf-8"), ensure_ascii=False, indent=1)
    json.dump(new_key, open(os.path.join(out, "key.json"), "w", encoding="utf-8"), ensure_ascii=False, indent=1)
    rubric_path = os.path.join(a.lot_dir, "rubric.html")
    rubric = (open(rubric_path, encoding="utf-8").read() if os.path.exists(rubric_path) else "") + (
        "<p><strong>Ce sont des cartes déjà vues</strong>, dans un autre ordre et avec d'autres numéros. "
        "Vos réponses précédentes ne sont pas rechargées : ne cherchez pas à les retrouver, jugez chaque carte "
        "comme si c'était la première fois.</p>")
    digest = hashlib.sha1(json.dumps(new_items, sort_keys=True, ensure_ascii=False).encode()).hexdigest()[:8]
    html = build_sheet(new_items, a.lot_dir, f"recheck-{digest}", a.title or "Re-vérification — cartes déjà vues", rubric)
    for leak in ('"pred"', "anchor", '"orig"', "flagged", '"first"'):
        if leak in html:
            sys.exit(f"fuite possible dans la page : {leak!r}")
    open(os.path.join(out, "serve", "etiquetage.html"), "w", encoding="utf-8").write(html)
    print(f"{len(new_items)} cartes dont {len(flagged)} signalées ({', '.join(flagged)}) -> {out}/serve/etiquetage.html")


def cmd_compare(a):
    key = json.load(open(os.path.join(a.lot_dir, "recheck", "key.json"), encoding="utf-8"))
    second = {l["qid"]: l for l in json.load(open(a.labels2, encoding="utf-8"))["labels"]}
    rows = [dict(v, rid=r, second=second[r]["verdict"], c2=second[r].get("comment", "")) for r, v in key.items() if r in second]
    ok = [r for r in rows if r["first"] == r["second"]]
    lo, hi = wilson(len(ok), len(rows))
    print(f"COHÉRENCE TEST-RETEST : {len(ok)}/{len(rows)} = {len(ok)/len(rows):.0%}   IC95 [{lo:.0%} ; {hi:.0%}]\n")
    for lbl, flag in (("signalées", True), ("non signalées", False)):
        g = [r for r in rows if r["flagged"] == flag]
        if g:
            print(f"   cartes {lbl:14}: {sum(1 for r in g if r['first']==r['second'])}/{len(g)} stables")
    print()
    for r in sorted(rows, key=lambda x: (x["first"] == x["second"], x["rid"])):
        mark = "stable " if r["first"] == r["second"] else "CHANGE "
        print(f"   {mark}{r['orig']} {'*' if r['flagged'] else ' '} [{r['kind'][:26]:26}] 1re={r['first'][:3]} 2e={r['second'][:3]}  « {r['c2'][:70]} »")
    print("\n   * = signalée à la construction")


def main():
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    sub = ap.add_subparsers(dest="cmd", required=True)
    b = sub.add_parser("build")
    b.add_argument("--lot-dir", required=True); b.add_argument("--labels", required=True)
    b.add_argument("--extra", default="", help="identifiants de cartes à re-soumettre en plus, séparés par des virgules")
    b.add_argument("--n", type=int, default=14); b.add_argument("--seed", type=int, default=7); b.add_argument("--title", default="")
    c = sub.add_parser("compare")
    c.add_argument("--lot-dir", required=True); c.add_argument("--labels", required=True); c.add_argument("--labels2", required=True)
    a = ap.parse_args()
    {"build": cmd_build, "compare": cmd_compare}[a.cmd](a)


if __name__ == "__main__":
    main()
