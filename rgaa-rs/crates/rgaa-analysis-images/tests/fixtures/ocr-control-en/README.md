# Banc de contrôle OCR (5 images, anglais)

## Rôle

Ces cinq phrases anglaises **sans accent** sont générées par exactement le
même pipeline que le banc français : voir `../../examples/generate_ocr_fr.rs`
avec l'option `--control`. Elles servent à distinguer un défaut de rendu des
fixtures d'un défaut du moteur OCR : si le CER y est mauvais, le banc lui-même
est en cause.

## Rendu

- 1280 × 128 pixels, texte noir sur fond blanc ;
- police **Liberation Serif** (SIL OFL 1.1), 40 px, pleine ligne visible ;
- vérité terrain dans `ground_truth.json`, écrite à la main.

Images générées par ce dépôt, publiées sous **CC0 1.0**.

## Mesures du 2026-11-22

- Rendu d'origine (Liberation Sans 32 px, 768 × 96) : `ocrs` CER = **0,903**.
- Rendu corrigé (ci-dessus) : `ocrs` CER = **0,009**, `oar-ocr` CER = **0,000**.

Le test `tests/ocr_control_bench.rs` exige un CER < 0,05 pour chaque moteur
quand ses modèles sont installés.
