# P-07 — Plugin `layout` (structure visuelle et accessibilité exposée)

**Critères** :

- 5.3 linéarisation d'un tableau de mise en forme ;
- 5.6 en-têtes de tableau visuels balisés en `td` ;
- 7.1 composant compatible avec les technologies d'assistance ;
- 7.5 messages de statut restitués ;
- 9.1 hiérarchie de titres ;
- 9.2 structure du document ;
- 10.3 ordre de lecture compréhensible ;
- 10.9 et 10.10 information par la forme, la taille ou la position.

**Dépend de** : P-00 · **Modèle** : aucun (LLM optionnel) · **Feature** : `plugin-layout` (défaut) · **Forme** : natif

## 1. Principe

Ces critères comparent **ce que l'on voit** (rendu) et **ce qui est exposé** (DOM, arbre d'accessibilité). Les deux sont déjà disponibles par CDP. Le plugin calcule les **écarts** entre les deux vues. Le LLM vision ne sert qu'à départager les cas restants.

## 2. Sources (toutes via Obscura / CDP, pas de nouvelle dépendance lourde)

| Source | Appel CDP | Contenu |
|--------|-----------|---------|
| Rendu | `DOMSnapshot.captureSnapshot` avec `computedStyles` (`font-size`, `font-weight`, `display`, `position`, `visibility`), `includeDOMRects` | boîtes, styles |
| Arbre AX | `Accessibility.getFullAXTree` | rôle, nom, états, propriétés |
| Capture | `Page.captureScreenshot` | pixels (vignettes, `imageproc` pour les régions) |
| Mutations | `Runtime` + `MutationObserver` injecté (7.5) | ajouts de texte, attributs `aria-live`, `role=status|alert` |

## 3. Analyses

| Critère | Mesure (`Mesuree` si DOM pur, sinon `Estimee`) | Indice `Suspect` si… |
|---------|------------------------------------------------|------------------------|
| **9.1** | Titres **visuels** : blocs de texte court (< 120 caractères), seuls sur leur ligne, avec une taille ≥ 1,25 × la taille médiane du corps, ou une graisse ≥ 600 avec une taille > médiane. On les compare aux titres **exposés** (`h1-h6`, `role=heading`). | titre visuel non exposé ; titre exposé dont la taille visuelle est inférieure à celle d'un titre de niveau inférieur (inversion) ; saut de niveau |
| **9.2** | Zones visuelles (en-tête, navigation, pied) repérées par position et répétition entre pages, comparées aux points de repère AX (`banner`, `navigation`, `main`, `contentinfo`) | zone répétée sans repère |
| **10.3** | Ordre du DOM ou du focus comparé à l'ordre visuel (tri par lignes : `y` puis `x`, tolérance de 8 px ; sens de lecture d'après `dir`). On compte les inversions, par tau de Kendall. | tau < 0,8 ; ou élément positionné (`absolute`, `order`, `grid-area`) qui saute plus de 3 rangs |
| **5.6** | Dans un `table` de données, cellules `td` de la 1re ligne ou de la 1re colonne qui ont le style d'un en-tête : graisse ≥ 600, fond différent du corps du tableau (ΔE > 10, P-02) | style d'en-tête sans `th` ni `scope` |
| **5.3** | Tableau de mise en forme (`role=presentation`) : on linéarise le DOM et on compare à l'ordre visuel, comme pour 10.3 | inversion |
| **7.1** | Widget personnalisé (élément cliquable `div`/`span` : `cursor:pointer` ou gestionnaire `click`, sans rôle natif). On vérifie le rôle AX, le nom, l'état (`aria-expanded`, `aria-pressed`, `aria-checked`) **avant et après** un clic simulé. | rôle générique ; état AX inchangé alors que le rendu change (différence de pixels > 2 % de la bbox) |
| **7.5** | Après chaque action de l'orchestrateur (soumission de formulaire, ajout au panier, filtre), on capte le texte apparu, puis on regarde s'il se trouve dans une région `aria-live`, `role=status` ou `role=alert`, ou s'il reçoit le focus. | message visible non restitué |
| **10.9 / 10.10** | Textes du DOM qui renvoient à la forme, la position ou la taille : expressions régulières FR/EN (« ci-dessous », « à droite », « le bouton rond », « en rouge », « l'icône verte »), via P-06 pour la langue | référence trouvée ; le LLM vérifie ensuite si une alternative textuelle existe |

**LLM** (option, par défaut seulement pour 9.1, 10.3 et 10.9/10.10 `Suspect`). On envoie à Mistral Large 4 :

- la vignette de la zone concernée ;
- la liste des titres visuels face aux titres exposés, ou l'extrait de texte.

La réponse va dans `mesures.avis_modele`. Le plugin envoie au plus 8 images par requête et ne transmet **jamais** la page entière.

## 4. `mesures` (exemple 9.1)

```json
{ "titres_visuels": [{ "texte": "Nos offres", "taille_px": 28, "graisse": 700, "expose": false }],
  "titres_exposes": [{ "niveau": 2, "texte": "Contact", "taille_px": 14 }],
  "inversions": 1, "sauts_niveau": ["h1→h3"] }
```

## 5. Tests

| Cas | Attendu |
|-----|---------|
| `<div class="big">Nos offres</div>` à 28 px gras | 9.1 titre visuel non exposé |
| `h2` à 14 px et `h3` à 24 px | inversion |
| `order: -1` qui fait remonter le pied de page | 10.3 tau < 0,8 |
| `div` cliquable qui ouvre un menu sans `aria-expanded` | 7.1 état inchangé |
| Message « Ajouté au panier » sans `aria-live` | 7.5 `Suspect` |
| « Cliquez sur le bouton vert à droite » | 10.9 candidat |
| Tableau dont la 1re ligne en `td` est grasse sur fond gris | 5.6 `Suspect` |

## 6. Critères d'acceptation

1. Les 7 cas sont verts. Il n'y a aucune régression sur les tests déterministes existants de 9.x et 10.x.
2. Le plugin traite une page de 3 000 nœuds en moins de 500 ms (`--release`), hors actions simulées.
3. Les actions simulées de 7.1 et 7.5 sont bornées (20 widgets maximum par page, réglable). Elles ne s'exécutent **jamais** sur un élément de type « soumettre, payer, supprimer » : la liste de mots interdits est partagée avec la règle F-SEC du PRD.
