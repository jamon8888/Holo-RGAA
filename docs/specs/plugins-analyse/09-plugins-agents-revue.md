# P-09 — Plugins agents et page de revue : présenter les preuves

**Critères** : tous (présentation) · **Dépend de** : P-00, et au moins un plugin d'analyse livré · **Forme** : skills Claude Code / Codex (`rgaa-rs/plugins/rgaa-consultant`, `rgaa-rs/plugins/rgaa-codex`), page de revue (prototype `docs/wayfinder/prototype/page-revue.PROTOTYPE.html`, branche `prototype/page-revue`)

## 1. Objectif

Le relecteur, guidé par Claude Code / Codex (avec Claude for Chrome ou computer use pour montrer la page), voit pour chaque test `review` **la preuve calculée par le plugin d'analyse**, pas un avis brut de modèle. Il prend sa *Décision de revue* sur son compte personnel (R5).

## 2. Plugins agents (skills)

Une skill par plugin d'analyse, toutes bâties sur le même gabarit :

| Skill | Commande | Ce qu'elle fait |
|-------|----------|-----------------|
| `contraste` | `/rgaa:contraste <url>` | lance `run_analysis_plugin { plugin: "contrast" }` ; trie les preuves par ratio croissant ; demande à Claude for Chrome de **surligner** chaque élément dans l'onglet du client, puis affiche ratio, seuil et vignette |
| `daltonisme` | `/rgaa:daltonisme <url>` | lance `colorvision` ; affiche côte à côte l'original et la simulation pour chaque sujet « indiscernable » ; propose « voir la page entière comme un deutéranope » (filtre CSS injecté par Claude for Chrome, avec les matrices de `types/colorvision-matrices.json`) |
| `images` | `/rgaa:images <url>` | lance `images` ; regroupe par pHash (« ce logo, 41 pages ») ; montre la vignette, l'alt, le texte OCR et l'avis du modèle ; une décision peut être appliquée en lot **après confirmation** du relecteur |
| `medias` | `/rgaa:medias <url>` | lance `media` ; pour chaque passage manquant, ouvre le lecteur à l'horodatage (`#t=120` ou `currentTime` par Claude for Chrome) et affiche l'extrait Whisper face à l'extrait fourni |
| `mouvement` | `/rgaa:mouvement <url>` | lance `motion` ; affiche la courbe L(t) et la bande de vignettes ; **ne rejoue jamais** l'animation incriminée à l'écran sans avertissement explicite (risque photosensible pour le relecteur) |
| `langue`, `structure`, `pertinence` | `/rgaa:langue`, `/rgaa:structure`, `/rgaa:pertinence` | même gabarit |

Règles du gabarit :

- La skill n'écrit **jamais** de statut (R7). Elle prépare la décision, et c'est le relecteur qui la prend, par la page de revue ou par la commande `decide` (confirmation hors bande, décision D2).
- La skill vérifie d'abord `list_analysis_plugins`. Si le plugin n'est pas compilé, elle l'explique. Si son modèle est absent, elle donne la commande exacte : `rgaa models pull images`.
- Dans la version **Codex** (`rgaa-codex`), on retrouve les mêmes commandes, avec Codex for Chrome à la place de Claude for Chrome.
- Dans la version **GitHub** (F-GH), les skills ne tournent pas. L'action CI exécute les plugins **sans LLM**, par défaut, et publie les preuves `Mesuree` en annotations de PR. Les preuves `Estimee` vont en résumé, avec un lien vers la page de revue.

## 3. Page de revue : un composant par type de preuve

| Plugin | Composant | Contenu |
|--------|-----------|---------|
| `contrast` | `PreuveContraste` | pastilles des deux couleurs, ratio en gros, seuil, vignette avec masque texte, état (repos, survol, focus) |
| `colorvision` | `PreuveCouleur` | curseur avant/après entre l'original et la simulation ; sélecteur de type de daltonisme |
| `images` | `PreuveImage` | vignette, alt, texte OCR surligné, score de similarité face à l'ancre de hasard, nombre d'occurrences |
| `media` | `PreuveMedia` | lecteur natif sur le média du client, avec la transcription Whisper défilante et les passages manquants en rouge (cliquables) |
| `motion` | `PreuveMouvement` | courbe SVG, vignettes **statiques**, avertissement photosensible |
| `lang`, `layout`, `text` | `PreuveListe` | tableau des signaux et extrait |

La page de revue :

- garde la règle R3 : aucun « verdict IA » n'est affiché ; l'avis du modèle est replié, sous le libellé « avis du modèle (indicatif) » ;
- respecte elle-même le RGAA : contrastes, alternatives des vignettes générées, et pas de lecture automatique des médias ;
- est contrôlée par `/rgaa:contraste` et `/rgaa:images` dans la CI du dépôt.

## 4. Critères d'acceptation

1. Chaque skill a un test d'évaluation (`rgaa-consultant/tests`) qui vérifie trois points sur une fixture locale :
   - elle appelle `run_analysis_plugin` ;
   - elle n'écrit aucun statut ;
   - elle affiche la commande `models pull` quand le modèle manque.
2. Le prototype de page de revue gagne `PreuveContraste` et `PreuveImage`, alimentés par des preuves réelles du corpus. Ils sont validés par le porteur, puis portés dans la page de revue de l'instance (F-VIS-8).
3. La page de revue passe `/rgaa:contraste` et `/rgaa:images` sans `Suspect`.
