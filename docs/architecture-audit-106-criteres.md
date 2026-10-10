# Audit RGAA 4.1.2 — architecture, couverture des 106 critères, reste à faire

> État du code : `master` après le merge de #280 (répartition par moteur, correction des faux `Pass` axe), 2026-10-05.
> Document destiné aux humains **et aux agents** : [AGENTS.md](../AGENTS.md) y renvoie, et ce document
> renvoie aux fichiers de code et aux PR/issues qui font foi. En cas de désaccord, **le code gagne** ;
> corrigez ce document.

## 1. Périmètre

Le référentiel est le RGAA 4.1.2 de la DINUM (PDF fourni, identique au catalogue
`rgaa-rs/crates/rgaa-core/data/rgaa-4.1.2/criteres.json`) : **13 thèmes, 106 critères, 258 tests**.
Aucun chiffre « 442 » n'apparaît dans le référentiel ni dans le dépôt. L'objectif est que **chacun des 106
critères** ait un traitement explicite : un moteur qui tranche, ou un renvoi assumé à l'humain.

## 2. Qui travaille avec qui

```
URL ──► rgaa-orchestrator (pipeline.rs)
          │
          ├─ 1. axe-core 4.9.1 ────────┐   injecté par rgaa-obscura (CDP)         → rgaa-rules::AxeMapper
          ├─ 2. sondes JS (« gap-fix »)┤   exécutées par rgaa-obscura (scrape)    → rgaa-rules::GapFixRules
          ├─ 3. contexte de page ──────┤   texte, AX tree, captures               → rgaa-obscura / rgaa-browser-tools
          ├─ 4. agent Holo (IA) ───────┘   rgaa-agent (+ rgaa-holo : client LLM)
          └─ 5. fusion  rgaa-orchestrator/src/merge.rs  ──► AuditResult ──► rgaa-report / rgaa-api / rgaa-mcp / rgaa-cli / rgaa-tui
```

| Brique | Rôle | Fichiers qui font foi |
|---|---|---|
| **rgaa-core** | Types, catalogue des 106 critères, classification, registre des mécanismes | `src/criteria.rs`, `src/registry.rs`, `data/rgaa-4.1.2/{criteres,mechanisms,axe_rules,automatable_criteres}.json|toml` |
| **rgaa-obscura** | Pont vers le navigateur **Obscura 0.2.2** (binaire épinglé, CDP) : navigation, évaluation JS, axe, captures, AX tree, clavier (`press_key`, `get_tab_order`) | `src/lib.rs` (`ObscuraBridge`), `src/cdp_pool.rs`, [docs/obscura-substrate.md](obscura-substrate.md) |
| **axe-core** | Moteur de règles d'accessibilité, 4.9.1, exécuté **dans la page** via Obscura. Ses règles étiquetées `RGAAv4` sont mappées aux critères | `rgaa-obscura/src/lib.rs` (`run_axe_batch`), `rgaa-rules/src/axe_mapper.rs` |
| **rgaa-rules** | Traduit les violations axe et les résultats des sondes JS en `CriterionResult` | `axe_mapper.rs`, `gap_fix.rs` |
| **rgaa-agent / rgaa-holo** | Évaluation par LLM (Holo) des critères qui demandent du jugement, avec RAG | `rgaa-agent/src/{agent,criteria_defs,rag}`, `rgaa-holo/src` |
| **rgaa-orchestrator** | Enchaîne les phases, fusionne les verdicts, calcule le taux | `src/pipeline.rs`, `src/merge.rs` |
| **rgaa-test-corpus** | Pages de test `<critère>-<nom>-pass|fail.html` et invariants du registre en CI | `criteria/`, `tests/registry_invariants.rs` |
| **rgaa-linter** | Analyse statique des sources (sans navigateur) | `src/rules.rs` |
| **rgaa-report / -api / -mcp / -cli / -tui** | Rapports, REST, outils MCP, CLI, TUI | [docs/README](../rgaa-rs/docs/README.md) |

### 2.1 Obscura

