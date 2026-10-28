# Les trois couches de test de rgaa-rs

Ce qui tourne, où, et ce que chaque couche prouve — ou ne prouve pas.

| Couche | Job CI | Besoins | Tourne sur chaque PR |
|---|---|---|---|
| Unitaires + intégration Rust | `Build & Test` | — | oui |
| Harnais DOM des sondes JS | `Probe harness (DOM, sans navigateur)` | Node 20 | oui |
| E2E navigateur | `E2E (navigateur)` | binaire Obscura 0.2.4 | oui |
| E2E pipeline complet | `E2E (agent + orchestrator…)` | Obscura **+ clé LLM** | **non** |
| E2E réseau du crawler | *(aucun)* | accès réseau sortant | **non** |

Les trois premières sont inconditionnelles : aucune variable de dépôt, aucun
secret à provisionner.

La quatrième reste conditionnée à `vars.MYIA_E2E_ENABLED == 'true'` et au secret
`MYIA_API_KEY`, ni l'un ni l'autre défini à ce jour. Ses trois tests
(`full_audit`, `obscura_audit`, `fixture_classification`) traversent le pipeline
complet et ont réellement besoin d'une clé LLM : **ils ne sont donc pas
vérifiés**, et ne le seront pas avant que la variable et le secret existent.
C'est un choix assumé — on ne peut pas provisionner une clé API dans un commit —
et non un oubli.

La cinquième ligne est un **second** endroit non vérifié, et il n'est lié à aucun
secret : les trois tests de `rgaa-spider/tests/spider_tool_test.rs` interrogent le
réseau et ne s'exécutent que sous `RUN_E2E=1`, que `Build & Test` ne pose pas. Leur
commentaire dit « e.g. in the CI `e2e` job », mais ce job ne les lance pas — sa
liste de `--test` est explicite et ne les contient pas — et il est de toute façon
conditionnel. Ils ne tournent donc nulle part. Les rendre effectifs demande une
politique sur les tests qui sortent du runner (quels sites, quelle tolérance aux
pannes réseau), pas un correctif de couverture : ce n'est pas tranché ici. La
différence avec l'état précédent est que le non-vérifié est maintenant **écrit**.

## Pourquoi cette page existe

Jusqu'à la correction du harnais, les tests `criterion_*_coverage` de
`rgaa-obscura` **ne tournaient nulle part** tout en ressemblant à une
couverture automatisée. Deux gardes empilées, et chacune suffisait :

1. le job E2E était conditionné à `vars.MYIA_E2E_ENABLED == 'true'`, variable
   jamais définie — le job passait au vert en 3 s sur chaque PR ;
2. aucun job ne posait `RUN_E2E=1`, que chaque test vérifie en tête avant de
   sortir **sans rien affirmer**.

La garde `MYIA_E2E_ENABLED` / `MYIA_API_KEY` n'avait aucune raison de s'appliquer
à eux : aucun test de `rgaa-obscura` n'appelle de LLM. Ils payaient un coût
qu'ils n'avaient pas. Ils vivent désormais dans `E2E (navigateur)`, sans
condition. Le job gardé par le secret subsiste pour les trois tests qui, eux, en
ont besoin — attendre une clé API n'était pas une raison de n'exécuter *rien*.

Un test vert qui n'a rien vérifié est précisément le défaut que cette suite
corrige dans le produit — un `pass` sur le silence. Il n'avait pas sa place
dans son propre harnais.

## Lancer l'E2E navigateur en local

Il faut le binaire Obscura épinglé (`obscura 0.2.4`) **sur le `PATH`**. Les tests
de couverture construisent leur pont par `ObscuraBridge::new()`, qui ne connaît que
le `PATH` : la variable `RGAA_OBSCURA_BIN` est lue par `ObscuraBridge::from_env()`,
que n'utilise aucun d'entre eux, et ne les redirige donc pas. Un autre `obscura`
déjà présent sur le `PATH` (une 0.1.x, par exemple) sera pris à sa place sans
avertissement : vérifier `obscura --version` d'abord.

```bash
cd rgaa-rs
RUN_E2E=1 cargo nextest run -p rgaa-obscura -E 'binary(/^criterion_.+_coverage$/)'
```

