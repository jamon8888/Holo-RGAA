# rgaa-chrome — extension Chrome de remédiation RGAA

Extension Manifest V3 (panneau latéral) : passe en revue les critères RGAA un par un.
Pour chaque finding : surlignage de l'existant, capture de l'onglet, et **aperçu en direct**
de chaque proposition de remédiation de l'IA, annulable. Rien n'est jamais écrit dans le code
source du site ; l'application réelle passe par `remediate` (approbation) puis `verify`.

## Télécharger

L'extension se distribue sous forme d'archive Chrome compatible (Manifest V3) :
`rgaa-chrome-<version>.zip`, publiée à chaque [GitHub Release](
https://github.com/jamon8888/Holo-RGAA/releases) en tant qu'élément joint
(asset).

Depuis une release :
1. Télécharger `rgaa-chrome-<version>.zip` et le décompresser.
2. `chrome://extensions` → mode développeur → « Charger l'extension non empaquetée »
   → le dossier décompressé.
3. Ouvrir la page auditée, cliquer sur l'icône : le panneau s'ouvre
   (`activeTab` accordé à cet onglet).
4. Charger `.rgaa/visual/input.json`, produit par `/remediate-visual`
   (contrat : en-tête de `lib.js`).

Depuis les sources (développement) :
```
cd rgaa-rs/plugins/rgaa-chrome
npm run dist   # produit dist/rgaa-chrome-<version>.zip
```
puis charger le contenu du dossier (ou le zip décompressé) via « Charger
l'extension non empaquetée ».

## Tester

```
npm ci && npx playwright install chromium && npm test
```

Contre le vrai serveur (binaire `rgaa-mcp-http` + `obscura` 0.2.4 sur le `PATH` ; le serveur refuse
les IP privées, la page auditée doit être publique) :

```
RGAA_MCP_BIN=/chemin/rgaa-mcp-http RGAA_AUDIT_URL=https://… node tests/panel.pw.mjs
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
