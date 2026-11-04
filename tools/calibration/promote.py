#!/usr/bin/env python3
"""Passe chaque candidat de candidates.toml sur un lot étiqueté et propose une promotion.

Par défaut : SIMULATION (affiche les commandes, n'appelle aucun modèle, n'envoie rien).
`--run` exécute réellement run_models.py puis score.py.

Garde-fous :
  * la clé n'est lue que par le NOM de variable (key_env) ; elle n'est jamais affichée ni écrite ;
  * un candidat `external = true` n'est lancé qu'avec `--run` ET `--allow-external` ;
  * le script ne modifie RIEN dans le dépôt : il écrit un rapport et PROPOSE les variables
    RGAA_LLM_* ; la bascule réelle passe par une PR relue par l'humain.

    python3 promote.py --lot-dir lot_t --labels etiquettes-...json --prompt-file prompts/resume_tableau.txt
    python3 promote.py --lot-dir lot_t --labels etiquettes-...json --prompt-file prompts/resume_tableau.txt \\
        --run --allow-external
"""
import argparse, json, os, re, subprocess, sys, tomllib

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
from score import wilson  # même intervalle de Wilson que le seuil §4


def _run(argv, **kw):
    """Seul point d'appel à subprocess. `argv` est une LISTE (jamais de shell) : aucun argument n'est
    interprété par un shell, donc pas d'injection de commande. Ses éléments viennent des options de la
    ligne de commande et de candidates.toml (relu en PR), jamais d'une sortie de modèle ni du réseau."""
    assert isinstance(argv, list) and all(isinstance(x, str) for x in argv)
    # nosemgrep: python.lang.security.audit.dangerous-subprocess-use-audit, python.lang.security.audit.dangerous-subprocess-use-tainted-env-args
    return subprocess.run(argv, shell=False, **kw)


def run_cmd(c, lot_dir, prompt_file):
    cmd = [sys.executable, os.path.join(HERE, "run_models.py"), "--lot-dir", lot_dir,
           "--prompt-file", prompt_file, "--model", c["model"], "--tag", c["name"]]
    if c.get("backend", "openai") == "ollama":
        return cmd + ["--backend", "ollama"]
    cmd += ["--backend", "openai", "--base-url", c["base_url"], "--key-env", c["key_env"],
            "--max-tokens", str(c.get("max_tokens", 3000))]
    if c.get("external"):
        cmd.append("--allow-external")
    return cmd