Obscura est le **substrat navigateur** : un binaire épinglé (`install.sh`, CI et `binary_version()` vérifient la
version), piloté en CDP par `ObscuraBridge`. C'est lui qui charge la page, exécute le JS (axe, sondes), rend
l'arbre d'accessibilité et prend les captures. `scrape --concurrency` borne le parallélisme (max 32).
Réglages d'exploitation, délais et rollback : [obscura-substrate.md](obscura-substrate.md).
Règle du registre : un mécanisme déclare `engine = "obscura"` ; **`chrome` n'est admis qu'avec la preuve qu'une
fixture échoue sous Obscura et passe sous Chrome** (`registry.rs`). Toute mesure qui dépend d'une vraie mise en
page (reflow 320 px, zoom 200 %, positions) est donc à prouver sous Obscura avant d'ouvrir la porte à Chrome
(#194, mode C).

### 2.2 axe-core

- Source : `axe.min.js` 4.9.1, **téléchargé depuis cdnjs à l'exécution** (`fetch_axe_source`, mis en cache dans le
  processus). Un audit hors-ligne échoue donc ; l'embarquer est un point ouvert (voir §7).
- 105 règles dans `axe_rules.json`, dont celles étiquetées `RGAAv4` (`RGAA-x.y.z`) : c'est la source d'attribution
  des règles aux critères.
