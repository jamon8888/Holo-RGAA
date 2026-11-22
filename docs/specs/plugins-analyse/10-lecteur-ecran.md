# P-10.a — Sidecar lecteur d'écran NVDA (Speech Logger)

**Critères** : appui à 7.1 (état annoncé avant/après action), 7.5 (message de statut restitué) et aux noms accessibles (comparaison à l'arbre AX de P-07) · **Dépend de** : P-00 (contrat et protocole sidecar), P-07 (sujets Élément), #430 (déclaration des environnements) · **Forme** : sidecar Rust, binaire `rgaa-sidecar-screenreader`, identifiant `screenreader-nvda` · **Périmètre** : P-10.a uniquement (NVDA + Speech Logger). P-10.b (add-on de point d'extension), P-10.c (VoiceOver) et P-10.d (Orca/CI) sont hors périmètre.

> Source de vérité de la conception : `docs/research/lecteur-ecran-sidecar.md` (branche `origin/research/lecteur-ecran-sidecar`), §1.2 (format du journal), §4 (protocole) et §6 (décisions). Aucun lecteur d'écran n'a été exécuté pour écrire cette spec : la validation sur un vrai NVDA reste HITL (§8).

## 1. Problème

La méthode RGAA demande des tests de restitution avec une technologie d'assistance (`NVDA + Firefox` en base de référence desktop [S36]). Computer use voit l'écran mais **n'entend pas** la synthèse vocale : les captures ne peuvent pas établir ce que NVDA prononce. Le sidecar lit donc le texte prononcé dans un fichier produit par l'add-on **Speech Logger** [S4][S5] et le joint à la preuve, sans jamais rendre de verdict (R-P1).

**Licence.** NVDA est GPL v2+ [S15] et Speech Logger GPL-2.0 [S4]. Conséquence directe de la recherche §6 : **sidecar uniquement** — aucun code NVDA n'est lié, importé ou copié dans `rgaa-*`. L'add-on est distribué à part, dans la VM ; notre binaire ne fait que lire un fichier UTF-8.

## 2. Composants

| Composant | Rôle |
|-----------|------|
| `crates/rgaa-sidecar-screenreader/` | crate du workspace ; binaire JSON-RPC stdio + bibliothèque de corrélation pure |
| `sidecar.toml` (racine du crate) | manifeste P-00 §7 bis ; à déployer dans `$RGAA_PLUGINS_DIR` pour `rgaa plugins list` |
| `journal.rs` | parseur pur du journal Speech Logger (§1.2), sans IO ni horloge |
| `protocole.rs` | configuration, suivi (tail) du fichier, horloge injectable, erreurs typées, boucle JSON-RPC |
| `correlation.rs` | association énoncé ↔ action et construction de la `Preuve` P-00 §5 |
| `rgaa-analysis::sidecar::SidecarPlugin` | client existant, réutilisé tel quel ; `avec_parametres()` transmet chemins et actions à `analyser` |
| `rgaa-report::declaration` | `environnements_utilises_avec_preuves()` branche la preuve sur la déclaration #430 |

Le sidecar n'ouvre **aucune connexion réseau** (R-P3) : les fichiers voyagent par chemins dans le répertoire temporaire de l'audit.

## 3. Protocole (P-00 §7 bis)

Manifeste livré (identique à `rgaa-rs/crates/rgaa-sidecar-screenreader/sidecar.toml`) :

```toml
id = "screenreader-nvda"
version = "0.1.0"
tests = ["7.1.1", "7.1.2", "7.1.3", "7.5.1", "7.5.2", "7.5.3"]
commande = ["rgaa-sidecar-screenreader"]
mode = "persistant"
timeout_ms = 30000
licence = "GPL-2.0 (add-on Speech Logger) ; sidecar seul, aucun code NVDA lié ni copié"
```

Échanges (un objet JSON par ligne, stdout réservé au protocole, journaux en stderr) :

