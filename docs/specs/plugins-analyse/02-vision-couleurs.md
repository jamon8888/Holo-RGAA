# P-02 — Plugin `colorvision` (daltonisme et information par la couleur)

**Critères** : 3.1 (information ne devant pas être donnée uniquement par la couleur) ; appui à 3.3 et 10.9 · **Dépend de** : P-00 · **Modèle** : aucun · **Feature** : `plugin-colorvision` (défaut)

## 1. Problème

Pour 3.1, le moteur est aujourd'hui `Holo` (vision : « l'information survit-elle en niveaux de gris ? »). Envoyer une capture brute au LLM coûte cher et n'est pas reproductible. On veut d'abord **simuler** ce que voit une personne daltonienne, **mesurer** ce qui disparaît, puis ne montrer au LLM et au relecteur que les zones où l'information s'effondre.

## 2. Bibliothèques

| Crate | Version | Usage |
|-------|---------|-------|
| `palette` | 0.7.7 | conversions sRGB linéaire, Lab ; ΔE CIEDE2000 (`ImprovedCiede2000`) |
| `image` | 0.25.x | application des matrices sur la capture |
| `image_hasher` | 3.1.x | hachage perceptuel : savoir si deux vignettes deviennent indiscernables |

Les crates de simulation existantes sont abandonnées depuis 2020 (`colorblind`, `color_blinder`) ou trop marginales (`colorverse`). On implémente donc les matrices publiées. Il y en a 4, de 3×3, issues de la littérature : environ 60 lignes, testables contre les valeurs de référence.

## 3. Simulations

Appliquées en **sRGB linéaire** (linéariser, multiplier par la matrice, puis ré-encoder) :

| Type | Méthode | Prévalence (indicative, hommes) |
|------|---------|----------------------------------|
| Deutéranopie | Machado et al. 2009, sévérité 1,0 | ~1 % (deutéranomalie ~5 %) |
| Protanopie | Machado et al. 2009, sévérité 1,0 | ~1 % |
| Tritanopie | Brettel et al. 1997, avec la variante Viénot | < 0,01 % |
| Achromatopsie | luminance Rec. 709 (`0,2126 R + 0,7152 G + 0,0722 B` linéaire) | rare ; c'est la référence « niveaux de gris » de 3.1 |

On propose aussi une option `severite: 0.0–1.0` pour les anomalies (Machado fournit les matrices par pas de 0,1).

## 4. Algorithme

1. **Repérer les candidats** (DOM, `Mesuree`) : ce sont les éléments qui se distinguent de leurs voisins **seulement** par la couleur.
   - Liens dans un paragraphe sans soulignement, sans graisse ou taille différente, et sans icône. Le ratio de contraste lien/texte doit être < 3:1 (règle WCAG F73).
   - Champs en erreur signalés par une bordure rouge sans texte ni icône.
   - Légendes de graphiques (`svg`, `canvas`) dont les séries ne diffèrent que par `fill` ou `stroke`.
   - Pastilles d'état (`span` coloré sans texte).
2. **Mesurer la perte** (`Estimee`). Pour chaque paire candidat/voisin et chaque simulation, on calcule le ΔE2000 entre leurs couleurs simulées.
   - ΔE < 5 donne « indiscernable sous <type> ». Le seuil est en paramètre (5 = proche de la limite de perception courante).
   - Pour les graphiques, on hache (`image_hasher`, dHash) chaque série isolée après simulation. Deux séries à distance de Hamming ≤ 4 sont confondues.
3. **Produire les artefacts** : une vignette originale et une vignette par simulation où l'information disparaît, soit 3 PNG au maximum par sujet.
4. **Appel LLM** (optionnel, `opts.llm = true`, par défaut vrai pour 3.1). Uniquement pour les sujets marqués « indiscernable », on envoie à Mistral Large 4 :
   - la vignette originale et la vignette achromatopsie (2 images, sous la limite de 8 par requête) ;
   - l'extrait de texte environnant ;
   - la question « l'information portée par la couleur est-elle aussi donnée autrement (texte, forme, motif) ? ».
   La réponse est enregistrée dans `mesures.avis_modele`. **Elle n'est pas un verdict** (R2/R3).

## 5. `mesures`

```json
{ "candidat": "lien-sans-soulignement|champ-erreur|serie-graphique|pastille",
  "simulations": [{ "type": "deuteranopie", "delta_e": 2.1, "indiscernable": true }],
  "ratio_lien_texte": 1.8, "avis_modele": { "autre_moyen": "aucun", "explication": "…" } }
```

## 6. Utilisation hors 3.1

- La **page de revue** (P-09) propose un sélecteur « voir comme : deutéranope / protanope / tritanope / niveaux de gris ». Il applique les mêmes matrices côté navigateur (CSS `filter: url(#svg-matrix)`). Les coefficients sont exportés par ce crate dans `types/colorvision-matrices.json`, pour ne pas avoir deux sources.
- P-01 peut demander le ΔE d'une paire, via une fonction publique `delta_e_2000(a, b)`.

## 7. Tests

| Cas | Attendu |
|-----|---------|
| Matrice deutan appliquée à `#ff0000` | valeur de référence publiée par Machado 2009 (oracle codé en dur, tolérance ±1/255) |
| Lien `#0000ee` dans du texte `#000`, non souligné | candidat F73, ratio < 3:1, indiscernable en achromatopsie |
| Même lien, souligné | pas de candidat |
| Graphique à barres rouge/vert sans motif (fixture SVG) | séries confondues sous deutan et protan |
| Champ en erreur avec bordure rouge et message texte | candidat, mais `Rassurant` « texte d'erreur présent » |

## 8. Critères d'acceptation

1. Les matrices sont égales aux références publiées, à ±1/255 près, sur 16 couleurs de test.
2. Le plugin traite une capture 1280×4000 en moins de 400 ms (`--release`), simulation pleine page comprise.
3. Pour 3.1, le nombre d'images envoyées au LLM baisse d'au moins 70 % par rapport à l'envoi de la capture pleine page. Mesure sur les 3 sites du corpus.
4. Il n'y a aucun statut dans la preuve ; 3.1 reste `review` (Assisté).