Sans `RUN_E2E=1`, chaque test sort en tête et l'affiche sur stderr. Les deux
jobs E2E posent aussi `RGAA_E2E_REQUIRED=1`, qui rend cette sortie **fatale** :
si la CI perd `RUN_E2E`, le job échoue au lieu de rapporter un vert vide. La
politique vit dans `rgaa_test_corpus::e2e::enabled`
(`crates/rgaa-test-corpus/src/e2e.rs`), partagée par `rgaa-obscura` et
`rgaa-orchestrator` : un seul endroit à relire pour savoir ce qui tourne.

### Ports CDP

Chaque invocation obtient son propre port, via
`ObscuraBridge::with_ephemeral_port()`. Les ports fixes (9341–9347) utilisés
auparavant rendaient deux exécutions concurrentes mutuellement destructrices :
la sonde de disponibilité de `start_server` acceptait *n'importe quelle*
réponse `/json/version` sur le port, donc la seconde exécution s'attachait au
serveur de la première et perdait sa connexion CDP dès que celle-ci sortait.

Pour un port alloué par `with_ephemeral_port()`, `start_server` refuse en plus, en
préalable, un port déjà occupé, avec un message qui nomme le port, et n'accepte la
disponibilité que tant que l'enfant qu'il a lancé tourne encore : un enfant qui perd
le `bind` sort en quelques millisecondes, donc la réponse d'un serveur étranger n'est
plus prise pour la nôtre. Ça réduit la fenêtre entre l'allocation du port et le `bind`
d'`obscura serve` ; ça ne la ferme pas entièrement. Il signale aussi un `obscura
serve` mort au démarrage avec son code de sortie, et **arrête l'enfant sur toute
erreur de démarrage**, pour qu'un nouvel essai sur le même pont ne soit pas refusé par
son propre serveur résiduel.

**Ce que ça ne change pas : la production.** Le port par défaut (`from_env()`, utilisé
par `rgaa-api`, `rgaa-mcp` et la CLI) et `with_port(...)` gardent leur comportement
d'avant, sans refus de port occupé : plusieurs audits qui se chevauchent démarrent un
pont sur 9222 et partagent le serveur du premier. Ce partage est fragile — le pont qui
possède le serveur l'arrête en sortant, y compris sous un audit encore en cours — mais
passer chaque lot à un port propre multiplie les processus `obscura serve` (un worker
par CPU chacun) et change le modèle de concurrence de la production. C'est une décision
séparée, pas un effet de bord d'un changement de harnais de test.

## Harnais DOM des sondes (complément, pas substitut)

`crates/rgaa-rules/tests/js/` exécute les chaînes JS embarquées dans
`plan_snippets.rs` et `gap_fix.rs` contre un `document` factice ou jsdom, sous
Node. Pas de navigateur, pas de binaire Obscura, quelques secondes.

```bash
cd rgaa-rs/crates/rgaa-rules/tests/js
npm ci && ./run-harnesses.sh
```

Ce que ça couvre : le parsing du balisage et la sélection de candidats —
l'essentiel de ce qu'une sonde statique doit garantir, et ce qui se casse en
silence quand on retouche un sélecteur. C'est la couche où une régression de
sonde est la moins chère à attraper.

Ce que ça **ne** couvre pas, et pourquoi ça ne remplace pas l'E2E navigateur :
aucun rendu, aucun layout, aucun calcul de style réel, aucun comportement
d'axe-core. Les défauts du genre #371 (une règle axe qui tombe en `incomplete`
via ce pipeline là où un vrai Chrome rend `violations`) ne sont visibles que
dans un navigateur.

`run-harnesses.sh` lance tout `*.js` du répertoire, avec une liste
d'exclusions explicites et justifiées, et échoue si une exclusion est périmée
ou si rien ne reste à lancer. Un nouveau harnais déposé là est donc pris
automatiquement plutôt qu'oublié.

`plan_snippets_check.js` est exclu : c'est un outil de diagnostic, pas un
garde. Il exige `pass === true` sur chaque fixture `-pass`, alors que les
sondes en revue seule ne rendent délibérément jamais `pass`, et il compte
« NO FIXTURE » comme un problème pour les 24 sondes sans fixture déclarée. Il
rapporte 54 problèmes sur un arbre sain. À réécrire en garde (attendre
`outcome`, pas `pass`) ou à retirer.
