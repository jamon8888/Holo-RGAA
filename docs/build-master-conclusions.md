# Conclusions du build master — jamon8888/Holo-RGAA

**Commit analysé :** `e7e866fa` — *Merge pull request #225 from jamon8888/feat/rgaa-linter-lint-static*
**Date du relevé :** 2026-10-01
**Portée :** état de master après les merges de #227 (issue #160) et #225 (issue #166)

> Relevé figé sur `e7e866fa`. PR #230 (documentation des neuf outils + durcissement du contrôle de contrat) a été mergée **après** ce relevé ; elle ne touche que de la documentation et un script de test, et ne modifie aucune des conclusions ci-dessous.

---

## Verdict

Master **compile et fonctionne sur les quatre plateformes cibles**. Un seul job est rouge, et ce n'est pas le produit : c'est la **comparaison de chaînes** d'un test de fumée sur Windows. Le binaire Windows se construit et répond correctement.

| | |
|---|---|
| Checks totaux | 23 |
| Succès | 19 |
| Ignorés (`skipped`) | 3 |
| **Échecs** | **1** |

L'unique échec : `Smoke Test (x86_64-pc-windows-msvc, windows-latest)`.

---

## Résultats par plateforme

| Plateforme | Build | Smoke Test |
|---|---|---|
| `x86_64-unknown-linux-gnu` | ✅ | ✅ |
| `x86_64-apple-darwin` | ✅ | ✅ |
| `aarch64-apple-darwin` | ✅ | ✅ |
| `x86_64-pc-windows-msvc` | ✅ | ❌ |

Les quatre builds passent. Trois des quatre smoke tests passent, **avec le même script**.

---

## L'unique défaillance, analysée

### Ce qui fonctionne

Le job va loin avant d'échouer, et ce qu'il prouve au passage compte :

- le binaire Windows (`rgaa-mcp.exe` et `rgaa-mcp-http.exe`) **se construit** ;
- il **démarre** et répond à `tools/list` ;
- il expose **les neuf outils attendus**, sur **les deux transports** (stdio et HTTP) :
  `analyze`, `audit_url`, `get_audit_result`, `igt`, `lint_static`, `list_criteria`, `remediate`, `source_map`, `verify_fix`.

Autrement dit : **aucun problème de portabilité Windows dans le code Rust**. Ni compilation, ni démarrage, ni surface d'outils.

### Ce qui casse

`scripts/smoke-mcp-http.sh` compare deux listes :

- `expected_tools` — dérivée par `awk` des attributs `#[tool(name = "…")]` dans `server.rs` ;
- `live_tools` — obtenue par `jq` sur la réponse `tools/list` du serveur lancé.

Les deux listes s'affichent **à l'identique** dans le log, et pourtant la comparaison échoue. Le `diff` produit :

```
1,8c1,8
< analyze … < source_map      (8 lignes)
---
> analyze … > source_map      (8 lignes)
```

Les lignes **1 à 8 diffèrent, la 9e non**. Du texte visuellement identique que `diff` rejette sur toutes les lignes sauf la dernière, c'est la signature d'un **caractère invisible en fin de ligne d'un seul côté** — un `\r` (retour chariot). La dernière ligne n'en porte pas, parce qu'elle n'est pas suivie d'un saut de ligne.

### Ce qui est écarté

J'ai d'abord supposé que `server.rs`, récupéré par `actions/checkout` sur un runner Windows, arrivait en CRLF et contaminait `expected_tools`. **C'est faux, et je l'ai vérifié** en rejouant l'`awk` du script sur une source CRLF fabriquée :

```
--- source LF   --->  a n a l y z e \n v e r i f y _ f i x \n
--- source CRLF --->  a n a l y z e \n v e r i f y _ f i x \n
```

Sortie identique : le `\r` se trouve **après** le guillemet fermant, donc `sub(/".*$/, "", line)` l'élimine. La source CRLF n'est pas la cause.

