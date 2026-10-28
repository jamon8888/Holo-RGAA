#!/usr/bin/env bash
# Lance les harnais DOM Node des sondes JS embarquées. Pas de navigateur, pas de
# binaire Obscura : quelques secondes. Lancé par le job CI `probe-harness`, et à
# la main depuis ce répertoire après `npm install`.
set -euo pipefail
cd "$(dirname "$0")"

# Exclusions, chacune avec sa raison. Tout le reste est lancé : un nouveau
# harnais déposé ici est pris automatiquement, plutôt qu'oublié en silence.
#
# - plan_snippets_check.js : outil de diagnostic, pas un garde. Il exige
#   `pass === true` sur chaque fixture `-pass`, alors que les sondes en revue
#   seule ne rendent délibérément jamais `pass` (c'est tout l'objet de la pile
#   #365→#377) ; et il compte « NO FIXTURE » comme un problème pour les 24
#   sondes sans fixture déclarée. Il rapporte 54 problèmes sur un arbre sain :
#   le câbler tel quel rendrait la CI rouge en permanence. À réécrire en garde
#   (attendre `outcome`, pas `pass`) ou à retirer — suivi séparément.
EXCLUDED=(plan_snippets_check.js)

for excluded in "${EXCLUDED[@]}"; do
  if [ ! -f "$excluded" ]; then
    echo "exclusion périmée : $excluded n'existe plus, retirer la ligne" >&2
    exit 1
  fi
done

shopt -s nullglob
to_run=()
for harness in *.js; do
  skip=0
  for excluded in "${EXCLUDED[@]}"; do
    [ "$harness" = "$excluded" ] && skip=1
  done
  [ "$skip" -eq 0 ] && to_run+=("$harness")
done

if [ "${#to_run[@]}" -eq 0 ]; then
  echo "aucun harnais à lancer — ce script passerait à vide" >&2
  exit 1
fi

for harness in "${to_run[@]}"; do
  echo "── $harness"
  node "$harness"
done
echo "${#to_run[@]} harnais ok"
