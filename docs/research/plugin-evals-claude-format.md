# Plugin evals Claude : format, graders, bonnes pratiques

> Recherche pour [#214](https://github.com/jamon8888/Holo-RGAA/issues/214), ticket de la
> carte [#211 Plugin RGAA : stratégie d'audit de bout en bout + design des evals](https://github.com/jamon8888/Holo-RGAA/issues/211).

## Question

Quel est le format officiel des « plugin evals » de Claude Code (structure de dossier, format des cas, graders disponibles, exécution en CI) tel qu'il est documenté par Anthropic — et ce format peut-il accueillir à la fois des évals de comportement du plugin `rgaa-consultant` et des évals de précision d'audit (faux « PASS », couverture non déclarée, chiffres de la déclaration d'accessibilité) ? Sources primaires uniquement.

## Findings

### 1. Origine, commande et périmètre du format

- La source officielle unique est la page `plugin evals` : <https://code.claude.com/docs/en/plugin-evals> — « Write eval cases for your Claude Code plugin, run them with claude plugin eval, grade the results, compare against a no-plugin baseline, and gate CI on the score. » L'index des docs ne contient qu'une seule page dédiée (https://code.claude.com/docs/llms.txt).
- Commande : `claude plugin eval` ; scaffold interactif `claude plugin eval init` (« asks you about your plugin, proposes the cases and graders, tries them, and writes the files »), ou `claude plugin eval init --bare <name>` pour un gabarit vide (nécessaire en CI, l'interactif a besoin d'un terminal) — https://code.claude.com/docs/en/plugin-evals#create-an-eval-suite-et-init.
- Prérequis : Claude Code **v2.1.269 ou ultérieur**, **git 2.31 ou ultérieur** si git est installé (avec un git plus ancien la commande s'arrête avant de lancer le moindre cas, car elle s'appuie sur `GIT_CONFIG_COUNT`, ajouté en git 2.31, pour neutraliser hooks et helpers du dépôt) ; sans git, la suite tourne normalement — même page, section Requirements et section « git is too old ».
- Les appels de modèles (exécutions, juges, `init`) sont facturés au plan ou à l'API (« they count against your plan's usage limits or your API bill »).
- Le format n'est **pas** celui de l'outil skill-creator : celui-ci utilise son propre `evals/evals.json` dans le dossier du skill et « neither tool reads the other's case files » / « The two formats aren't interchangeable » — https://code.claude.com/docs/en/skills#run-evals-with-skill-creator. `claude plugin validate` ne contrôle que syntaxe et schéma, pas le comportement — https://code.claude.com/docs/en/plugins/cli-reference#plugin-validate.

### 2. Structure du dossier et format des cas

- La suite vit dans `evals/` à la racine du plugin. Elle se déplace par la clé de manifest `experimental.evals` (« Directory that holds the plugin's eval cases when it isn't the default evals/ ») ou par `--eval-dir` (le drapeau l'emporte ; chemin relatif de noms de dossiers simples — un chemin absolu ou contenant `..` est rejeté) — https://code.claude.com/docs/en/plugins/manifest-reference#fields et https://code.claude.com/docs/en/plugin-evals#use-a-different-eval-directory.
- Un **cas** = un sous-dossier contenant `prompt.md`, `case.yaml`, ou les deux ; il doit avoir au moins un grader, sinon il échoue au chargement (erreur `invalid case.yaml` nommant `graders`). Imbriquer des dossiers qui ne sont pas des cas sert à les grouper — section « Write and refine cases ».
- `prompt.md` : le **front-matter** porte les champs du cas (`description`, `tags`, `runs`, `model`, `max_turns`, `timeout_seconds`, `allowed_tools`, `append_system_prompt`, `env`, `plugins`, `expected_outcome`) ; le **corps** est le prompt envoyé à Claude dans chaque run.
- `graders/<name>.md` : le nom de fichier **est** le nom du grader ; front-matter = `type` (obligatoire), `weight` (défaut 1), `arm` ; le corps sert de critères pour les grader `llm`.
- `case.yaml` : exige `schema_version: "1.1"` et `name`, et ajoute les champs qui pointent vers d'autres fichiers — `context.scaffold_script` (script Bash dans le dossier du cas, exécuté dans le workspace vide avant Claude, **uniquement avec `--scaffold`**, limite 120 s, exit non nul = run échoué), `context.history_file` (transcript `.jsonl` repris comme tour utilisateur suivant ; un tel cas ne tourne qu'un seul bras par défaut), `context.add_dirs` (dossiers en lecture seule), `plugins` (ex. `["../.."]`), `execution.*` (champs d'exécution), `graders`. Quand les deux fichiers existent : le front-matter de `prompt.md` **écrase** les champs correspondants de `case.yaml`, le corps de `prompt.md` est le prompt, et les `graders/*.md` sont **ajoutés après** les graders listés dans `case.yaml` — section « case.yaml fields ».
- `evals/mocks/<server>/<tool>.md` (partagé) ou `<cas>/mocks/` (par cas) : chaque fichier répond à un outil MCP ; corps = résultat renvoyé avec substitutions `{{input.<field>}}` et `{{file:fixtures/<name>}}` ; front-matter `type` (`fixed`/`agent`), `expect` (validation des entrées — une violation **interrompt le run avec un score 0**, rapporté `aborted`), `error`, `abort_when`. Un appel à un outil **mocké n'a pas besoin d'autorisation** (`--allow-tools`) ; un vrai serveur MCP exige `--allow-real-servers` ou `--mocks off` **et** une autorisation par nom (`mcp__plugin_<plugin>_<server>__<tool>`) — sections « Mock MCP servers » et « Tool permissions ».
- `evals/results/<horodatage>/report.html` : fichier autonome, sans requête externe, jointable à un job CI. Arbre complet dans la section « eval suite reference » (dossier `evals/` : cases, `mocks/`, `results/`).
- Isolation des runs : aucun réglage utilisateur, hook, `CLAUDE.md`, serveur MCP, autre plugin, mémoire ; rien n'est lu dans `.claude/` du workspace ; « The case definitions are hidden from the agent » (le cas ne voit ni son prompt, ni ses graders, ni les cas voisins) ; seuls un allowlist d'env et des variables `EVAL_*` sont transmis — section « What a run can access ».

### 3. Graders, assertions, agrégation, CI

**Six types de graders, sans code personnalisé** — « There are no custom-code graders. » — https://code.claude.com/docs/en/plugin-evals#grader-types :

| Type | Options | Passe quand |
| --- | --- | --- |
| `regex` | `pattern`, `flags`, `match`, `target` | la regex JS `pattern` est trouvée dans la cible ; `match: not_contains` exige l'absence, `match: "count:N"` exactement N occurrences |
| `tool_used` | `tool`, `input_match`, `min`, `max` | nombre d'appels à `tool` dont l'entrée JSON matche `input_match` entre `min` (défaut 1) et `max` (défaut ∞) ; `min: 0` + `max: 0` = outil jamais appelé |
| `tool_order` | `before`, `after` | les deux outils appelés et le premier appel `before` précède le premier appel `after` |
| `file_exists` | `path`, `exists` | un fichier **créé pendant le run** matche le glob ; seuls les fichiers créés au run comptent |
| `llm` | `criteria`, `focus` | le juge vote PASS au moins 2 fois sur 3 ; corps du `.md` = critères |
| `baseline` | `baseline_file`, `criteria` | le juge estime que le run satisfait les critères **au moins aussi bien que la transcript de référence** `baseline_file` (un `.jsonl` dans le dossier du cas) |

**Cibles** (`target` pour `regex`, `focus` pour `llm`, mêmes valeurs) : `last_message` (défaut), `trace` (session JSON, un message par ligne ; le juge voit les 12 premiers et 12 derniers messages), `files` (**liste des chemins créés** pendant le run — ni leur contenu, ni les fichiers créés par un scaffold ou simplement modifiés), `{ source: file, path: <chemin> }` (contenu d'un fichier du workspace **après** le run — un PNG/JPEG/GIF/WebP est montré au juge comme image, un PDF ou `.pptx` est refusé), `mock_calls` (chaque appel à un outil MCP mocké, avec entrée et réponse).

- Pondération : `weight` par grader ; `arm: with-only` exclut un grader du scoring à deux bras, `arm: both` force un grader autrement exclu. Score d'un run = fraction **pondérée** des graders passés ; score du cas = moyenne sur ses runs (**3 runs par défaut**) ; un cas passe quand son score ≥ `--threshold` (défaut **1.0**) — section « Scoring and reports ».
- Rapport JSON : `aggregates.casesPassed`, `aggregates.casesTotal`, `overallScore`, `meanDelta`, bras `cases[].arms.without` ; le rapport HTML affiche la ligne de verdict « Plugin effect: +33.3 pts vs baseline… » et, par run, les votes du juge et ses preuves ; les graders qui ne comptent pas dans le score (ex. `tool_used: Skill`) portent un badge `plugin-fired indicator`.
- `expected_outcome` : « For humans. Not used at run time » — un champ documentaire, pas une assertion.
- Le **bras par défaut** est avec/sans plugin (`--ablation with-without` / `none`) ; le `Δ` (Ablation Δ) est rapporté mais **ne change jamais le code de sortie**.
- CI (section « Run evals in CI ») : un extrait shell exécute la suite avec `--json results.json`, `--threshold 0.8`, modèles épinglés (`--model`, `--judge-model`), `--no-publish`, `--max-cost-usd` et `--trust-plugin`, et fait échouer le build sur le code de sortie : **0** = tous les cas ≥ threshold et tous les fichiers chargés ; **1** = un cas sous le threshold, un fichier de cas non chargé, aucun cas, run impossible, plugin non approuvé sans `--trust-plugin`, ou option invalide. La page impose aussi : installation de Claude Code + identifiants dans l'environnement (ex. `ANTHROPIC_API_KEY`), approbation du dossier (`--trust-plugin`, sinon refus exit 1 hors TTY), et `init --bare` en CI.
- `Bash`/`PowerShell` accordés impose un backend de sandbox OS : sur Linux il faut `bubblewrap` et `socat`, sous Windows natif il n'y en a pas (WSL2 requis), sinon chaque run est refusé (score 0 le plus souvent).
- Hooks et vrais serveurs MCP tournent **hors sandbox de l'agent** : « treat its scores as advisory unless you ran it in an isolated environment such as a container or CI runner, since hooks and servers … could modify the files the graders read » — section « Security ».

### 4. Évals de comportement de plugin et évals de précision d'audit dans ce format

**Ce que le format couvre factuellement (comportement) :**

- Un cas = un prompt « qu'un utilisateur taperait », noté sur la transcript et les fichiers produits ; c'est exactement le modèle décrit pour les skills, où `tool_used: Skill` mesure le déclenchement — https://code.claude.com/docs/en/skills (« `tool_used: Skill` grader recommended »).
- Le bras avec/sans plugin mesure l'apport du plugin (delta, améliorations/régressions par cas) ; les mocks permettent de simuler les sorties d'outils (ex. axe-core, MCP) sans service réel ; les fixtures passent par `context.scaffold_script` (avec `--scaffold`) ou `context.add_dirs`.
- Exemple réel de suite dans ce format : `homeassistant-ai/skills`, ~30 cas sous `evals/<cas>/case.yaml` (graders `tool_used` avec `input_match` sur `Skill`, `regex` complexes sur des sorties YAML) — https://github.com/homeassistant-ai/skills/tree/main/evals (son workflow `validate-skills.yml` ne fait que de la validation de structure, il n'exécute pas `claude plugin eval`).

**Ce que le format couvre factuellement (précision d'audit) :**

- Le rapport d'audit produit peut être noté comme fichier : grader `regex` sur `{ source: file, path: … }`, ou grader `llm` avec `criteria` (rubrique dans le corps du `.md`) et `focus` sur ce même fichier — la doc décrit ce cas explicitement : « Use this to grade what the plugin produced ».
- La comparaison à une référence n'existe que sous deux formes : `regex`/`llm` contre des critères écrits à la main, ou grader `baseline` contre **une transcript `.jsonl`** de référence — jamais contre un document de référence (audit golden) ; aucun type de grader ne fait un diff artefact-vs-référence.
- Aucun code de grader personnalisé : pas de métrique agrégée maison (taux de faux PASS par critère, rappel par critère RGAA) ; l'agrégation est uniquement fraction pondérée par run → moyenne par cas → moyenne de suite, et `--threshold` sur le score de cas.
- Un cas = un prompt ; il n'existe pas de champ « dataset de N pages auditées » dans un cas : la variété s'obtient par N dossiers de cas sous `evals/` (la doc recommande de les imbriquer pour les grouper).
- Côté recette générale (hors format plugin), le cookbook officiel « Building evals » distingue grading par code / humain / modèle autour d'une « golden answer » — https://platform.claude.com/cookbook/misc-building-evals — mais ce n'est pas le format plugin.
- Dans ce dépôt : aucun dossier `evals/` n'existe (recherche `**/evals/**` : aucun résultat) ; le plugin `rgaa-rs/plugins/rgaa-consultant/` contient `.claude-plugin/plugin.json`, `.mcp.json`, `commands/`, `skills/`, `README.md`, `SPEC.md`, `CONNECTORS.md`.

**Exemples réels d'usage en CI (tiers, pas Anthropic) :** `Nanako0129/sepia` publie `.github/workflows/behavioral-eval.yml` qui exécute `claude plugin eval . --json results.json --threshold 0.7 --model claude-sonnet-5 --no-publish --trust-plugin` avec `CLAUDE_CODE_OAUTH_TOKEN` — https://github.com/Nanako0129/sepia/blob/main/.github/workflows/behavioral-eval.yml. La recherche de code GitHub `q="claude plugin eval" path:.github` renvoie 96 résultats.

## Not verified

- **Pas d'exemple GitHub Actions officiel** exécutant `claude plugin eval` : la section CI de la page plugin evals ne donne qu'un extrait shell et une table de codes de sortie ; la page GitHub Actions (https://code.claude.com/docs/en/github-actions) ne traite que de `anthropics/claude-code-action` et ne mentionne pas `plugin eval` ; idem pour la page GitLab CI/CD (https://code.claude.com/docs/en/gitlab-ci-cd).
- **Aucun workflow d'un dépôt anthropics** n'exécute `claude plugin eval` : `anthropics/claude-plugins-official` n'a qu'une seule path `evals` (`plugins/math-olympiad/skills/math-olympiad/evals/trigger_eval.json`, format `{"query","should_trigger"}` différent), et ses workflows (`.github/workflows/` = `bump-plugin-shas`, `check-mcp-urls`, `validate-plugins`, `validate-frontmatter`, `validate-licenses`, `scan-plugins`, `apply-auto-fixes`) ne lanceront pas la commande.
- `CLAUDE_CODE_WALNUT_SPIRE: "1"` (affirmé par le workflow tiers de sepia comme activant un `plugin eval` en accès anticipé) : **introuvable dans la documentation officielle** ; les docs précisent seulement qu'une commande peut être en accès anticipé et désactivée côté serveur (page troubleshooting).
- **Invocation des `commands/`** : la doc ne dit pas si un appel de slash-command apparaît dans la transcript de façon exploitable par `tool_used` — non trouvé.
- Aucune suite publique trouvée au format `prompt.md`/`grader .md` avec une référence d'audit multi-fichiers ; les suites trouvées utilisent `case.yaml` (homeassistant-ai) ou des formats distincts (skills `evals/evals.json`, `trigger_eval.json`).
- Existence d'un métrique agrégé de type faux-PASS / couverture par critère dans le format : **non trouvé** (l'agrégat documenté est le score pondéré et le compteur `casesPassed/casesTotal`).
- Statut « early-access » ou éventuelles autres commandes liées : non vérifié au-delà des pages citées.

## Sources

- https://code.claude.com/docs/en/plugin-evals — format, cas, graders, scoring, CI, sécurité (page officielle, référencée par https://code.claude.com/docs/llms.txt)
- https://code.claude.com/docs/en/plugins/cli-reference — `claude plugin eval` / `eval init`, options, codes de sortie
- https://code.claude.com/docs/en/plugins/manifest-reference#fields — `experimental.evals`
- https://code.claude.com/docs/en/skills — format `evals/evals.json` du skill-creator et sa non-interchangeabilité ; grader `tool_used: Skill`
- https://code.claude.com/docs/en/github-actions — page officielle GitHub Actions (aucune mention de `plugin eval`)
- https://code.claude.com/docs/en/gitlab-ci-cd — page officielle GitLab (aucune mention de `plugin eval`)
- https://platform.claude.com/cookbook/misc-building-evals — recette officielle « Building evals » (golden answer, grading par code/humain/modèle)
- https://github.com/anthropics/claude-plugins-official — arborescence et `.github/workflows/` (aucun `claude plugin eval`) ; `plugins/math-olympiad/skills/math-olympiad/evals/trigger_eval.json`
- https://github.com/homeassistant-ai/skills/tree/main/evals — suite réelle au format `case.yaml` ; https://github.com/homeassistant-ai/skills/blob/main/.github/workflows/validate-skills.yml (validation de structure uniquement)
- https://github.com/Nanako0129/sepia/blob/main/.github/workflows/behavioral-eval.yml — CI réel lançant `claude plugin eval`
- `rgaa-rs/plugins/rgaa-consultant/` (dépôt local) — layout du plugin, absence de dossier `evals/` (recherche `**/evals/**` sans résultat)
