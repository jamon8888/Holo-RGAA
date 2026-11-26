---
name: visual-remediation
description: Produce screenshots of the audited site highlighting, for each RGAA criterion, the non-compliant existing elements, then render several AI remediation proposals per finding live in Playwright/Chromium and show them side by side. Use when the user wants visual evidence of RGAA failures, before/after previews, or to compare remediation options visually.
---

# visual-remediation — Preuves visuelles et propositions de remédiation

## Objectif

Pour un site audité :

1. **Existant par critère** — une capture pleine page par critère RGAA, avec tous
   les éléments en échec encadrés en rouge et étiquetés `RGAA x.y`.
2. **Existant par finding** — un zoom sur l'élément en échec.
3. **Plusieurs propositions IA** — pour chaque finding, 2 à 3 remédiations
   différentes, appliquées en direct dans Chromium (Playwright) et capturées,
   encadrées en vert.
4. **Galerie** — `index.html` accessible, existant et propositions côte à côte.

Rien n'est modifié dans le code source : les propositions sont injectées dans
la page chargée. Le passage au code reste soumis à la skill `remediate` et à
son approbation.

## Workflow

1. **Récupérer les findings** — depuis un audit existant (`get_audit_result`,
   `.rgaa/` ou la sortie de la skill `audit`), sinon lancer `analyze` sur l'URL.
   Garder les findings en échec ayant un `target` (sélecteur CSS) et un
   `criterion_id`. Au-delà de 25 findings, demander lesquels prioriser.
2. **Générer plusieurs propositions par finding** — rédiger 2 à 3 variantes
   réellement différentes, chacune avec un `label` et une `rationale` citant le
   critère. Angles à couvrir quand ils ont du sens :
   - **Correctif minimal** — attribut ou ARIA (`attributes`).
   - **Correctif sémantique** — remplacement du balisage (`html`).
   - **Correctif visuel** — contraste, focus, taille de cible (`css`).
   Si le serveur `rgaa-mcp` est disponible, ajouter la proposition de l'outil
   `remediate` comme variante supplémentaire (label « Proposition rgaa-mcp »).
   Ne jamais inventer un contenu métier (texte d'alternative, intitulé de lien)
   sans le signaler comme à valider dans la `rationale`.
3. **Écrire l'entrée** — `.rgaa/visual/input.json` :

   Format complet et règles : `CONTRACT.md` (même dossier). Version 1.

   ```json
   {
     "contract_version": 1,
     "url": "https://site.audite/page",
     "findings": [{
       "id": "fng_abc123", "criterion_id": "1.1", "target": "img.hero",
       "description": "Image porteuse d'information sans alternative",
       "proposals": [
         { "label": "Alternative descriptive", "rationale": "1.1 — texte à valider par l'éditeur",
           "needs_human_review": true,
           "attributes": { "alt": "Équipe en réunion" } },
         { "label": "Image décorative", "rationale": "1.2 — si l'image n'apporte pas d'information",
           "attributes": { "alt": "", "role": null } }
       ]
     }]
   }
   ```

   Champ optionnel `tier` d'un finding (`automatique` | `assiste` | `irreductible`,
   les niveaux d'automatisation de `CONTEXT.md`) : l'extension Chrome l'affiche pour
   rappeler qu'un test Assisté reste `review` et un test Irréductible un contrôle humain.

   `needs_human_review: true` est obligatoire sur toute proposition qui invente un contenu métier.

   Champs d'une proposition (combinables) : `attributes` (valeur `null` =
   suppression), `css` (feuille injectée), `html` (remplace l'élément cible).
4. **Capturer** :

   ```bash
   cd "${CLAUDE_PLUGIN_ROOT}/scripts/visual"
   [ -d node_modules ] || npm ci
   npx playwright install chromium   # une fois par machine
   node capture.mjs "${CLAUDE_PROJECT_DIR}/.rgaa/visual/input.json" "${CLAUDE_PROJECT_DIR}/.rgaa/visual"
   ```

   Une URL derrière authentification n'est pas prise en charge : le dire et
   proposer une URL publique ou un serveur local.
5. **Présenter** — lire `manifest.json`, regarder les captures (outil de
   lecture d'image) pour vérifier que chaque proposition rend bien ce qui est
   annoncé, puis donner le chemin de `index.html`. Signaler les findings en
   `error` (cible introuvable, proposition non applicable).
6. **Choix et suite** — demander quelle proposition retenir par finding, puis
   passer la proposition choisie à la skill `remediate` (patch source avec
   approbation) et à `verify` après application.

## Extension Chrome

`input.json` est le contrat partagé avec l'extension `rgaa-chrome` (`rgaa-rs/plugins/rgaa-chrome`) :
l'utilisateur l'ouvre dans le panneau latéral et parcourt les critères un par un, avec aperçu
en direct des propositions dans son propre onglet (annulable, jamais d'écriture dans la source).
Le Playwright de cette skill reste le producteur de captures pour la CI et les lots.

## Sorties

| Fichier | Contenu |
|---------|---------|
| `criteres/<critère>.png` | Page entière, cibles du critère encadrées en rouge |
| `findings/<id>/existant.png` | Zoom sur l'élément en échec |
| `findings/<id>/proposition-<n>.png` | Rendu de la proposition *n*, encadré en vert |
| `manifest.json` | Index machine des captures et des erreurs |
| `index.html` | Galerie comparative accessible |

## Limites

- Un rendu visuel ne prouve pas la conformité : un `alt` ou un rôle ARIA ne se
  voit pas. Après application, la conformité se vérifie avec `verify`.
- Les sélecteurs d'iframe ou de Shadow DOM (tableaux axe) ne sont pas
  traversés : seul le dernier sélecteur est utilisé.
- Les critères non visuels (ex. 8.x métadonnées) n'ont pas de cible à encadrer ;
  les traiter dans le rapport plutôt qu'ici.
- Le HTML des propositions est nettoyé de ses contenus actifs (scripts, gestionnaires
  d'événements et URL `javascript:`) avant insertion dans la page auditée.

## Skills liées

- `audit` — source des findings
- `remediate` — patch source de la proposition retenue
- `verify` — re-test après application
- `report` — joindre les captures au rapport