```jsonc
{"jsonrpc":"2.0","id":1,"method":"initialiser","params":{"protocole":1,"options":{"source":"speech-logger","environnement":"NVDA 2026.3.1 + Firefox 141.0"}}}
{"jsonrpc":"2.0","id":1,"result":{"id":"screenreader-nvda","version":"0.1.0","tests":["7.1.1","7.1.2","7.1.3","7.5.1","7.5.2","7.5.3"],"version_modele":null}}
{"jsonrpc":"2.0","id":2,"method":"analyser","params":{
  "url":"https://exemple.fr/fixture-menu",
  "sujets":[{"selecteur":"#menu-button","xpath":"/html/body/button[1]","bbox":{"x":120,"y":210,"width":90,"height":32}}],
  "fichiers":{"parole":"/tmp/audit-123/analysis/nvda/speech.log","actions":"/tmp/audit-123/actions.jsonl"},
  "actions":[
    {"id":"a1","type":"focus","selecteur":"#menu-button","fenetre_ms":1500},
    {"id":"a2","type":"activation","selecteur":"#menu-button","declencheur":"Enter","fenetre_ms":2500,"nom_accessible_attendu":"menu"}
  ],
  "options":{"environnement":"NVDA 2026.3.1 + Firefox 141.0"}
}}
{"jsonrpc":"2.0","id":2,"result":{"preuves":[/* Preuve P-00 §5 */]}}
```

`actions.jsonl` porte les instants mesurés par le pilote de parcours, un objet par ligne : `{"id":"a1","t":"2026-11-22T09:14:00.204Z"}` / `{"id":"a2","t":"2026-11-22T09:14:02.120Z"}`. `journal` (notification `niveau`/`message`) est relayé vers `tracing` ; `arreter` ferme proprement.

Options reconnues (au `initialiser` et surchargées à l'`analyser`) : `source` (`speech-logger`), `separateur` (deux espaces par défaut), `environnement` (`"NVDA <version> + <navigateur> <version>"`), `nvda_detecte` (permet de signaler un NVDA absent), `horodatage_simule` (horloge de rejeu **debug uniquement**, jamais en audit).

## 4. Algorithme

### 4.1 Lecture du journal §1.2

Speech Logger écrit un enregistrement par séquence de parole dans un fichier UTF-8, avec un séparateur configurable (deux espaces par défaut) et des marqueurs `### Log started on … ###` / `### Log ended on … ###` en début/fin de session [S4][S5]. Le parseur est pur et testé :

1. lignes vides et marqueurs `###` ignorés ;
2. chaque ligne restante est découpée sur `separateur`, les morceaux vides écartés, puis recollés avec une espace : `Menu   sous-menu Ouvrir` → `Menu sous-menu Ouvrir` ;
3. un en-tête `# environnement: …` est reconnu quand il est présent ;
4. une ligne sans `\n` final (journal encore ouvert ou tronqué) est quand même lue ; un UTF-8 coupé est décodé avec perte, jamais paniqué.

### 4.2 Horodatage à la réception

Speech Logger n'horodate **pas** chaque énoncé [S3][S5] : le sidecar pose donc l'horodatage **au moment de la lecture** et le déclare explicitement `horodatage_type: "reception"` (recherche §4.3, règle 2). En mode persistant, `SuiveurJournal` mémorise le nombre d'octets déjà lus : à chaque appel il ne parse que les octets nouvellement apparus. L'horloge est un trait `Horloge` injecté (`HorlogeSysteme` en production, horloges fixes/simulées dans les tests) — aucun `sleep` dans les tests. Un énoncé partiel en fin de fichier est conservé pour l'appel suivant.

### 4.3 Corrélation action → parole

1. Les instants du JSONL sont joints aux définitions d'actions par `id` (ordre d'apparition conservé, `IndexMap`).
2. Chaque énoncé est associé à la première action dont la fenêtre `[t, t + fenetre_ms]` le contient (`t` inclus, `t + fenetre_ms` inclus).
3. Pour une action `activation`, on produit une preuve : `avant` = dernier énoncé **strictement antérieur** à `t`, `apres` = énoncé dans la fenêtre. Une activation sans énoncé dans sa fenêtre ne produit rien (le test reste en « preuve manquante »).
4. `horodatage_type` vaut `"reception"` pour cette voie ; `"source"` est réservé au add-on de point d'extension de P-10.b, qui horodate lui-même [S7].