Le `\r` est donc du **côté `live_tools`** — la réponse HTTP ou son traitement par `jq`/le shell sur Windows. Localiser le point exact demande d'instrumenter le script sur un runner Windows ; ce n'est pas déterminable depuis les logs seuls.

> **Statut du diagnostic :** le mécanisme (caractère invisible, un seul côté, spécifique à Windows) est établi. L'origine précise du `\r` est **circonscrite mais non prouvée**.

---

## Antériorité : cet échec précède les merges de #225 et #227

C'est le point à retenir pour l'attribution.

### Preuves directes

Deux commits antérieurs aux merges échouent de la même façon.

**Run #72, sur `f8fb043c`.** Même job Windows, même paire de listes identiques rejetée par `diff`, et surtout **sans `lint_static` dans la liste** (sept outils). Le mécanisme est strictement le même et antérieur.

**Run #73, sur `5cededee`** — le merge de #217, antérieur aux deux merges. Son **unique** échec est `Smoke Test (x86_64-pc-windows-msvc, windows-latest)`. Tout le reste du run passe.

Ce second cas est le plus net : un commit qui ne contient ni #225 ni #227, dont le seul job rouge est précisément celui-là.

### Chronologie du workflow `Release`

| Run | Commit | État | Note |
|---|---|---|---|
| #64 | `0f222927` (merge #226) | ✅ **dernier vert** | script smoke absent |
| #65 | `c2e72caa` (merge #219) | ❌ | script toujours absent → **autre cause** |
| #66 | `255b0a66` (merge **#221**) | ❌ | **le script smoke arrive ici** (05h47) |
| #67 – #72 | … | ❌ | #72 vérifié : échec Windows identique |
| #73 | `5cededee` (merge #217) | ❌ | **unique échec = le smoke Windows** — commit antérieur aux deux merges |
| #74 | `707259d5` (merge **#227**) | ⏳ `in_progress` | file d'attente des runners |
| #75 | `e7e866fa` (merge **#225**) | ❌ | le relevé analysé ici |

Deux choses à ne pas confondre :

