# P-01 — Plugin `contrast` (contraste)

**Critères** : 3.2 (texte), 3.3 (composants d'interface et éléments graphiques) · **Dépend de** : P-00 · **Modèle** : aucun · **Feature** : `plugin-contrast` (défaut)

## 1. Problème

Aujourd'hui, 3.2 et 3.3 reposent sur la règle axe `color-contrast`. Elle ne mesure que le texte sur un fond CSS uni et renvoie *incomplete* dans trois cas : fond image, dégradé, chevauchement. Elle ne traite pas 3.3 (composants, icônes, graphiques), ni les états survol et focus. Ce sont justement les cas qu'un auditeur vérifie à la pipette.

## 2. Bibliothèques

| Crate | Version | Usage |
|-------|---------|-------|
| `palette` | 0.7.7 | sRGB ↔ linéaire, luminance relative ; base de P-02 |
| `image` | 0.25.x | découpe de la capture, accès aux pixels |
| `imageproc` | 0.27.x | binarisation d'Otsu, composantes connexes (séparer le texte du fond) |

Les crates `wcag-contrast` et `perceive-color` ont été écartées : moins de 35 000 téléchargements, mainteneur unique. La formule WCAG 2 tient en 10 lignes, on l'écrit sur `palette`. APCA est **hors périmètre**, car le RGAA 4.1.2 se réfère à WCAG 2.1.

## 3. Algorithme

**Étape A — mesure CSS (`Portee::Mesuree`).** Elle s'applique à chaque nœud texte visible du `DOMSnapshot` dont les ancêtres jusqu'au premier fond opaque n'ont ni `background-image`, ni `opacity` < 1, ni `mix-blend-mode`, ni `filter`.

- Couleur du texte et fond composé : on fait l'*alpha-compositing* des ancêtres, du haut vers le bas.
- Ratio `(L1 + 0,05) / (L2 + 0,05)`, avec L la luminance relative WCAG 2.1. La linéarisation sRGB utilise le seuil 0,04045.
- Seuils RGAA 3.2 : **4,5:1**. Le seuil passe à **3:1** si le texte est grand :
  - ≥ 24 px ;
  - ou ≥ 18,66 px (14 pt) avec `font-weight` ≥ 700.
  La taille retenue est la taille calculée, en px CSS, pas en pixels physiques.
- Pas de seuil pour un texte désactivé (`disabled`, `aria-disabled=true`), un logotype ou un texte décoratif. Pour ces cas, on rend une preuve avec `indice = Neutre` et la raison.

**Étape B — mesure sur pixels (`Portee::Estimee`).** Elle s'applique quand l'étape A est inapplicable : fond image, dégradé, texte sur vidéo, `opacity`, chevauchement.

1. On découpe la capture sur la `bbox` du nœud × `dpr`.
2. On binarise par Otsu sur la luminance. La classe minoritaire connexe est le texte, la majoritaire le fond. On vérifie la cohérence avec la couleur CSS du texte : écart ΔE2000 < 10, calculé avec `palette`. Si ce n'est pas cohérent, l'indice vaut `Suspect` « séparation texte/fond incertaine ».
3. Luminance du texte = médiane des pixels texte. Fond : on calcule les percentiles **5 et 95** de luminance, puis on garde le ratio **le plus défavorable**. C'est la pratique « pire zone » des auditeurs.
4. On produit un artefact : la vignette de la zone (PNG) avec le masque texte en surimpression.

**Étape C — composants non textuels (3.3, `Estimee`).** Elle couvre les bordures de champs, les icônes `svg` ou `img` de moins de 64 px de côté, les indicateurs d'état (cases, interrupteurs) et les contours de focus.

- On compare la couleur de l'élément (CSS si possible, sinon pixels) à celle des **pixels adjacents**, sur un anneau de 3 px CSS autour de la `bbox`.
- Le seuil est **3:1**.

**Étape D — états.** C'est une option `etats: ["hover", "focus"]` (par défaut `["focus"]`). L'orchestrateur rejoue :

- `DOM.focus` pour le focus ;
- `Input.dispatchMouseEvent` pour le survol, sur les éléments interactifs.

Le plugin reçoit une capture par état et refait les étapes A à C. La preuve porte `etat`.

## 4. `mesures` (schéma)

```json
{ "etape": "A|B|C", "etat": "repos|hover|focus", "ratio": 3.12, "seuil": 4.5,
  "texte_grand": false, "couleur_avant": "#7a7a7a", "couleur_fond": "#ffffff",
  "fond_p5": "#…", "fond_p95": "#…", "taille_px": 16, "graisse": 400 }
```

## 5. Statut décidé par le moteur (rappel R-P1/R-P2)

- `A` + ratio < seuil → **fail**. 3.2 est un test Automatique quand la mesure est CSS. C'est le comportement d'axe aujourd'hui, mais avec une couverture plus large.
- `A` + ratio ≥ seuil → **pass** pour ce nœud.
- `B` / `C` → **review**, avec l'indice `Suspect` si ratio < seuil et `Rassurant` si ratio ≥ seuil × 1,2.
- Le LLM n'est **pas** appelé par ce plugin.

## 6. Tests (corpus `rgaa-test-corpus`)

| Cas | Attendu |
|-----|---------|
| `#767676` sur `#fff`, 16 px | A, ratio 4,54, ≥ 4,5 |
| `#777` sur `#fff`, 16 px | A, ratio 4,48, < 4,5 |
| `#777` sur `#fff`, 24 px | A, seuil 3, ≥ seuil |
| Texte blanc sur photo claire (fixture PNG) | B, p95 donne un ratio < 3, `Suspect` |
| Dégradé noir vers blanc derrière un texte gris | B, ratio le plus défavorable retenu |
| Bordure d'input `#ccc` sur `#fff` | C, ratio 1,6, < 3 |
| Contour de focus absent | D `focus`, ratio 1,0 |
| Bouton `disabled` gris pâle | `Neutre`, raison « désactivé » |

Les valeurs de ratio servent d'oracle externe : calculées avec le WebAIM Contrast Checker, elles sont **codées en dur** dans les tests (pas de recalcul tautologique).

## 7. Critères d'acceptation

1. Sur le corpus existant des critères 3.2 et 3.3, il n'y a aucune régression par rapport à axe : tout `fail` d'axe reste `fail`.
2. Parmi les nœuds qu'axe renvoie en *incomplete*, au moins 80 % reçoivent une preuve B ou C.
3. Temps du plugin inférieur à 300 ms par page sur une page de 500 nœuds texte, en `--release`, hors capture.
4. Clippy est propre, aucun `unwrap` hors tests, et les sorties sont déterministes (`IndexMap`, ordre DOM).
