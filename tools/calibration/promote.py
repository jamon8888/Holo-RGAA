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
import argparse, os, re, subprocess, sys, tomllib

HERE = os.path.dirname(os.path.abspath(__file__))


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
    a = ap.parse_args()

    cfg = tomllib.load(open(a.candidates, "rb"))
    cands = [cfg["witness"]] + cfg["candidate"]
    lines = ["# Rapport de promotion", f"Lot : `{a.lot_dir}` — étiquettes : `{a.labels}`", ""]
    admitted = []
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
        subprocess.run(cmd, check=True)
        sc = subprocess.run([sys.executable, os.path.join(HERE, "score.py"),
                             "--key", os.path.join(a.lot_dir, "key.json"), "--labels", a.labels,
                             "--model", results_path(c, a.lot_dir)],
                            capture_output=True, text=True)
        out = sc.stdout
        verdict = re.search(r"seuil d'admission[^*]*\*\*(\w+)\*\*", out)
        verdict = verdict.group(1) if verdict else "INCONNU"
        lines += [f"## {c['name']} (`{c['model']}`) — {verdict}", "```", out.strip(), "```", ""]
        if verdict == "ADMIS" and c is not cfg["witness"]:
            admitted.append(c)
    if not a.run:
        print("\nSimulation : rien n'a été appelé. Ajoutez --run (et --allow-external) pour exécuter.")
        return

    lines.append("## Proposition (non appliquée)")
    if admitted:
        best, rest = admitted[0], admitted[1:]
        lines += ["À reporter dans une PR relue par l'humain :", "```",
                  f"RGAA_LLM_MODEL={best['model']}", f"RGAA_LLM_BASE_URL={best['base_url']}"]
        if rest:
            lines += [f"RGAA_LLM_FALLBACK_MODEL={rest[0]['model']}",
                      f"RGAA_LLM_FALLBACK_BASE_URL={rest[0]['base_url']}"]
        lines.append("```")
        lines.append("Vérifier aussi : le témoin doit tomber nettement sous le meilleur candidat (§4, pouvoir discriminant).")
    else:
        lines.append("Aucun candidat admis : on garde le modèle actuel (jamais de bascule forcée).")
    rep = a.report or os.path.join(a.lot_dir, "promotion-report.md")
    open(rep, "w", encoding="utf-8").write("\n".join(lines) + "\n")
    print(f"rapport : {rep}")


if __name__ == "__main__":
    main()