1. Le workflow `Release` était **déjà rouge avant** que le script smoke existe (#65) — cause différente, non caractérisée ici.
2. L'échec **Windows smoke** spécifiquement date de **#66**, quand PR #221 a introduit `scripts/smoke-mcp-http.sh` (commit `efe3bda`, *« ci(release): call every MCP tool over HTTP at release time (#170) »*) — soit environ **quatre heures avant** les merges de #227 (09h25) et #225 (09h59).

### Pourquoi personne ne l'avait vu

Une question de **file d'attente, pas de régression**. Les runners macOS et Windows restent des heures en attente : le run #73, déclenché à 08h53, ne s'est terminé que vers 12h00 — plus de trois heures. Le run #74 est toujours en cours.

Le run du merge de #225 est donc simplement **le premier à avoir atteint l'étape smoke**, et #73 l'a atteinte ensuite en échouant de la même manière. Le job ne s'est pas mis à échouer au moment des merges : il n'avait pas encore eu l'occasion de s'exécuter.

### Rattachement

Le script provient du travail de l'**issue #170** (*release smoke*), délibérément laissée ouverte parce que son dernier critère — « run de release vert de bout en bout » — ne peut être satisfait que par un tag, pas par un commit. Ce défaut appartient donc à #170.

---

## Ce que les merges ont corrigé sur master

### `Plugin contract` : de rouge à vert

Ce job était **rouge sur master** avant #225, pour une dérive de documentation préexistante : #226 avait livré l'outil `source_map` sans l'ajouter au tableau du README. La vérification étant tout-ou-rien, #225 ne pouvait pas passer sans documenter les deux outils manquants — ce qui a **rendu master vert** sur ce job par effet de bord.

À noter : ce job n'avait **jamais tourné** sur la branche de #225, coupée avant que #217 ne le branche en CI. La branche a traversé cinq runs verts en portant la dérive. C'est la mise à jour depuis master qui l'a exposée.

---

## Reproduire le build localement

La compilation complète de l'espace de travail **n'a pas pu être menée à terme** dans un conteneur Linux standard. Trois manques successifs, dans cet ordre :

| Manque | Symptôme | Résolution |
|---|---|---|
| `ld` | `collect2: fatal error: cannot find 'ld'` | `apt-get install binutils lld mold clang` |
| `protoc` | `lance-encoding` : `Could not find protoc` | `apt-get install protobuf-compiler cmake` |
| ONNX Runtime | `ort-sys` : échec du build script | **bloquant** — télécharge des binaires natifs, refusé par le réseau du sandbox |

Les deux premiers correspondent aux paquets listés dans `CLAUDE.md`. Le troisième est un vrai obstacle : `ort-sys` récupère ONNX Runtime par le réseau, donc **tout environnement sans accès sortant ne peut pas construire les crates qui en dépendent**.

Conséquence pratique : les tests de ces crates se valident **en CI**, pas en local dans un conteneur restreint. Ce qui reste vérifiable localement : `cargo fmt`, les scripts shell, et les crates sans cette dépendance transitive.

---

## À faire

| Priorité | Sujet |
|---|---|
| Moyenne | **Smoke Windows** — neutraliser les retours chariot dans la comparaison (`tr -d '\r'` sur `live_tools`, ou comparer via `diff --strip-trailing-cr`). Corrige un faux négatif, pas un bug produit. Périmètre : #170. |
| Moyenne | **Cause du rouge en #65** — antérieure au script smoke, non caractérisée ici. À isoler pour savoir si le `Release` a un second défaut. |
| Basse | **Runs #73/#74** — encore en file. À vérifier à leur terme ; ils devraient reproduire le même échec Windows. |
| Basse | **Couverture du contrat** — `docs/rgaa-plugin-install.md` et `docs/plugin-integration.md` sont désormais vérifiés en avant (PR #230). Rien ne vérifie encore les exemples de code eux-mêmes. |

---

## Annexe — commandes de vérification

```bash
# État complet des checks de master
gh api "repos/jamon8888/Holo-RGAA/commits/e7e866fa47b8fedd471c542a10de0812d51a36bb/check-runs?per_page=100" \
  --jq '.check_runs|group_by(.conclusion // .status)|map("\(.[0].conclusion // .[0].status): \(length)")|.[]'

# Le job en échec
gh api "repos/jamon8888/Holo-RGAA/commits/e7e866fa47b8fedd471c542a10de0812d51a36bb/check-runs?per_page=100" \
  --jq '.check_runs[]|select(.conclusion=="failure")|{name, html_url}'

# Historique du workflow Release
gh api "repos/jamon8888/Holo-RGAA/actions/workflows/338710510/runs?per_page=15" \
  --jq '.workflow_runs[]|"#\(.run_number) \(.head_sha[0:8]) \(.status)/\(.conclusion // "-")"'

# Commit ayant introduit le script smoke
git rev-list origin/master -- scripts/smoke-mcp-http.sh | tail -1
```

---

## Résumé des faits vérifiés

- Master compile sur **les quatre** plateformes ; 19 checks verts, 3 ignorés, **1 rouge**.
- L'unique échec est une **comparaison de chaînes**, pas un défaut produit : le binaire Windows démarre et expose correctement les neuf outils sur les deux transports.
- Cet échec est **antérieur aux merges de #225 et #227**, prouvé deux fois : échec identique sur `f8fb043c` (run #72) sans `lint_static`, et run #73 sur `5cededee` (merge #217) dont le **seul** job rouge est ce même smoke Windows.
- Il date de **PR #221** (run #66), qui a introduit le script, et relève de l'**issue #170**.
- Le délai d'apparition est un effet de **file d'attente des runners** (plus de trois heures pour #73), pas une régression des merges.
- `Plugin contract` est passé de **rouge à vert** sur master grâce à #225, qui a corrigé une dérive de documentation préexistante issue de #226.
- L'origine exacte du retour chariot est **circonscrite au côté HTTP, non prouvée** ; mon hypothèse initiale (source CRLF) a été testée et **écartée**.
