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

- `01.png` … `30.png` : 1280 × 128 pixels, texte noir sur fond blanc,
  Liberation Serif 40 px (police à empattement, empattements et accents lisibles).
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

Relevé du 2026-11-22 après correction du rendu (auparavant Liberation Sans
32 px sur 768 × 96, CER `ocrs` = 1,046) :

- `ocrs` (modèles ONNX officiels) : **CER = 0,086** ;
- `oar-ocr` (PP-OCRv5 latin) : **CER = 0,001** ;
- contrôle anglais (5 images) : 0,009 / 0,000.

`oar-ocr` est donc le moteur par défaut, configurable dans `rgaa.toml`.
`ocrs` reste disponible. Le sidecar `tesseract` n'est pas installé sur la
machine de mesure : il n'a pas été comparé et reste un point d'extension.
