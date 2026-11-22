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
| OCR natif : `ocrs` et PP-OCRv5 latin (`oar-ocr`) | `ocr` | `ocrs` 0.14, `rten` 0.28, `oar-ocr` 0.11 |
| CLIP zéro-shot et similarité alt avec ancre de hasard | `clip` | `fastembed` 7.7 (encodeurs locaux, aucun appel HF) |

`cargo tree -d` confirme que `fastembed` et `oar-ocr` utilisent **la même**
version d'`ort` (2.0.0-rc.15). La feature `auto-download` de `oar-ocr` est
désactivée : aucun modèle n'est téléchargé à l'exécution.

Le format du snapshot est documenté dans `src/inventory.rs` :
`{"page": "…", "images": [{"type_noeud": "img", "alt": "…", …}]}`.
Le collecteur DOM qui le produit n'existe pas encore dans ce dépôt.

## Modèles

Douze fichiers sont déclarés dans `src/models.rs` (2 `ocrs`, 3 `oar-ocr`,
7 CLIP) avec URL, SHA-256, taille et licence. Ils sont **uniquement** installés
par `rgaa models pull images` (P-00 §6) dans `$RGAA_MODELS_DIR`. Pendant un
audit, un modèle absent donne une preuve `Estimee` dont `mesures.ocr` /
`mesures.clip` contient `preuve_manquante` ; les règles déterministes continuent
de tourner. Le plugin active pour cela `models_optional()` (nouveau hook du
trait P-00, `false` par défaut pour tous les autres plugins).

## Choix du moteur OCR

Le moteur par défaut est celui qui a le **meilleur CER français** sur le banc
du crate : `oar-ocr` (PP-OCRv5 latin), 0,001 contre 0,086 pour `ocrs`.
`rgaa.toml` permet de le remplacer (`$RGAA_CONFIG` peut désigner un autre
fichier) :

```toml
[analysis.images]
ocr_engine = "ocrs"   # ou "oar-ocr"
```

Si les modèles du moteur choisi sont absents mais ceux de l'autre sont
présents, l'autre moteur prend le relais. La preuve OCR porte toujours
`version_modele` = identifiant du moteur réellement utilisé (`ocrs` ou
`oar-ocr-v5-latin`), et `mesures.ocr.moteur` le répète.

Aucun appel LLM : ce plugin ne contient ni client Mistral ni envoi de
vignette. Un CAPTCHA (classes `recaptcha`/`hcaptcha`/`captcha`, URL ou champ
« recopiez ») n'est jamais OCRisé.

## Points d'extension

`OcrBackend` et `ClipBackend` sont des traits : le futur sidecar
`ocr-tesseract` (binaire `tesseract` ≥ 5 avec `tesseract-ocr-fra`) pourra être
injecté dans `ImagesPlugin::with_backends` sans changer le plugin. Il n'est pas
installé sur la machine de mesure et n'a donc pas été comparé.

## Bancs OCR

`tests/fixtures/ocr-fr/` (30 images accentuées) et
`tests/fixtures/ocr-control-en/` (5 images anglaises sans accent, contrôle du
rendu) contiennent des PNG générés (CC0) et leur vérité terrain écrite à la
main. Les deux bancs ne mesurent que des modèles déjà installés ; sans modèle,
les tests se terminent sans rien télécharger.

Relevé du 2026-11-22, après correction du rendu des fixtures (Liberation
Serif 40 px, noir sur blanc, 1280×128) :

| Banc | `ocrs` | `oar-ocr` |
|------|--------|-----------|
| Contrôle anglais (5) | 0,009 | 0,000 |
| Français (30) | 0,086 | 0,001 |

Le CER de contrôle initial (avant correction, Liberation Sans 32 px sur
768×96) était de **0,903** avec `ocrs` : le rendu des fixtures était bien en
cause. Les modèles `.rten` du dépôt Hugging Face `robertknight/ocrs` donnent
également de mauvais résultats avec `ocrs` 0.14 ; le plugin utilise les
fichiers ONNX officiels de `ocrs-cli`, vérifiés par SHA-256.