### 4.4 Preuve et échecs

La preuve suit P-00 §5 : `portee: "Mesuree"` (texte lu dans un fichier produit par NVDA), `indice` éventuel `Suspect` (aide au tri), **aucun champ `status` ni `verdict`** (R-P1, rejet par `validate_preuve` côté client). `tests` = `["7.1.1", "7.5.1"]` sur la preuve d'activation, conformément à l'exemple de la recherche §4.2. L'artefact `parole.jsonl` (NDJSON, `application/x-ndjson`) est matérialisé dans un dossier temporaire, référencé sous `analysis/screenreader-nvda/<sha256>.jsonl` avec son sha256 ; `empreinte_entree` est le sha256 du journal lu et des instants d'actions.

Fichier absent, NVDA déclaré absent, JSONL d'actions invalide, artefact non écritable ou absence de corrélation ⇒ réponse JSON-RPC d'erreur `-32000` dont le message est `preuve manquante (sidecar screenreader-nvda : <raison>)`, précédée d'une notification `journal`. Le client `SidecarPlugin` transforme l'absence de preuve en `RgaaError::Analysis` ; l'audit continue. Aucun `unwrap`/`expect` hors tests, aucune panique.

## 5. `mesures`

```json
{
  "action": {"id": "a2", "type": "activation", "declencheur": "Enter"},
  "element": {"selecteur": "#menu-button", "nom_accessible_attendu": "menu"},
  "texte_prononce": "menu, bouton, coché",
  "horodatage": "2026-11-22T09:14:02.481Z",
  "horodatage_type": "reception",
  "avant": {"texte_prononce": "menu, bouton, pas coché", "horodatage": "2026-11-22T09:14:00.500Z"},
  "apres": {"texte_prononce": "menu, bouton, coché", "horodatage": "2026-11-22T09:14:02.481Z"},
  "source": "nvda-speech-logger",
  "environnement": "NVDA 2026.3.1 + Firefox 141.0"
}
```

Les quatre champs du ticket (`action`, `element`, `texte_prononce`, `horodatage`) sont portés par `mesures` ; `avant`/`apres` répondent à l'acceptation 7.1.

## 6. Déclaration des environnements (#430)

