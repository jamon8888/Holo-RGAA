#!/usr/bin/env python3
"""Dépouillement d'une calibration : accord, kappa, Wilson, ancres, et collinéarité.

Protocole : `docs/specs/calibration-protocol.md`.

Trois contrôles décident si une mesure vaut quelque chose, et ils sont affichés avant
l'accord lui-même :

  1. **Collinéarité** phi(justification, verdict). S'il vaut -1, le verdict ne porte aucune
     information propre : le signal réel est « l'étiqueteur a-t-il écrit une correction ».
     C'est ce défaut qui a rendu deux lots entiers inexploitables.
  2. **Ancres.** Des cartes dont le verdict n'est pas discutable, dont une factuellement
     fausse. Moins de 4/4 et la passe est suspecte, quel que soit l'accord.
  3. **Repère trivial.** Le score du meilleur verdict constant. Un accord qui ne le dépasse
     pas, intervalles disjoints, ne vaut rien.

    python3 score.py --key lot/key.json --labels etiquettes.json [--model resultats.jsonl]
"""
import argparse, collections, json, math, re, sys

VERDICTS = ("pertinent", "non_pertinent")
# Marqueurs d'un reproche dans une justification : sert au seul calcul de collinéarité.
REPROCHE = re.compile(r"manqu|ajouter|remplac|faux|hors sujet|pas de|ne prend|trop |incompl", re.I)


def wilson(k, n, z=1.96):
    if not n:
        return (0.0, 0.0)
    p, d = k / n, 1 + z * z / n
    centre = (p + z * z / (2 * n)) / d
    marge = z * math.sqrt(p * (1 - p) / n + z * z / (4 * n * n)) / d
    return (max(0.0, centre - marge), min(1.0, centre + marge))


def kappa(pairs):
    n = len(pairs)
    if not n:
        return 0.0
    po = sum(1 for a, b in pairs if a == b) / n
    pa = sum(1 for a, _ in pairs if a == VERDICTS[0]) / n
    pb = sum(1 for _, b in pairs if b == VERDICTS[0]) / n
    pe = pa * pb + (1 - pa) * (1 - pb)
    return (po - pe) / (1 - pe) if pe < 1 else 0.0


def phi(a, b, c, d):
    den = math.sqrt((a + b) * (c + d) * (a + c) * (b + d))
    return (a * d - b * c) / den if den else 0.0


def pct(k, n):
    lo, hi = wilson(k, n)
    return f"{k}/{n} = {k/n:.0%}  IC95 [{lo:.0%} ; {hi:.0%}]" if n else "—"


