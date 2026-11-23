# rgaa-chrome — extension Chrome de remédiation RGAA

Extension Manifest V3 (panneau latéral) : passe en revue les critères RGAA un par un.
Pour chaque finding : surlignage de l'existant, capture de l'onglet, et **aperçu en direct**
de chaque proposition de remédiation de l'IA, annulable. Rien n'est jamais écrit dans le code
source du site ; l'application réelle passe par `remediate` (approbation) puis `verify`.

## Utiliser

1. `chrome://extensions` → mode développeur → « Charger l'extension non empaquetée » → ce dossier.
2. Ouvrir la page auditée, cliquer sur l'icône : le panneau s'ouvre (`activeTab` accordé à cet onglet).
3. Charger `.rgaa/visual/input.json`, produit par `/remediate-visual` (contrat : en-tête de `lib.js`).

## Tester

```
npm ci && npx playwright install chromium && npm test
```

## Serveur d'audit

Le panneau (« Serveur d'audit ») appelle l'outil `analyze` de `rgaa-mcp-http` pour l'onglet courant.
Le serveur n'accepte que les origines listées : lancer avec
`RGAA_CORS_ORIGINS=chrome-extension://<id de l'extension>` et `RGAA_MCP_TOKEN=…` (jeton saisi dans
le panneau, gardé en `storage.session`, jamais persisté). Les échecs deviennent des findings
`automatique`, les « à relire » des findings `assiste`, avec le texte `remediation` du serveur en
« piste ». Le serveur ne produit pas encore de propositions structurées : pour les prévisualiser,
charger le `input.json` de `/remediate-visual`.

## Pas encore fait (décisions en attente sur la carte #490)

- Propositions structurées (attributs/CSS/HTML) générées côté serveur : ticket « API serveur ».
- Capture pleine page (aujourd'hui : zone visible, `captureVisibleTab`).
- Distribution (Chrome Web Store) et confidentialité : ticket dédié.
- Claude for Chrome : la recherche conclut à **contourner** (aucune API d'intégration tierce
  documentée) ; l'IA passera par l'API côté serveur.
