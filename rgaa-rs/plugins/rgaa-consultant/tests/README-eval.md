# Évaluation des skills d'analyse (spec P-09 §4.1)

Le dépôt n'a **pas de moteur d'évaluation de prompts** (pas de runner, pas de
harness de fixtures agentiques). Deux niveaux de test sont donc fournis.

## 1. Contrôle déterministe — sans modèle

```bash
bash rgaa-rs/plugins/rgaa-consultant/tests/skills-eval.sh
```

Ce script vérifie, pour les huit skills (`contraste`, `daltonisme`, `images`,
`medias`, `mouvement`, `langue`, `structure`, `pertinence`) :

1. l'appel à `run_analysis_plugin` avec le bon identifiant de plugin ;
2. le rappel explicite de R7 (« n'écrit jamais de statut ») et de D2 (décision
   renvoyée au relecteur) — aucun statut n'est écrit par la skill ;
3. la commande `rgaa models pull <plugin>` pour les plugins à modèles
   (`images`, `media`, `text`) ;
4. le prérequis `list_analysis_plugins`, l'avertissement photosensible de
   `mouvement`, l'étiquette « indicatif » de l'avis du modèle, et la conformité
   de la fixture au schéma `Preuve` (P-00 §5).

Il s'exécute en moins d'une seconde, sans réseau et sans LLM. C'est ce contrôle
qui est branché dans `tests/plugin-contract.sh` en CI.

## 2. Évaluation agentique — avec un agent et la fixture locale

La fixture `tests/fixtures/preuves-analyse.json` contient deux preuves
d'exemple conformes au schéma `Preuve` : une preuve `contrast` `Mesuree`
(ratio 4,48 sous le seuil 4,5, oracle WebAIM de la spec P-01 §6) et une preuve
`images` `Estimee` (logo répété sur 41 pages, similarité 0,19 sous l'ancre de
hasard 0,22, spec P-03 §6). Les valeurs sont des oracle externes codés en dur,
jamais recalculés par le code.

Pour évaluer réellement une skill avec un agent :

1. Démarrer `rgaa-mcp` (stdio ou HTTP) avec au moins le plugin ciblé compilé,
   et servir la fixture comme réponse de `run_analysis_plugin` (un simple MCP
   de test qui renvoie `{"preuves": [ ...fixture... ]}` suffit).
2. Ouvrir une session Claude Code avec le plugin installé :
   `claude plugin marketplace add . && claude plugin install rgaa-accessibility@holo-rgaa`.
3. Dans cette session, lancer la commande de la skill, par exemple
   `/rgaa:contraste <url-de-test>`, puis trois vérifications :

   | Point contrôlé | Ce qui doit être vrai |
   |----------------|-----------------------|
   | Appel outil | `list_analysis_plugins` puis `run_analysis_plugin { plugin: "contrast" }` apparaissent dans la trace |
   | R7 | Aucun appel d'outil n'écrit de statut ; la skill annonce explicitement que la décision revient au relecteur (D2) |
   | Modèle manquant | Avec une réponse `list_analysis_plugins` où `present: false`, la skill affiche textuellement `rgaa models pull images` (respectivement `media` / `text`) et s'arrête |

4. Répéter pour `images` (fixture fournie) et pour `mouvement` (le test doit
   montrer l'avertissement photosensible avant toute replay).

La même procédure vaut pour `rgaa-codex`, en remplaçant Claude for Chrome par
Codex for Chrome et `claude plugin install` par l'installation depuis
`.agents/plugins/marketplace.json`.

## Critères d'acceptation couverts

- **§4.1.1** — les trois points sont vérifiés par le contrôle déterministe et
  rejoués sur la fixture par la procédure agentique.
- **§4.1.2 / §4.1.3** — la page de revue (`docs/wayfinder/prototype/page-revue.PROTOTYPE.html`)
  est un prototype statique ; ses composants `PreuveContraste` et `PreuveImage`
  sont alimentés par les mêmes preuves d'exemple et ne sont pas pilotés par un
  contrôle CI dans ce ticket. Voir le corps de la PR pour ce qui reste ouvert.