`rgaa-report` expose `environnements_depuis_preuves(&[Preuve])` et `environnements_utilises_avec_preuves(navigateur_audit, &[TestEnvironment], &[Preuve])`. La fusion part de `environnements_utilises` (#430), ajoute les environnements révélés par les preuves `plugin = "screenreader-*"`, **dédoublonne** (clé = technologie et navigateur réduits au premier mot) et conserve l'ordre d'apparition via `IndexMap`. Un environnement déjà déclaré à la main (`NVDA` + `Firefox`) n'est pas dupliqué par une preuve `NVDA x.y + Firefox z`. Une preuve sans `mesures.environnement`, ou mal formée, est ignorée avec `tracing::warn`, sans panique ni environnement inventé.

À ce jour aucun pipeline n'écrit encore les preuves P-00 dans `AuditBundle` : c'est l'appelant de la déclaration (futur pipeline) qui doit transmettre ses `Preuve` à ce helper. Le schéma d'`AuditBundle` n'est pas modifié par P-10.a.

## 7. Tests (sans NVDA)

Fixtures écrites à la main d'après le format de la recherche, aucune dépendance à une VM :

| Test | Ce qu'il vérifie |
|------|------------------|
| `journal.rs` | format §1.2, marqueurs ignorés, séparateur personnalisé `|`, journal tronqué sans `\n` final, en-tête d'environnement |
| `protocole.rs` | tail incrémental (rien à relire), horodatage à la lecture, fichier absent ⇒ erreur typée, instants joints aux actions, JSONL invalide ⇒ erreur |
| `correlation.rs` | preuve 7.1 avant/après, absence de `status`/`verdict`, artefact + sha256, énoncé hors fenêtre, sujet Page à défaut |
| `tests/bout_en_bout.rs` | **acceptation #437 simulée** : bouton « menu » sans `aria-expanded`, parole avant et après Entrée, actions horodatées, preuve obtenue **via le client sidecar de `rgaa-analysis`**, découverte par `$RGAA_PLUGINS_DIR`, fichier absent, NVDA non détecté, environnement transmis à la déclaration |
| `rgaa-report/declaration.rs` | environnement ajouté au rendu `render_declaration_fr`, déduplication avec une déclaration manuelle, entrée absente/mal formée ignorée, ordre déterministe |

Commandes : `cargo test -p rgaa-sidecar-screenreader` et `cargo test -p rgaa-report`.

## 8. Critères d'acceptation et checklist HITL

**Acceptation simulée (verte sur cette machine).** La fixture bouton « menu » sans `aria-expanded` produit une preuve `7.1` dont `avant` = « menu, bouton, pas coché » et `apres` = « menu, bouton, coché », horodatés à la réception, avec artefact `parole.jsonl` haché, et l'environnement `NVDA 2026.3.1 + Firefox 141.0` apparaît dans la déclaration.

**Validation sur un vrai NVDA — HITL, non exécutée ici.** Un humain doit, dans une VM Windows du cabinet :

1. installer NVDA (version exacte notée) + Firefox (version exacte notée) ; installer l'add-on **Speech Logger** depuis son dépôt upstream, configuré en journalisation « Always » au démarrage et séparateur à deux espaces ;
2. configurer le fichier de journal dans un **dossier partagé hôte ↔ VM** (variables `%temp%` / `%userprofile%` de l'add-on) ; garder la session ouverte (le mode sécurisé désactive les add-ons [S1]) ;
3. démarrer le parcours, placer le focus sur le bouton « menu », appuyer sur Entrée ; vérifier dans le journal que les deux énoncés attendus y sont bien écrits ;
4. déclarer les versions exactes dans `options.environnement` (`"NVDA <version> + Firefox <version>"`) et lancer `rgaa-sidecar-screenreader` via `$RGAA_PLUGINS_DIR` ; vérifier que `rgaa plugins list` affiche `screenreader-nvda` ;
5. confirmer que la preuve 7.1 porte `portee: "Mesuree"`, `horodatage_type: "reception"`, aucun `status`/`verdict`, et que la déclaration d'accessibilité cite l'environnement utilisé ;
6. constater que la décision de conformité reste prise par le relecteur humain (R3).

**Le ticket #437 reste ouvert** jusqu'à cette validation. Le mode « No speech » [S9] permet de se passer de carte son pour la capture locale ; le remote intégré peut ne plus relayer la parole sans périphérique audio [S16].

## 9. Installation (à faire côté VM, non testée ici)

- `cargo install --path rgaa-rs/crates/rgaa-sidecar-screenreader` sur l'hôte, puis copier `sidecar.toml` dans `$RGAA_PLUGINS_DIR` (ou `~/.config/rgaa/plugins`) : `rgaa plugins list` affiche alors la commande et l'empreinte SHA-256 du binaire, comme pour tout sidecar P-00.
- Dans la VM Windows : NVDA + Firefox + add-on Speech Logger « Always », journal dans le dossier partagé. Le sidecar sur l'hôte lit **un chemin**, jamais le réseau (R-P3). Aucun code GPL n'est lié dans nos binaires : l'add-on reste un composant externe installé à part.

Ces étapes n'ont **pas** été exécutées : NVDA ne tourne pas sur la machine de développement, et aucune preuve RGAA n'est produite ici à partir d'un lecteur d'écran réel.

## 10. Hors périmètre

- **P-10.b** : add-on NVDA `pre_speechQueued` (≥ 2025.1 [S8]) pour un horodatage par énoncé (`horodatage_type: "source"`).
- **P-10.c** : VoiceOver, panneau de sous-titres + AXAPI, preuves `Estimee` [S17][S21].
- **P-10.d** : `screenreader-orca` sur le JSONL d'Orca 51+ comme harnais CI uniquement [S26][S31].
- Aucune automatisation de décision de revue, aucun verdict (R-P1/R3).