- **axe ne couvre qu'une partie des tests** d'un critère. D'où la notion de *couverture* par mécanisme :
  `complete` (le silence d'axe vaut Pass) ou `partial` (axe peut faire échouer, jamais valider).

### 2.3 Sondes JS (« gap-fix ») et registre

Ce que axe ne sait pas faire est écrit en JS, exécuté dans la page, et déclaré dans
`mechanisms.toml` (un `[[mechanism]]` par mécanisme : `kind` = `axe-native | js-static | js-behavioural |
media-analysis | site`, `coverage`, `outcomes`, `fixtures`). Invariants vérifiés en CI : « pass » exige
`coverage = complete` ; tout mécanisme non hérité a ses fixtures dans `rgaa-test-corpus/criteria/` ; toute règle
axe citée existe. Une sonde rend `pass | fail | review` (contrat à trois issues, #277).

### 2.4 Holo (agent IA)

Les critères qui demandent du jugement (pertinence d'un `alt`, d'un titre, d'une étiquette ; vision) passent par
l'agent : 32 `IaAssiste` + les `PartiallyAutomatable` du catalogue (63 critères au total, `criteria.rs` et
`automatable_criteres.json`). Les phases sont `AgentIaAssiste` puis `AgentPartial`.

### 2.5 Fusion des verdicts (`merge.rs`, #206)

1. Une évaluation en erreur n'est pas une preuve (tout en erreur ⇒ `NotTested`, jamais `NeedsReview`).
2. **Une preuve déterministe (axe, sonde, manuel) l'emporte sur un verdict LLM.**
3. À rang égal, le plus prudent gagne : `Fail` > `NeedsReview` > `Pass` > `NotApplicable`.
4. Égalité : le premier candidat ; les sources considérées sont tracées dans `considered_sources`.

Conséquence : **un `Pass` axe « complet » mais faux ne peut pas être rattrapé par Holo.** La fiabilité du mapping
est donc critique (§6).

## 3. Comment traiter un critère

Pour chaque critère, dans cet ordre :

1. **Peut-on décider sans jugement ?** Oui ⇒ moteur *axe* si une règle axe vérifie réellement ses tests, sinon
   *sonde JS* (ou comportement : clavier, site) — déclarée dans `mechanisms.toml`.
2. **Couverture** : tous les tests du critère ? `complete` seulement alors. Sinon `partial` : échec avéré
   possible, jamais de Pass.
3. **Reste un jugement** (pertinence, sens, vision) ⇒ Holo, sur les seuls candidats que la détection
   déterministe laisse debout (pattern de la spec #202).
4. **Aucun contrôle fiable** (écoute d'un média, lecteur d'écran réel, équivalence d'information) ⇒ `Manuel` :
   `needs_review`, avec pré-tri automatique si possible.
5. **Un outil absent est `NotTested`, jamais `Pass`.**
6. Toute nouvelle sonde livre ses fixtures `<critère>-<nom>-pass.html` et `-fail.html` et passe les invariants.

Détail des répartitions : [docs/research/couverture-repartition-106.md](research/couverture-repartition-106.md)
(répartition portée dans `rgaa-core/data/rgaa-4.1.2/engine_plan.json`, API `EnginePlan`, PR [#280](https://github.com/jamon8888/Holo-RGAA/pull/280)).

## 4. Les 106 critères

Colonne **Moteur prévu** : moteur qui devrait trancher (`engine_plan.json`, #280 ; proposition, le pipeline ne l'utilise pas encore pour router). axe-core n'est moteur principal que si sa couverture du critère est **complète** (invariant testé dans #280) ; en couverture partielle, c'est le moteur déterministe ou Holo qui tranche. **Mécanismes** : ce qui existe dans
`mechanisms.toml`. **Statut** : *Couvert* = le moteur prévu est implémenté ; *Non testé* = il ne l'est pas
(seul un éventuel repli Holo produit un verdict, sinon `needs_review`) ; *Manuel* = humain par conception.
Un statut « Couvert » ne veut pas dire « testé critère par critère » : voir les fixtures du corpus.

| Critère | Tests | Intitulé | Moteur prévu | Mécanismes | Holo | Statut | Issue |
|---|---|---|---|---|---|---|---|
| 1.1 | 8 | Chaque image porteuse d’information a-t-elle une alternative textuelle | axe-core | axe:complet, sonde JS:complet | oui | **Couvert** |  |
| 1.2 | 6 | Chaque image de décoration est-elle correctement ignorée par les techn | Holo | axe:partiel, sonde JS:complet | oui | **Couvert** |  |
| 1.3 | 9 | Pour chaque image porteuse d’information ayant une alternative textuel | Holo | — | oui | **Couvert** |  |
| 1.4 | 7 | Pour chaque image utilisée comme CAPTCHA ou comme image-test, ayant un | Holo | — | oui | **Couvert** |  |
| 1.5 | 2 | Pour chaque image utilisée comme CAPTCHA, une solution d’accès alterna | Holo | — | oui | **Couvert** |  |
| 1.6 | 10 | Chaque image porteuse d’information a-t-elle, si nécessaire, une descr | Déterministe | — | oui | **Sonde partielle** |  |
| 1.7 | 6 | Pour chaque image porteuse d’information ayant une description détaill | Holo | — | oui | **Couvert** |  |
| 1.8 | 6 | Chaque image texte porteuse d’information, en l’absence d’un mécanisme | Holo | — | oui | **Couvert** |  |
| 1.9 | 5 | Chaque légende d’image est-elle, si nécessaire, correctement reliée à  | Déterministe | sonde JS:partiel | oui | **Couvert** |  |
| 2.1 | 1 | Chaque cadre a-t-il un titre de cadre ? | axe-core | axe:complet, sonde JS:complet | — | **Couvert** |  |
| 2.2 | 1 | Pour chaque cadre ayant un titre de cadre, ce titre de cadre est-il pe | Holo | — | oui | **Couvert** |  |
| 3.1 | 6 | Dans chaque page web, l’information ne doit pas être donnée uniquement | Holo | — | oui | **Couvert** |  |
| 3.2 | 5 | Dans chaque page web, le contraste entre la couleur du texte et la cou | axe-core | axe:complet, sonde JS:complet | — | **Couvert** |  |
| 3.3 | 4 | Dans chaque page web, les couleurs utilisées dans les composants d’int | Déterministe | axe:partiel | — | **Sonde partielle** |  |
| 4.1 | 3 | Chaque média temporel pré-enregistré a-t-il, si nécessaire, une transc | Déterministe | sonde JS:partiel | oui | **Couvert** |  |
| 4.2 | 3 | Pour chaque média temporel pré-enregistré ayant une transcription text | Humain | — | oui | **Manuel** |  |
| 4.3 | 2 | Chaque média temporel synchronisé pré-enregistré a-t-il, si nécessaire | axe-core | axe:complet | oui | **Couvert** |  |
| 4.4 | 1 | Pour chaque média temporel synchronisé pré-enregistré ayant des sous-t | Humain | — | oui | **Manuel** |  |
| 4.5 | 2 | Chaque média temporel pré-enregistré a-t-il, si nécessaire, une audiod | Déterministe | sonde JS:partiel | — | **Couvert** |  |
| 4.6 | 2 | Pour chaque média temporel pré-enregistré ayant une audiodescription s | Humain | — | oui | **Manuel** |  |
| 4.7 | 1 | Chaque média temporel est-il clairement identifiable (hors cas particu | Déterministe | sonde JS:partiel | oui | **Couvert** |  |
| 4.8 | 2 | Chaque média non temporel a-t-il, si nécessaire, une alternative (hors | Déterministe | sonde JS:partiel | oui | **Couvert** |  |
| 4.9 | 1 | Pour chaque média non temporel ayant une alternative, cette alternativ | Holo | — | oui | **Couvert** |  |
| 4.10 | 1 | Chaque son déclenché automatiquement est-il contrôlable par l’utilisat | Déterministe | axe:partiel | — | **Couvert** |  |
| 4.11 | 3 | La consultation de chaque média temporel est-elle, si nécessaire, cont | Déterministe | sonde JS:partiel | — | **Couvert** |  |
| 4.12 | 2 | La consultation de chaque média non temporel est-elle contrôlable par  | Déterministe | — | — | **Sonde partielle** |  |
| 4.13 | 2 | Chaque média temporel et non temporel est-il compatible avec les techn | Déterministe | — | — | **Sonde partielle** |  |
| 5.1 | 1 | Chaque tableau de données complexe a-t-il un résumé ? | Déterministe | sonde JS:partiel | — | **Couvert** |  |
| 5.2 | 1 | Pour chaque tableau de données complexe ayant un résumé, celui-ci est- | Holo | — | oui | **Couvert** |  |
| 5.3 | 1 | Pour chaque tableau de mise en forme, le contenu linéarisé reste-t-il  | Holo | — | oui | **Couvert** |  |
| 5.4 | 1 | Pour chaque tableau de données ayant un titre, le titre est-il correct | Déterministe | axe:partiel | — | **Couvert** |  |
| 5.5 | 1 | Pour chaque tableau de données ayant un titre, celui-ci est-il pertine | Holo | — | oui | **Couvert** |  |
| 5.6 | 4 | Pour chaque tableau de données, chaque en-tête de colonne et chaque en | Holo | axe:partiel | oui | **Couvert** |  |
| 5.7 | 5 | Pour chaque tableau de données, la technique appropriée permettant d’a | axe-core | axe:complet | oui | **Couvert** |  |
| 5.8 | 1 | Chaque tableau de mise en forme ne doit pas utiliser d’éléments propre | Déterministe | sonde JS:partiel | oui | **Couvert** |  |
| 6.1 | 5 | Chaque lien est-il explicite (hors cas particuliers) ? | Holo | axe:partiel, sonde JS:complet | oui | **Couvert** |  |
| 6.2 | 1 | Dans chaque page web, chaque lien a-t-il un intitulé ? | axe-core | axe:complet | — | **Couvert** |  |
| 7.1 | 3 | Chaque script est-il, si nécessaire, compatible avec les technologies  | Holo | axe:partiel | oui | **Couvert** |  |
| 7.2 | 2 | Pour chaque script ayant une alternative, cette alternative est-elle p | Holo | — | oui | **Couvert** |  |
| 7.3 | 2 | Chaque script est-il contrôlable par le clavier et par tout dispositif | Déterministe | axe:partiel | — | **Couvert** |  |
| 7.4 | 1 | Pour chaque script qui initie un changement de contexte, l’utilisateur | Déterministe | sonde JS:partiel | oui | **Couvert** |  |
| 7.5 | 3 | Dans chaque page web, les messages de statut sont-ils correctement res | Humain | — | — | **Manuel** |  |
| 8.1 | 3 | Chaque page web est-elle définie par un type de document ? | Déterministe | sonde JS:partiel | — | **Couvert** |  |
| 8.2 | 1 | Pour chaque page web, le code source généré est-il valide selon le typ | Déterministe | axe:partiel | oui | **Couvert** |  |
| 8.3 | 1 | Dans chaque page web, la langue par défaut est-elle présente ? | axe-core | axe:complet, sonde JS:complet | oui | **Couvert** |  |
| 8.4 | 1 | Pour chaque page web ayant une langue par défaut, le code de langue es | Holo | axe:partiel | oui | **Couvert** |  |
| 8.5 | 1 | Chaque page web a-t-elle un titre de page ? | axe-core | sonde JS:complet, axe:complet | — | **Couvert** |  |
| 8.6 | 1 | Pour chaque page web ayant un titre de page, ce titre est-il pertinent | Holo | — | oui | **Couvert** |  |
| 8.7 | 1 | Dans chaque page web, chaque changement de langue est-il indiqué dans  | Déterministe | — | oui | **Sonde partielle** |  |
| 8.8 | 1 | Dans chaque page web, le code de langue de chaque changement de langue | Holo | axe:partiel | oui | **Couvert** |  |
| 8.9 | 1 | Dans chaque page web, les balises ne doivent pas être utilisées unique | Déterministe | sonde JS:partiel | — | **Couvert** |  |
| 8.10 | 2 | Dans chaque page web, les changements du sens de lecture sont-ils sign | Déterministe | sonde JS:partiel | oui | **Couvert** |  |
| 9.1 | 3 | Dans chaque page web, l’information est-elle structurée par l’utilisat | Holo | axe:partiel | oui | **Couvert** |  |
| 9.2 | 1 | Dans chaque page web, la structure du document est-elle cohérente (hor | Holo | — | oui | **Couvert** |  |
| 9.3 | 3 | Dans chaque page web, chaque liste est-elle correctement structurée ? | axe-core | axe:complet | oui | **Couvert** |  |
| 9.4 | 2 | Dans chaque page web, chaque citation est-elle correctement indiquée ? | Déterministe | sonde JS:partiel | — | **Couvert** |  |
| 10.1 | 3 | Dans le site web, des feuilles de styles sont-elles utilisées pour con | Déterministe | sonde JS:partiel | oui | **Couvert** |  |
| 10.2 | 1 | Dans chaque page web, le contenu visible porteur d’information reste-t | Déterministe | sonde JS:complet | — | **Couvert** |  |
| 10.3 | 1 | Dans chaque page web, l’information reste-t-elle compréhensible lorsqu | Holo | — | oui | **Couvert** |  |
| 10.4 | 2 | Dans chaque page web, le texte reste-t-il lisible lorsque la taille de | Déterministe | axe:partiel | — | **Couvert** |  |
| 10.5 | 3 | Dans chaque page web, les déclarations CSS de couleurs de fond d’éléme | Déterministe | sonde JS:partiel | — | **Couvert** |  |
| 10.6 | 1 | Dans chaque page web, chaque lien dont la nature n’est pas évidente es | axe-core | axe:complet | oui | **Couvert** |  |
| 10.7 | 1 | Dans chaque page web, pour chaque élément recevant le focus, la prise  | Déterministe | sonde JS:partiel | oui | **Couvert** |  |
| 10.8 | 1 | Pour chaque page web, les contenus cachés ont-ils vocation à être igno | axe-core | axe:complet | — | **Couvert** |  |
| 10.9 | 4 | Dans chaque page web, l’information ne doit pas être donnée uniquement | Holo | — | — | **Sonde partielle** |  |
| 10.10 | 4 | Dans chaque page web, l’information ne doit pas être donnée par la for | Holo | — | oui | **Couvert** |  |
| 10.11 | 2 | Pour chaque page web, les contenus peuvent-ils être présentés sans per | Déterministe | axe:partiel | — | **Couvert** |  |
| 10.12 | 1 | Dans chaque page web, les propriétés d’espacement du texte peuvent-ell | Déterministe | — | — | **Sonde partielle** |  |
| 10.13 | 3 | Dans chaque page web, les contenus additionnels apparaissant à la pris | Déterministe | — | oui | **Sonde partielle** |  |
| 10.14 | 2 | Dans chaque page web, les contenus additionnels apparaissant via les s | Déterministe | sonde JS:complet | — | **Couvert** |  |
| 11.1 | 3 | Chaque champ de formulaire a-t-il une étiquette ? | axe-core | axe:complet, sonde JS:complet | oui | **Couvert** |  |
| 11.2 | 6 | Chaque étiquette associée à un champ de formulaire est-elle pertinente | Holo | axe:partiel | oui | **Couvert** |  |
| 11.3 | 2 | Dans chaque formulaire, chaque étiquette associée à un champ de formul | Déterministe | — | oui | **Sonde partielle** |  |
| 11.4 | 3 | Dans chaque formulaire, chaque étiquette de champ et son champ associé | Déterministe | sonde JS:complet | oui | **Couvert** |  |
| 11.5 | 1 | Dans chaque formulaire, les champs de même nature sont-ils regroupés,  | Déterministe | sonde JS:partiel | oui | **Couvert** |  |
| 11.6 | 1 | Dans chaque formulaire, chaque regroupement de champs de même nature a | Déterministe | sonde JS:partiel | — | **Couvert** |  |
| 11.7 | 1 | Dans chaque formulaire, chaque légende associée à un regroupement de c | Holo | — | oui | **Couvert** |  |
| 11.8 | 3 | Dans chaque formulaire, les items de même nature d’une liste de choix  | Déterministe | — | oui | **Sonde partielle** |  |
| 11.9 | 2 | Dans chaque formulaire, l’intitulé de chaque bouton est-il pertinent ( | Holo | axe:partiel | oui | **Couvert** |  |
| 11.10 | 7 | Dans chaque formulaire, le contrôle de saisie est-il utilisé de manièr | Holo | — | oui | **Couvert** |  |
| 11.11 | 2 | Dans chaque formulaire, le contrôle de saisie est-il accompagné, si né | Holo | — | — | **Sonde partielle** |  |
| 11.12 | 2 | Pour chaque formulaire qui modifie ou supprime des données, ou qui tr | Humain | — | oui | **Manuel** |  |
| 11.13 | 1 | La finalité d’un champ de saisie peut-elle être déduite pour faciliter | Holo | axe:partiel | oui | **Couvert** |  |
| 12.1 | 1 | Chaque ensemble de pages dispose-t-il de deux systèmes de navigation d | Déterministe | axe:partiel | — | **Sonde partielle** | #269 |
| 12.2 | 1 | Dans chaque ensemble de pages, le menu et les barres de navigation son | Déterministe | — | — | **Sonde partielle** | #270 |
| 12.3 | 3 | La page « plan du site » est-elle pertinente ? | Holo | — | oui | **Couvert** | #271 |
| 12.4 | 3 | Dans chaque ensemble de pages, la page « plan du site » est-elle acces | Déterministe | axe:partiel | — | **Sonde partielle** | #269 |
| 12.5 | 3 | Dans chaque ensemble de pages, le moteur de recherche est-il atteignab | Déterministe | — | — | **Sonde partielle** | #270 |
| 12.6 | 1 | Les zones de regroupement de contenus présentes dans plusieurs pages w | Déterministe | axe:partiel | oui | **Couvert** |  |
| 12.7 | 2 | Dans chaque page web, un lien d’évitement ou d’accès rapide à la zone  | axe-core | axe:complet, sonde JS:complet | oui | **Couvert** |  |
| 12.8 | 2 | Dans chaque page web, l’ordre de tabulation est-il cohérent ? | Déterministe | — | oui | **Sonde partielle** | #263 #279 |
| 12.9 | 1 | Dans chaque page web, la navigation ne doit pas contenir de piège au c | Déterministe | — | — | **Sonde partielle** | #264 |
| 12.10 | 1 | Dans chaque page web, les raccourcis clavier n’utilisant qu’une seule  | Déterministe | — | — | **Sonde partielle** | #272 |
| 12.11 | 1 | Dans chaque page web, les contenus additionnels apparaissant au survol | Déterministe | — | — | **Sonde partielle** |  |
| 13.1 | 4 | Pour chaque page web, l’utilisateur a-t-il le contrôle de chaque limit | Humain | axe:partiel | — | **Manuel** | #265 |
| 13.2 | 1 | Dans chaque page web, l’ouverture d’une nouvelle fenêtre ne doit pas ê | Déterministe | sonde JS:partiel | — | **Couvert** | #266 |
| 13.3 | 1 | Dans chaque page web, chaque document bureautique en téléchargement po | Déterministe | — | — | **Sonde partielle** |  |
| 13.4 | 1 | Pour chaque document bureautique ayant une version accessible, cette v | Humain | — | oui | **Manuel** |  |
| 13.5 | 1 | Dans chaque page web, chaque contenu cryptique (art ASCII, émoticône,  | Déterministe | sonde JS:partiel | oui | **Couvert** |  |
| 13.6 | 1 | Dans chaque page web, pour chaque contenu cryptique (art ASCII, émotic | Holo | — | oui | **Couvert** |  |
| 13.7 | 3 | Dans chaque page web, les changements brusques de luminosité ou les ef | Humain | — | — | **Manuel** | #273 |
| 13.8 | 2 | Dans chaque page web, chaque contenu en mouvement ou clignotant est-il | Déterministe | axe:partiel | — | **Couvert** | #267 |
| 13.9 | 1 | Dans chaque page web, le contenu proposé est-il consultable quelle que | Déterministe | axe:partiel | — | **Couvert** | #268 |
| 13.10 | 2 | Dans chaque page web, les fonctionnalités utilisables ou disponibles a | Déterministe | — | — | **Sonde partielle** | #272 |
| 13.11 | 1 | Dans chaque page web, les actions déclenchées au moyen d’un dispositif | Déterministe | — | — | **Sonde partielle** | #272 |
| 13.12 | 3 | Dans chaque page web, les fonctionnalités qui impliquent un mouvement  | Déterministe | — | — | **Sonde partielle** | #272 |

**État après raccordement :** 75 critères disposent des mécanismes décrits dans l’audit initial ; les 23 critères ci-dessous disposent maintenant d’une sonde partielle ou d’une comparaison site raccordée. Ces 23 résultats restent `Fail` ou `NeedsReview` : aucun mécanisme incomplet ne peut les passer. Les 8 critères humains restent manuels (4.2, 4.4, 4.6, 7.5, 11.12, 13.1, 13.4, 13.7). Moteurs principaux : axe-core 13 · déterministe 53 · Holo 32 · humain 8.

## 5. Historique : ce qui a déjà été fait (PR mergées)

| PR | Apport |
|---|---|
| #204 | Fin des `Pass` déterministes qui ne peuvent pas échouer (#199) |
| #206 | Précédence de fusion explicite (`merge.rs`) |
| #207 | Récupération de 49 règles axe `RGAAv4` + 3 mécanismes déterministes (10.1, 11.5, 1.9) |
| #208 / #244 | Correctifs Obscura / axe (chemin poolé, orphelins, délais) |
| #209 | Résultats par test, inapplicabilité déterministe, taux publiable seulement avec sa couverture (#203) |
| #210 | Cartographie `RGAAv4` des règles axe |
| #257 / #278 | 3.3, 5.6, 12.1, 12.4 déclarés `partial` (#256) |
| #260 | Spec d'extension de la couverture réelle |
| #276 | **Registre unique des mécanismes** (`mechanisms.toml`) |
| #277 | Contrat de sortie à trois issues des sondes |
| #280 | `EnginePlan` (moteur qui tranche, 106 critères), 9 faux `Pass` axe retirés, 17 sondes DOM + 2 règles axe avec fixtures |

## 6. Reste à faire

### 6.1 Faux `Pass` axe — corrigé par #280

Neuf critères déclaraient `axe-native / complete` alors que la règle ne vérifie pas leurs tests (1.5, 1.6, 10.2, 10.5,
10.9, 11.4, 13.3, 13.4, 13.5). #280 a retiré ces mécanismes, rétrogradé 1.2, 6.1, 9.1 et 12.6 en `partial`, rattaché
`document-title` à 8.5 et ajouté `button-name` pour 11.9. Reste la règle de fond : **un mécanisme `partial` ne peut
jamais émettre `Pass`** (invariant du registre). Les critères 10.2 et 11.4 gardent leur sonde JS.

### 6.2 Les 23 critères désormais sondés partiellement

1.6, 3.3, 4.12, 4.13, 8.7, 10.9, 10.12, 10.13, 11.3, 11.8, 11.11, 12.1, 12.2, 12.4, 12.5, 12.8, 12.9, 12.10, 12.11, 13.3, 13.10, 13.11, 13.12.

Les résultats uniformes ou sans défaut détecté restent à revoir : le rendu exact, la pertinence sémantique, les exceptions RGAA, la compatibilité avec les technologies d’assistance et l’exhaustivité d’un crawl ne sont pas établis par ces signaux.

| Contrôle raccordé | Critères | Limite du contrôle |
|---|---|---|
| Sondes DOM, média et formulaire | 1.6, 3.3, 4.13, 8.7, 10.9, 11.3, 11.8, 11.11, 13.3 | indices candidats seulement ; analyse visuelle et sémantique humaine/Holo encore requise |
| Interactions bornées Obscura | 4.12, 10.12, 10.13, 12.8, 12.9, 12.10, 12.11, 13.10, 13.11, 13.12 | Tab limité pour 12.8/12.9 ; autres gestes, annulation, persistances et équivalences restent en `NeedsReview` |
| Comparaison du même échantillon multi-pages | 12.1, 12.2, 12.4, 12.5 | les signaux contradictoires peuvent échouer ; un échantillon incomplet ou cohérent ne peut pas établir un `Pass` |
| Métadonnées de médias | 4.12, 4.13 | le test réel avec technologie d’assistance reste manuel |

### 6.3 Chantiers transverses

- **Embarquer axe-core** au lieu du CDN (audit hors-ligne, reproductibilité) — à ouvrir si absent de #244.
- **Verdict plafonné par mécanisme** (#279) : un mécanisme « suspect » doit rendre `review`, pas `fail` (12.8).
- **Générateur** `rgaa-data` : il régénère un mapping obsolète sans `coverage` (#274) ; à aligner sur le registre.
- **Granularité par test** (#203, partiellement faite par #209) : remplace le drapeau complet/partiel par critère.
- **Routage par moteur** : n'appeler Holo que sur ses critères (coût, #180) ; change les verdicts, passe par #180/#183.
- **Évaluation** : corpus de référence et métriques (#189, #192, #211–#215).
- **Valeur légale** : visa humain à l'export (#191), barrière de couverture (#190).

### 6.4 Ordre recommandé

1. Vérifier sur le job E2E Obscura que les fixtures de #280 (dont 8.5 et 11.9 pour axe) se classent correctement sous
   le vrai navigateur ; c'est leur seule validation hors jsdom.
2. Étendre les sondes comportementales Obscura pour couvrir les gestes non destructifs, le survol, l’annulation et les alternatives tactiles sans déclencher d’action métier.
3. Remplacer les signatures heuristiques inter-pages par une observation validée des composants et destinations, sans convertir un crawl échantillonné en preuve de conformité de tout le site.
4. Mesures dépendant de la mise en page (preuve Obscura vs Chrome).
5. Routage par moteur (`EnginePlan::primary`) et coûts Holo : change les verdicts, passe par #180 et #183.

## 7. Pour un agent qui contribue

- Lire [AGENTS.md](../AGENTS.md) (conventions Rust, commandes de build/test, revue des bots) puis ce document.
- Ajouter un contrôle : sonde dans `rgaa-rules` → entrée `[[mechanism]]` dans `mechanisms.toml` → fixtures
  `<critère>-<nom>-pass.html` et `-fail.html` → `cargo nextest run -p rgaa-core -p rgaa-rules -p rgaa-test-corpus`.
- Ne jamais émettre `Pass` depuis un mécanisme `partial`. Ne jamais traiter un outil absent comme un succès.
- Le build complet du workspace est long (dépendances `lance`) : testez crate par crate.
- Suivi : issues et cartes Wayfinder (#247 couverture, #180 flux économique, #183 confiance et mesure).
