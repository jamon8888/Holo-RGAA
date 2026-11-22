# P-03 — Plugin `images` (images et alternatives textuelles)

**Critères** : 1.2 (décorative ignorée), 1.3 (alternative pertinente), 1.4 (CAPTCHA / image-test), 1.7 (description détaillée pertinente), 1.8 (image texte remplaçable) · **Dépend de** : P-00 · **Modèles** : OCR, CLIP · **Feature** : `plugin-images`

## 1. Problème

Ce sont les critères les plus fréquents d'un audit, et ils reposent tous sur Holo (vision). En pratique, la moitié des cas se tranchent sans modèle génératif :

- l'image contient-elle du texte ?
- son `alt` décrit-il quelque chose qui ressemble à l'image ?
- l'image est-elle répétée sur 40 pages ?

## 2. Bibliothèques

| Crate | Version | Usage |
|-------|---------|-------|
| `oar-ocr` | 0.11.x | **OCR par défaut** (taux d'erreur de caractères de 0,001 sur le banc français, PR #474) : modèles PaddleOCR multilingues sur `ort` |
| `ocrs` + `rten` | 0.14.x / 0.28.x | OCR léger en pur Rust (détection + reconnaissance), modèles `.rten`, sans ONNX Runtime |
| `fastembed` | ≥ 4 (déjà au workspace ; 7.7 disponible) | CLIP ViT-B/32 : plongements image (`ImageEmbedding`) et texte, similarité cosinus `alt` ↔ image, zéro-shot « décorative / icône / photo / graphique / texte » |
| `image_hasher` | 3.1.x | pHash : dédoublonner les images entre pages, cache des décisions de revue |
| `resvg` / `usvg` | 0.50.x | rasteriser les `<svg>` inline et les `.svg` avant OCR et CLIP |
| `kamadak-exif` | 0.6.x | lire `ImageDescription` et `XPTitle` EXIF/IPTC (indice de contenu, sans valeur de preuve) |

**Points à vérifier dans le ticket** (bloquants pour la feature, pas pour le reste) :

1. **Banc d'essai OCR.** On compare trois moteurs sur 30 images du corpus, avec les accents français (é, è, à, ç, œ) :
   - `oar-ocr` (PaddleOCR, multilingue) ;
   - `ocrs` (pur Rust, entraîné surtout sur l'anglais) ;
   - le **sidecar `ocr-tesseract`** (P-00 §7 bis : binaire `tesseract` ≥ 5 avec `tesseract-ocr-fra`, sortie TSV convertie en preuves, sans lier Leptonica ni Tesseract).
   Le moteur qui a le plus faible taux d'erreur de caractères devient le défaut. Les deux autres restent sélectionnables dans `rgaa.toml`. Voir `bibliotheques.md`.
2. Les licences des modèles `ocrs` et CLIP doivent figurer dans `ModelSpec.licence` avant distribution.

## 3. Algorithme par image

On commence par l'**inventaire** (DOM, `Mesuree`). Il couvre :

- `img`, `input[type=image]`, `svg`, `area`, `object`, `embed`, `canvas` ;
- `role="img"` ;
- les images CSS porteuses (`background-image` sur un élément sans texte).

Pour chacune, on relève :

- le nom accessible (arbre AX), la `bbox` et la source ;
- la position : en lien, en bouton, dans un `figure` ;
- le texte voisin : `figcaption`, 300 caractères autour ;
- la taille affichée ;
- le nombre d'occurrences sur le site, par pHash.

Ensuite, quatre analyses, toutes `Estimee` :

| Signal | Méthode | Sert à |
|--------|---------|--------|
| `texte_ocr`, `couverture_texte` | `ocrs` ; surface des boîtes de mots / surface de l'image | 1.8 : une couverture > 50 % et un texte > 3 mots donnent `Suspect` « image texte » ; 1.3 : le texte OCR absent de l'alt donne `Suspect` |
| `classe_zero_shot` | CLIP, image vs prompts FR/EN (« une photo », « une icône », « un logo », « un graphique », « une capture d'écran de texte », « un motif décoratif, un séparateur ») | 1.2 : décorative probable mais avec un alt non vide, ou l'inverse |
| `similarite_alt` | cosinus CLIP(image) · CLIP(texte alt) ; on la compare au même score pour 5 alts **mélangés** de la même page (ancre de hasard) | 1.3 : une similarité inférieure à l'ancre donne `Suspect` « alt sans rapport » |
| `repetition` | nombre de pages où le pHash apparaît (distance ≤ 6) | décision partagée : une décision de revue sur une image vaut pour ses copies (gain de revue) |

Règles déterministes (`Mesuree`, sans modèle), en complément de l'existant :

- un alt égal au nom de fichier (`IMG_2034.jpg`, `banner-final-v2`) ;
- un alt composé uniquement de « image », « photo », « logo » ;
- un alt de plus de 150 caractères, qui suggère une description détaillée manquante (1.7) ;
- un alt identique au texte adjacent du lien, donc redondant.

**Appel LLM** (Mistral Large 4, vision). Uniquement pour les images **non** tranchées par les règles et marquées `Suspect` ou ambiguës. La requête contient :

- la vignette ≤ 768 px ;
- l'alt, le texte OCR et le texte voisin ;
- la question de la *Famille de preuve* « alternative d'image » (voir GLOSSARY, *Alternative pertinente* : cohérente avec ce que l'image montre **et** avec ce que la page en dit).

On regroupe jusqu'à 8 images par requête (limite API retenue). Le résultat va dans `mesures.avis_modele`.

**CAPTCHA (1.4).** Un CAPTCHA est repéré par :

- les classes et URL connues (`recaptcha`, `hcaptcha`, `captcha`) ;
- ou une image près d'un champ « recopiez ».

L'alt est envoyé au LLM avec la consigne « identifie-t-il la nature et la fonction sans donner la solution ? ». **L'image du CAPTCHA n'est jamais OCRisée** : pas de résolution de CAPTCHA.

## 4. `mesures`

```json
{ "type_noeud": "img", "nom_accessible": "Logo", "dans_lien": true, "occurrences_site": 41,
  "ocr": { "texte": "SOLDES -50%", "couverture": 0.62, "confiance": 0.88 },
  "clip": { "classe": "capture d'écran de texte", "score": 0.71, "similarite_alt": 0.19, "ancre_hasard": 0.22 },
  "regles": ["alt-nom-fichier"], "avis_modele": { "pertinente": "non", "manque": "le pourcentage de remise" } }
```

## 5. Cache et coût

- La clé de cache est `(pHash, alt, contexte_hash)`. Une même image avec le même alt n'est analysée et envoyée au LLM qu'**une fois par audit**.
- Une décision de revue humaine sur cette clé est proposée pour les autres occurrences. C'est le relecteur qui confirme l'application en lot (R5 : décision nominative).
- Cible : appels LLM vision ≤ 25 % des images inventoriées, mesurés sur les 3 sites du corpus.

## 6. Tests

| Cas | Attendu |
|-----|---------|
| Bannière JPEG « SOLDES -50% » avec `alt=""` | OCR couverture > 0,5 ; `Suspect` 1.8 et 1.2 |
| Photo de chat, `alt="chat roux sur un canapé"` | similarité > ancre ; `Rassurant` |
| Photo de chat, `alt="Rapport annuel 2025"` | similarité < ancre ; `Suspect` |
| `alt="DSC_0042.JPG"` | règle `alt-nom-fichier`, `Mesuree` |
| Logo répété sur 41 pages | un seul appel LLM ; `occurrences_site=41` |
| SVG inline avec `<text>` | rasterisé (`resvg`) puis OCR |
| reCAPTCHA | pas d'OCR, question 1.4 seulement |

## 7. Critères d'acceptation

1. Sans modèle téléchargé, les règles déterministes tournent, et OCR et CLIP sont marqués « preuve manquante ».
2. Le débit dépasse 20 images/s en OCR et 50 images/s en CLIP, sur CPU 8 cœurs (`--release`), avec des images ≤ 768 px.
3. Le corpus 1.x existant n'a aucune régression. Les preuves sont attachées à chaque test 1.2, 1.3, 1.7 et 1.8 en `review`.
4. Le nombre d'images envoyées au LLM est ≤ 25 % de l'inventaire, mesuré et noté dans le ticket.