def results_path(c, lot_dir):
    backend = c.get("backend", "openai")
    name = c["name"].replace("/", "_").replace(":", "_") + "_file"
    return os.path.join(lot_dir, f"results_{backend}_{name}.jsonl")


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--candidates", default=os.path.join(HERE, "candidates.toml"))
    ap.add_argument("--lot-dir", required=True)
    ap.add_argument("--labels", required=True, help="export de la feuille d'étiquetage (humain)")
    ap.add_argument("--prompt-file", required=True)
    ap.add_argument("--run", action="store_true", help="exécute (sinon simulation)")
    ap.add_argument("--allow-external", action="store_true", help="autorise l'envoi aux candidats external=true")
    ap.add_argument("--report", default="")
    ap.add_argument("--min-cards", type=int, default=36,
                    help="taille minimale du lot pour proposer une promotion (calibration-protocol, #358 Q3 : >= 36)")
    ap.add_argument("--skip-witness", action="store_true",
                    help="n'exécute pas le témoin (ex. runner sans Ollama) : aucune promotion n'est alors proposée, "
                         "car le pouvoir discriminant du lot (§4) n'est pas mesuré")
    a = ap.parse_args()

    cfg = tomllib.load(open(a.candidates, "rb"))
    witness = cfg["witness"]
    cands = ([] if a.skip_witness else [witness]) + cfg["candidate"]
    lines = ["# Rapport de promotion", f"Lot : `{a.lot_dir}` — étiquettes : `{a.labels}`", ""]
    admitted = []
    wit = None   # (k, n) accord du témoin avec l'humain
    for c in cands:
        external = bool(c.get("external"))
        cmd = run_cmd(c, a.lot_dir, a.prompt_file)
        print(("[simulation] " if not a.run else "") + " ".join(cmd))
        if not a.run:
            continue
        if external and not a.allow_external:
            sys.exit(f"{c['name']} est external=true : relancez avec --allow-external pour confirmer l'envoi.")
        if external and not os.environ.get(c["key_env"]):
            sys.exit(f"variable {c['key_env']} absente : exportez-la dans votre shell.")
        _run(cmd, check=True)
        sc = _run([sys.executable, os.path.join(HERE, "score.py"),
                             "--key", os.path.join(a.lot_dir, "key.json"), "--labels", a.labels,
                             "--model", results_path(c, a.lot_dir)],
                            capture_output=True, text=True)
        out = sc.stdout
        verdict = re.search(r"seuil d'admission[^*]*\*\*(\w+)\*\*", out)
        verdict = verdict.group(1) if verdict else "INCONNU"
        lines += [f"## {c['name']} (`{c['model']}`) — {verdict}", "```", out.strip(), "```", ""]
        m = re.search(r"MODÈLE vs HUMAIN : (\d+)/(\d+)", out)
        k_n = (int(m.group(1)), int(m.group(2))) if m else None
        if c is witness:
            wit = k_n
        elif verdict == "ADMIS":
            admitted.append((c, k_n))
    if not a.run:
        print("\nSimulation : rien n'a été appelé. Ajoutez --run (et --allow-external) pour exécuter.")
        return

    lines.append("## Proposition (non appliquée)")
    n_cards = len(json.load(open(os.path.join(a.lot_dir, "items.json"), encoding="utf-8")))
    if n_cards < a.min_cards:
        admitted = []
        lines.append(f"Lot trop petit ({n_cards} cartes < {a.min_cards}) : échantillon sous-dimensionné, "
                     "aucune promotion proposée. Les verdicts ci-dessus restent indicatifs.")
    # §4, pouvoir discriminant : le lot doit faire tomber le témoin nettement sous le candidat
    # (intervalles de Wilson disjoints). Sans témoin mesuré, ou si le témoin fait aussi bien,
    # aucun candidat n'est proposé : le lot est « non discriminant ».
    if wit is None:
        retained = []
        lines.append("Témoin non mesuré : pouvoir discriminant du lot non vérifié (§4). "
                     "Aucune promotion proposée, même pour un candidat ADMIS.")
    else:
        wit_hi = wilson(*wit)[1]
        retained = [c for c, k_n in admitted if k_n and wilson(*k_n)[0] > wit_hi]
        dropped = [c["name"] for c, _ in admitted if c not in retained]
        if dropped:
            lines.append("Écartés faute de pouvoir discriminant (IC du candidat non disjoint de celui du témoin) : "
                         + ", ".join(dropped) + ".")
    if retained:
        best, rest = retained[0], retained[1:]
        lines += ["À reporter dans une PR relue par l'humain (la clé n'est jamais écrite : on référence la variable) :", "```",
                  "RGAA_LLM_PROVIDER=custom",
                  f"RGAA_LLM_MODEL={best['model']}", f"RGAA_LLM_BASE_URL={best['base_url']}",
                  f"RGAA_LLM_API_KEY=${{{best['key_env']}}}   # le fournisseur « custom » lit RGAA_LLM_API_KEY"]
        if rest:
            lines += ["RGAA_LLM_FALLBACK_PROVIDER=custom",
                      f"RGAA_LLM_FALLBACK_MODEL={rest[0]['model']}",
                      f"RGAA_LLM_FALLBACK_BASE_URL={rest[0]['base_url']}",
                      f"RGAA_LLM_FALLBACK_API_KEY=${{{rest[0]['key_env']}}}"]
        lines.append("```")
    elif n_cards < a.min_cards:
        pass   # déjà expliqué plus haut
    elif wit is not None and not admitted:
        lines.append("Aucun candidat admis : on garde le modèle actuel (jamais de bascule forcée).")
    elif wit is not None:
        lines.append("Aucun candidat retenu : on garde le modèle actuel.")
    rep = a.report or os.path.join(a.lot_dir, "promotion-report.md")
    open(rep, "w", encoding="utf-8").write("\n".join(lines) + "\n")
    print(f"rapport : {rep}")


if __name__ == "__main__":
    main()