def main():
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--key", required=True, help="clé de correction : {qid: {pred, anchor, kind}}")
    ap.add_argument("--labels", required=True, help="export de la feuille d'étiquetage")
    ap.add_argument("--labels2", help="second étiqueteur, pour l'accord inter-juges")
    ap.add_argument("--model", help="résultats d'un modèle, en JSON Lines {qid|id, verdict}")
    a = ap.parse_args()

    key = json.load(open(a.key, encoding="utf-8"))
    export = json.load(open(a.labels, encoding="utf-8"))
    H = {l["qid"]: l for l in export["labels"]}
    unknown = sorted(set(H) - set(key))
    if unknown:
        sys.exit(f"étiquettes sans entrée dans la clé : {unknown}")

    rows = [dict(key[q], qid=q, h=H[q]["verdict"], hc=H[q].get("comment", "")) for q in key if q in H]
    tranches = [r for r in rows if r["h"] in VERDICTS]
    print(f"Étiqueteur : {export.get('labeler') or '(non renseigné)'}")
    print(f"Cartes tranchées : {len(tranches)} / {len(key)}\n")

    # 1. collinéarité — deux quantités, et seule la première détecte le défaut du §2.
    def table(pred):
        t = [0, 0, 0, 0]  # vrai&favorable, vrai&défavorable, faux&favorable, faux&défavorable
        for r in tranches:
            i = (0 if pred(r) else 2) + (0 if r["h"] == VERDICTS[0] else 1)
            t[i] += 1
        return t

    print("== 1. COLLINÉARITÉ justification / verdict")
    vides = [r for r in tranches if not (r["hc"] or "").strip()]
    rempli = table(lambda r: bool((r["hc"] or "").strip()))
    if vides:
        p_rempli = phi(*rempli)
        etat = ("SUSPECT — le verdict ne porte aucune information propre : le signal réel est "
                "« l'étiqueteur a-t-il écrit »" if p_rempli <= -0.9 else "à surveiller")
        print(f"   champ rempli / verdict : phi = {p_rempli:+.3f}   ({etat})")
        print(f"      rempli -> pertinent {rempli[0]:>3} / non pertinent {rempli[1]:>3}")
        print(f"      vide   -> pertinent {rempli[2]:>3} / non pertinent {rempli[3]:>3}")
        print(f"   {len(vides)} carte(s) tranchée(s) sans justification : **le protocole du §2 "
              f"n'est pas respecté**, cette passe est inexploitable en l'état.")
    else:
        print("   champ rempli / verdict : non mesurable — toutes les cartes sont justifiées,")
        print("   ce qui est précisément ce que le protocole du §2 impose. C'est l'état attendu.")

    repr_t = table(lambda r: bool(REPROCHE.search(r["hc"] or "")))
    p_repr = phi(*repr_t)
    print(f"   justification = un reproche / verdict : phi = {p_repr:+.3f}   (indicatif)")
    print(f"      reproche     -> pertinent {repr_t[0]:>3} / non pertinent {repr_t[1]:>3}")
    print(f"      pas reproche -> pertinent {repr_t[2]:>3} / non pertinent {repr_t[3]:>3}")
    print("   Ce second chiffre est attendu négatif : qui rejette une alternative dit ce qui")
    print("   manque. Il ne remplace pas le premier et ne suffit pas à valider une passe.\n")

    # 2. ancres
    anchors = [r for r in tranches if r.get("anchor")]
    ok = [r for r in anchors if (r["h"] == VERDICTS[0]) == (r["pred"] == "P")]
    print(f"== 2. ANCRES : {len(ok)}/{len(anchors)}" + ("" if len(ok) == len(anchors) else "   SUSPECT"))
    for r in anchors:
        mark = "ok   " if r in ok else "ÉCHEC"
        print(f"   {mark} {r['qid']} [{r.get('kind','')}] règle={r['pred']} humain={r['h']}")
    print()

    # 3bis. répétition des justifications favorables. Une phrase copiée sur la moitié des cartes
    # favorables ne dit pas POURQUOI ces alternatives conviennent : le champ est rempli, donc le
    # contrôle de collinéarité ne voit rien, mais la justification ne porte plus d'information.
    fav = [(r["hc"] or "").strip().lower() for r in tranches if r["h"] == VERDICTS[0]]
    if fav:
        top, n_top = collections.Counter(fav).most_common(1)[0]
        part = n_top / len(fav)
        etat = "SUSPECT — ne dit pas pourquoi" if part > 0.30 else "acceptable"
        print(f"== 3. RÉPÉTITION des justifications favorables : la plus fréquente couvre "
              f"{n_top}/{len(fav)} = {part:.0%}   ({etat})")
        if part > 0.30:
            print(f"   « {top[:80]} »")
        print()

    # 3. repère trivial
    freq = collections.Counter(r["h"] for r in tranches)
    triv, triv_n = (freq.most_common(1) or [("—", 0)])[0]
    print(f"== 4. REPÈRE TRIVIAL (réponse constante « {triv} ») : {pct(triv_n, len(tranches))}\n")

    if a.labels2:
        H2 = {l["qid"]: l["verdict"] for l in json.load(open(a.labels2, encoding="utf-8"))["labels"]}
        pairs = [(r["h"], H2[r["qid"]]) for r in tranches if H2.get(r["qid"]) in VERDICTS]
        print(f"== ACCORD INTER-JUGES : {pct(sum(1 for x, y in pairs if x == y), len(pairs))}"
              f"   kappa = {kappa(pairs):+.2f}\n")

    if a.model:
        # Un runner peut indexer ses résultats par `qid` ou par l'identifiant stable
        # de la carte (`id` dans la clé) ; on accepte les deux.
        raw = {}
        for line in open(a.model, encoding="utf-8"):
            r = json.loads(line)
            ident = r.get("qid") or r.get("id")
            if ident is not None:
                raw[ident] = r.get("verdict")
        M = {r["qid"]: raw.get(r["qid"], raw.get(r.get("id"))) for r in tranches}
        manquants = [r["qid"] for r in tranches if M[r["qid"]] is None]
        if manquants:
            print(f"   (sans résultat de modèle : {len(manquants)} carte(s))")
        pairs = [(r["h"], M[r["qid"]]) for r in tranches if M.get(r["qid"]) in VERDICTS]
        agree = sum(1 for x, y in pairs if x == y)
        fp = sum(1 for x, y in pairs if x == VERDICTS[1] and y == VERDICTS[0])
        rejets = sum(1 for x, _ in pairs if x == VERDICTS[1])
        fn = sum(1 for x, y in pairs if x == VERDICTS[0] and y == VERDICTS[1])
        acceptations = sum(1 for x, _ in pairs if x == VERDICTS[0])
        print(f"== MODÈLE vs HUMAIN : {pct(agree, len(pairs))}   kappa = {kappa(pairs):+.2f}")
        print(f"   faux « pertinent » : {fp} / {rejets} rejets"
              f" = {fp/rejets:.0%}  (plafond 10 % — une non-conformité manquée)" if rejets else "")
        print(f"   faux « non pertinent » : {fn} / {acceptations} acceptations"
              f" = {fn/acceptations:.0%}  (plafond 25 % — du travail de revue inutile)"
              if acceptations else f"   faux « non pertinent » : {fn}")

        # Quatre conditions. La dernière interdit qu'un modèle passe en répondant
        # presque toujours la même chose : son accord doit dépasser le repère trivial
        # avec des intervalles de Wilson disjoints.
        lo_m, _ = wilson(agree, len(pairs))
        _, hi_t = wilson(triv_n, len(tranches))
        cond = [
            ("ancres", len(ok) == len(anchors), f"{len(ok)}/{len(anchors)}"),
            ("faux « pertinent »", bool(rejets) and fp / rejets <= 0.10,
             f"{(fp/rejets if rejets else 0):.0%} / 10 %"),
            ("faux « non pertinent »", bool(acceptations) and fn / acceptations <= 0.25,
             f"{(fn/acceptations if acceptations else 0):.0%} / 25 %"),
            ("au-dessus du repère trivial", lo_m > hi_t,
             f"IC modèle [{lo_m:.0%} ; …] contre repère trivial [… ; {hi_t:.0%}]"),
        ]
        seuil = "ADMIS" if all(c[1] for c in cond) else "REFUSÉ"
        print(f"\n   seuil d'admission (§4 du protocole) : **{seuil}**")
        for nom, passe, detail in cond:
            print(f"      {'ok   ' if passe else 'ÉCHEC'} {nom} : {detail}")


if __name__ == "__main__":
    main()
