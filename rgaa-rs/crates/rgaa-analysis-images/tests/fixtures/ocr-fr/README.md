# Banc OCR français (30 images)

## Origine et licence

Aucune image tiers n'est redistribuée. Les 30 PNG ont été **générés par ce
dépôt** le 2026-11-22, à partir deSVG contenant du texte, puis rasterisés avec
`resvg`/`usvg` 0.50 (voir `../../examples/generate_ocr_fr.rs`). Ils sont
publiés sous **CC0 1.0**.

La police utilisée est **Liberation Sans** (SIL Open Font License 1.1),
installée sur la machine de génération ; une police équivalente est utilisée
en repli (`DejaVu Sans`, puis `sans-serif`). Les chaines de vérité terrain
sont écrites à la main dans `ground_truth.json` : elles ne sont pas calculées
à partir de la sortie d'un moteur OCR.

## Contenu

- `01.png` … `30.png` : 768 × 96 pixels, texte noir sur fond blanc.
- `ground_truth.json` : `[{ "fichier": "01.png", "texte": "…" }, …]`.

Les accents sont ceux de la spec : `é`, `è`, `à`, `ç`, `œ` (et leurs
majuscules `É`, `È`, `À`, `Ç`, `Œ`). Les images 02, 15 et 16 montrent
respectivement `ÉTÉ`/`À`, `château`/`où`, et `ŒUFS`.

## Régénération

```sh
export CARGO_TARGET_DIR=/root/.cache/rgaa-target-p03 CARGO_INCREMENTAL=0
cargo run -p rgaa-analysis-images --example generate_ocr_fr
```

Le générateur charge les polices du système : sans cela, `usvg` supprime
silencieusement les éléments `<text>` et produit des images blanches.

## Usage

`tests/ocr_fr_bench.rs` vérifie le corpus et, **uniquement si les modèles sont
déjà installés**, mesure le taux d'erreur de caractères (CER, distance de
Levenshtein) de `ocrs`. Sans modèle, le test affiche un message et revient :
aucun téléchargement n'a lieu pendant un audit (R-P3). Les résultats comparatifs
`oar-ocr` et sidecar `tesseract` restent à faire dans le ticket suivant.

Relevé du 2026-11-22 avec les modèles `ocrs` installés : 30/30 images lues,
**CER = 1,046**. L'alphabet anglais par défaut d'`ocrs` remplace les accents
par `?` et la détection duplique certaines lignes. Ce chiffre est une mesure de
référence, pas un seuil : il documente pourquoi le banc doit comparer les trois
moteurs avant de choisir le défaut.
