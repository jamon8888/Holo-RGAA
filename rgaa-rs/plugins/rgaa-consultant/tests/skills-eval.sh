#!/usr/bin/env bash
# skills-eval.sh — évaluation contractuelle des skills d'analyse (spec P-09 §4.1).
#
# Le dépôt n'a pas (encore) de moteur d'évaluation de prompts : ce script est le
# test exécutable, déterministe et sans modèle, qui vérifie les trois points
# exigés par la spec pour CHAQUE skill, sur une fixture locale :
#   1. elle appelle `run_analysis_plugin` avec le bon identifiant de plugin ;
#   2. elle n'écrit aucun statut (R7) et laisse la décision au relecteur (D2) ;
#   3. elle affiche `rgaa models pull <plugin>` quand le plugin a des modèles.
#
# Il vérifie aussi le prérequis `list_analysis_plugins` et l'avertissement
# photosensible de `mouvement`. Pour une évaluation agentique complète, voir
# tests/README-eval.md (procédure et mêmes fixtures).

set -uo pipefail

PLUGIN_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
FIXTURE="$PLUGIN_ROOT/tests/fixtures/preuves-analyse.json"
FAILURES=0

fail() {
  echo "  ❌ $1"
  FAILURES=$((FAILURES + 1))
}
pass() {
  echo "  ✅ $1"
}

# skill -> identifiant du plugin d'analyse (source indépendante : specs P-01..P-08)
plugin_of() {
  case "$1" in
    contraste)  echo "contrast" ;;
    daltonisme) echo "colorvision" ;;
    images)     echo "images" ;;
    medias)     echo "media" ;;
    mouvement)  echo "motion" ;;
    langue)     echo "lang" ;;
    structure)  echo "layout" ;;
    pertinence) echo "text" ;;
    *)          echo "" ;;
  esac
}

# plugins dont les modèles sont téléchargés séparément (specs P-03, P-04, P-08)
has_models() {
  case "$1" in
    images|media|text) return 0 ;;
    *) return 1 ;;
  esac
}

echo "=== Évaluation des skills d'analyse (P-09 §4.1) ==="

for skill in contraste daltonisme images medias mouvement langue structure pertinence; do
  echo "--- $skill ---"
  file="$PLUGIN_ROOT/skills/$skill/SKILL.md"
  if [[ ! -f $file ]]; then
    fail "$skill : skills/$skill/SKILL.md absent"
    continue
  fi
  if ! grep -q '^name:' "$file" || ! grep -q '^description:' "$file"; then
    fail "$skill : front matter name/description manquant"
  fi
  if ! grep -q 'list_analysis_plugins' "$file"; then
    fail "$skill : ne vérifie pas list_analysis_plugins"
  fi
  plugin=$(plugin_of "$skill")
  if [[ -z $plugin ]]; then
    fail "$skill : correspondance de plugin inconnue (table de test obsolète)"
    continue
  fi
  if ! grep -q "run_analysis_plugin" "$file" || ! grep -q "\"$plugin\"" "$file"; then
    fail "$skill : n'appelle pas run_analysis_plugin { plugin: \"$plugin\" }"
  fi
  # R7 / D2 : la skill prépare, elle n'écrit jamais de statut.
  if ! grep -qi "n'écrit jamais" "$file" || ! grep -q 'R7' "$file"; then
    fail "$skill : ne rappelle pas la règle R7 (aucune écriture de statut)"
  fi
  if ! grep -q 'D2' "$file"; then
    fail "$skill : ne renvoie pas la décision au relecteur (D2)"
  fi
  # Modèles absents -> commande exacte.
  if has_models "$plugin"; then
    if ! grep -q "rgaa models pull $plugin" "$file"; then
      fail "$skill : n'affiche pas « rgaa models pull $plugin » quand le modèle manque"
    fi
  fi
done

echo "--- sécurité spécifique ---"
if ! grep -qi 'photosensible' "$PLUGIN_ROOT/skills/mouvement/SKILL.md" 2>/dev/null; then
  fail "mouvement : avertissement photosensible absent"
fi
# R3 : aucun verdict IA affiché ; l'avis du modèle reste replié et indicatif.
if ! grep -qi 'indicatif' "$PLUGIN_ROOT/skills/images/SKILL.md" 2>/dev/null; then
  fail "images : avis du modèle non étiqueté « indicatif » (R3)"
fi

echo "--- fixture JSON (schéma Preuve, P-00 §5) ---"
if [[ ! -f $FIXTURE ]]; then
  fail "fixture tests/fixtures/preuves-analyse.json absente"
else
  if python3 -I - "$FIXTURE" <<'PY'
import json, sys
data = json.load(open(sys.argv[1], encoding="utf-8"))
required = {"plugin", "version_plugin", "version_modele", "tests", "sujet",
            "portee", "mesures", "indice", "artefacts", "empreinte_entree"}
items = data if isinstance(data, list) else [data]
assert items, "fixture vide"
for item in items:
    missing = required - set(item)
    assert not missing, f"champs manquants: {sorted(missing)}"
    assert item["portee"] in ("Mesuree", "Estimee"), "portee invalide"
    assert isinstance(item["tests"], list) and item["tests"], "tests vide"
    assert isinstance(item["mesures"], dict), "mesures doit être un objet"
PY
  then
    pass "fixture conforme au schéma Preuve"
  else
    fail "fixture invalide au regard du schéma Preuve"
  fi
fi

echo
if [[ $FAILURES -eq 0 ]]; then
  echo "✅ ÉVALUATION DES SKILLS: tous les contrôles passent"
  exit 0
fi
echo "❌ $FAILURES contrôle(s) d'évaluation échoué(s)"
exit 1
