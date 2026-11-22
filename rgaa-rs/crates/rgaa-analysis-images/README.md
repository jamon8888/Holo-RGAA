# rgaa-analysis-images — plugin P-03

Analyse des images et alternatives textuelles (critères RGAA 1.2, 1.3, 1.4,
1.7, 1.8). Le plugin produit des [`Preuve`](../rgaa-analysis/src/types.rs)
sans statut : l'orchestrateur décide seul du statut (R-P1).

## Périmètre

| Fonction | Module | Dépendance |
|----------|--------|------------|
| Inventaire DOM (`img`, `input[type=image]`, `svg`, `area`, `object`, `embed`, `canvas`, `role=img`, CSS porteuses) | `inventory` | snapshot DOM sérialisé |
| Règles déterministes (`alt-nom-fichier`, `alt-generique`, `alt-trop-long`, `alt-redondant`) | `regles` | aucune |
| pHash 64 bits, seuil de répétition ≤ 6, cache `(pHash, alt, contexte_hash)` | `phash`, `cache` | `image_hasher` 3.1 |
| Rasterisation SVG avant OCR/CLIP | `svg` | `resvg`/`usvg` 0.50 |
| OCR natif | `ocr` | `ocrs` 0.14 + `rten` 0.28 (**feature `rten_format` obligatoire** pour les modèles `.rten`) |
| CLIP zéro-shot et similarité alt avec ancre de hasard | `clip` | `fastembed` 7.7 (encodeurs locaux, aucun appel HF) |

Le format du snapshot est documenté dans `src/inventory.rs` :
`{"page": "…", "images": [{"type_noeud": "img", "alt": "…", …}]}`.
Le collecteur DOM qui le produit n'existe pas encore dans ce dépôt.

## Modèles

Neuf fichiers sont déclarés dans `src/models.rs` avec URL, SHA-256, taille et
licence. Ils sont **uniquement** installés par `rgaa models pull images`
(P-00 §6) dans `$RGAA_MODELS_DIR`. Pendant un audit, un modèle absent donne
une preuve `Estimee` dont `mesures.ocr` / `mesures.clip` contient
`preuve_manquante` ; les règles déterministes continuent de tourner. Le plugin
active pour cela `models_optional()` (nouveau hook du trait P-00, `false` par
défaut pour tous les autres plugins).

Aucun appel LLM : ce plugin ne contient ni client Mistral ni envoi de
vignette. Un CAPTCHA (classes `recaptcha`/`hcaptcha`/`captcha`, URL ou champ
« recopiez ») n'est jamais OCRisé.

## Points d'extension

`OcrBackend` et `ClipBackend` sont des traits : les futurs sidecars
`ocr-tesseract` (binaire `tesseract` ≥ 5 avec `tesseract-ocr-fra`) et `oar-ocr`
pourront être injectés dans `ImagesPlugin::with_backends` sans changer le
plugin. Le banc de comparaison des trois moteurs reste au ticket suivant.

## Banc OCR FR

`tests/fixtures/ocr-fr/` contient 30 images générées avec des accents, leur
vérité terrain et un README. `tests/ocr_fr_bench.rs` mesure le CER si les
modèles sont présents ; `tests/clip_real.rs` exécute une inférence CLIP réelle
dans les mêmes conditions. Sans modèle, ces tests se terminent sans rien
télécharger.

Relevé du 2026-11-22 avec les modèles installés : `ocrs` 0.14 avec son
alphabet anglais par défaut donne un CER de **1,046** sur les 30 phrases
accentuées (accents rendus `?`, lignes dupliquées par la détection). CLIP
ViT-B/32 charge les fichiers locaux et produit des scores finis. Ces mesures
sont des points de comparaison, pas des seuils de validation.
