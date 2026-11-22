# Recherche — sidecar lecteur d'écran (ticket #437, P-10)

> **Statut** : recherche terminée, **pas d'implémentation**, pas de PR. · **Date** : 2026-11-22.
> **Périmètre** : comment capter *par programme* le texte prononcé par NVDA (Windows), VoiceOver (macOS), et l'option Linux Orca + speech-dispatcher ; protocole conforme à [P-00 §7 bis](../specs/plugins-analyse/00-contrat-plugin.md) ; faisabilité en VM Windows de cabinet.
> **Méthode** : sources **primaires uniquement** — documentation et code de NV Access, Apple, GNOME et Brailcom (speech-dispatcher), plus la méthode RGAA officielle. Chaque fait porte un marqueur `[S__]` renvoyant à la section [Sources](#sources) (URL exacte + date de consultation, toutes au 2026-11-22). **Aucun lecteur d'écran n'a été exécuté** : NVDA et VoiceOver ne tournent pas sur cette machine Linux ; les exemples de sortie sont soit des formats observés dans le code source, soit des propositions explicitement signalées.

## 0. Synthèse et recommandation

| Voie | Plateforme | Fiabilité de la capture | Horodatage | Licence | Version minimale | Verdict |
|---|---|---|---|---|---|---|
| **Speech Viewer** (fourni avec NVDA) | Windows | Moyenne : texte complet mais fenêtre graphique, arrêt de mise à jour au survol/focus [S1][S2] | **Aucun** par énoncé (demande ouverte #16327) [S3] | GPL v2+ (NVDA) [S15] | NVDA 2019.3.1 au moins (présent dans les versions vérifiées) [S2] | Écarable comme source unique ; utile comme témoin visuel |
| **Add-on Speech Logger** (`speechLogger`) | Windows | **Élevée** pour le texte : écrit chaque séquence dans un fichier UTF-8, configurable au démarrage [S4][S5] | ** début/fin de session seulement** (`%x`/`%X`) [S5] | GPL-2.0 [S4] | **NVDA 2019.3.1 et plus** [S4] | **Route recommandée pour un premier sidecar NVDA** |
| **Points d'extension NVDA** (`pre_speech`, `pre_speechQueued`, `filter_speechSequence`) | Windows | **Élevée** : séquence de parole complète avant synthèse, dans le processus NVDA [S6][S7] | À ajouter par notre add-on (le point d'extension ne fournit pas d'horodatage) [S7] | GPL v2+ (code chargé dans NVDA) [S15] | `filter_speechSequence` ≥ 2024.1 ; `pre_speech` ≥ 2024.2 ; `pre_speechQueued` ≥ 2025.1 [S8] | Route recommandée en v2, pour un horodatage par énoncé |
| **NVDA Remote / accès distant intégré** | Windows ↔ Windows | Moyenne : la parole est relayée vers la machine contrôleuse, mais dépend d'un relais et d'un add-on ; non testé avec la fonction intégrée [S13][S16] | Hérité de Speech Logger (session seulement) [S4] | GPL-2.0 (add-on) [S13] | Add-on 2.6 documenté ; version NVDA mini non documentée par le dépôt [S13] | Optionnel, jamais source primaire de preuve |
| **nvdaControllerClient** | Windows | **Nul pour la capture** : API *entrante* (notre application → NVDA), aucune fonction de lecture de la parole [S11][S12] | n/a (horodatage de notre appel) | **LGPL-2.1** (DLL distincte) [S11] | API 2.0 ≥ 2024.1 ; API 3.0 ≥ 2026.3 [S11] | Utilisable seulement pour injecter des marqueurs synchronisants |
| **Pilote de synthèse « No speech » / personnalisé** | Windows | Élevée : `speak(speechSequence)` reçoit la séquence juste avant audio ; « No speech » est un pilote factice fourni [S9][S10] | À ajouter par notre pilote | GPL v2+ (add-on lié à NVDA) [S15] | ≥ 2019.3.1 (pilote `silence.py` présent dans les versions vérifiées) [S9] | Bon plan de repli ; API interne, à versionner finement |
| **Panneau de sous-titres VoiceOver** | macOS | Moyenne : affiche exactement ce que VoiceOver prononce, mais c'est une fenêtre graphique [S17] | Aucun [S17] | Logiciel système Apple (propriétaire) ; aucun code lié | Documenté depuis **macOS 11 (Big Sur)** [S17] | Preuve `Estimee` (lecture écran/OCR du panneau) |
| **Dernière phrase (VO-Shift-C) / archive (VO-Shift-Z)** | macOS | Faible : une seule phrase à la fois, commande interactive ; dossier de diagnostic, pas un flux continu [S18][S19] | Aucun [S18] | Idem | Documenté depuis macOS 11 [S18][S19] | Dépannage uniquement |
| **AXAPI (`announcementRequested`)** | macOS | Partielle : ne capture que les annonces *postées par les applications*, pas la restitution VoiceOver complète (focus, navigation) [S21][S22] | Aucun dans la notification [S21] | API système Apple | **macOS 10.7+** [S21] | Complément pour 7.5, pas pour 7.1 |
| **AppleScript / « System Events »** | macOS | Faible : pilotage de l'UI et du presse-papiers, aucune API documentée de flux vocal [S18][S19] | Aucun | n/a (script local) | macOS 11+ (documentation consultée) [S18] | Dernier recours, non opposable |
| **Orca + JSONL (≥ 51) / speech-dispatcher** | Linux | Élevée *techniquement* : un enregistreur JSONL par énoncé existe dans Orca 51 [S26][S27] | Aucun champ `horodatage` dans l'enregistrement ; le sidecar doit le poser à la lecture [S26] | Orca **LGPL-2.1** [S32] ; speech-dispatcher **GPL-2.0** [S35] | **Orca 51.alpha** pour le JSONL ; moniteur vocal graphique dès 50.x [S26][S30] | **Harnais CI uniquement**, jamais une preuve RGAA de référence [S36] |

**Recommandation.** Commencer par `screenreader-nvda` branché sur **Speech Logger** (fichier, GPL-2.0, sidecar uniquement), ajouter en v2 un add-on de point d'extension pour obtenir un vrai horodatage par énoncé, puis traiter VoiceOver en preuve `Estimee` (panneau de sous-titres + AXAPI). Garder Orca comme **banc de test CI** du protocole, pas comme substitut des environnements de référence RGAA.

---

## 1. NVDA (Windows)

### 1.1 Visionneuse de parole (Speech Viewer) — fournie avec NVDA

La visionneuse est une fenêtre flottante « qui permet de voir tout le texte que NVDA prononce actuellement » ; elle s'active depuis le menu Outils, avec une case « Show speech viewer on startup » [S1]. Le code confirme le mécanisme : `speech.speak()` appelle `speechViewer.appendSpeechSequence(speechSequence)` lorsque la visionneuse est active, et `appendSpeechSequence` concatène les éléments **texte** de la séquence avec deux espaces, puis ajoute un retour à la ligne par séquence [S2]. La mise à jour s'arrête lorsque la fenêtre a le focus ou est survolée, pour permettre la sélection [S1][S2].

- **Fiabilité** : moyenne. Le texte est exact (c'est la source NVDA elle-même), mais la sortie est une fenêtre graphique, sans API documentée pour la lire ; les bords de séquence sont ceux de NVDA (`"  "` / `"\n"`), pas nos actions [S2].
- **Horodatage** : aucun. La demande d'horodatage de la visionneuse est toujours ouverte (#16327, 2024-03-20) [S3].
- **Licence / version** : NVDA est sous **GPL v2 ou ultérieur, avec deux exceptions** [S15] ; la visionneuse fait partie du produit [S1][S2]. Présente dans toutes les versions vérifiées depuis 2019.3.1 [S2]. Conséquence : **sidecar uniquement**, aucun code NVDA ne doit être lié dans nos binaires Rust.
- **Exemple de sortie réel (format issu du code [S2])** :

```text
recycle bin  1 of 55
Menu   sous-menu Ouvrir
```

### 1.2 Add-on Speech Logger — route recommandée

L'add-on `speechLogger` (Luke Davis, contributions James Scholes) « enregistre la parole dans un fichier ou des fichiers », localement et/ou depuis NVDA Remote ; compatibilité annoncée **NVDA 2019.3.1 et plus**, licence **GPL-2.0** [S4]. Le répertoire, le nom de fichier local, le séparateur d'énoncés (deux espaces par défaut, newline, tab, personnalisable), le mode horodatage et le démarrage automatique au lancement sont configurables [S4]. Le code confirme qu'il s'abonne au point d'extension `pre_speech` quand il existe et écrit chaque séquence avec `open(file, "a+", encoding="utf-8")` [S5].

- **Fiabilité** : élevée pour le texte (fichier UTF-8, un enregistrement par séquence, séparateur configurable) [S4][S5]. L'add-on peut démarrer la journalisation « Always » au lancement de NVDA [S4], ce qui supprime une dépendance à un geste clavier dans une VM.
- **Horodatage** : **faible**. Le mode timestamp insère `### Log started on <date> at <heure> ###` / `### Log ended ... ###` au début et à la fin de la session (`strftime("%x")`, `strftime("%X")`) [S5]. Il n'y a **pas** d'horodatage par énoncé (confirmé par le fil de discussion #16327 : « It time-stamps by default (not each line of speech, though that's coming) » [S3]). Le sidecar devra donc horodater à la réception et le déclarer explicitement (`horodatage_type: "reception"`).
- **Licence / version** : GPL-2.0 [S4], NVDA ≥ 2019.3.1 [S4]. Sidecar uniquement.
- **Exemple de sortie (format observé dans le code [S5], séparateur par défaut [S4])** :

```text
### Log started on 11/22/26 at 09:14:02 ###
recycle bin  1 of 55
Menu   sous-menu Ouvrir
### Log ended on 11/22/26 at 09:14:31 ###
```

### 1.3 Points d'extension de parole — route recommandée en v2

Le guide développeur NVDA documente cinq familles de points d'extension et, pour la parole : `speechCanceled`, `pre_speech`, `post_speechPaused`, `pre_speechQueued`, `filter_speechSequence`, côté `synthDriverHandler` : `synthIndexReached`, `synthDoneSpeaking`, `synthChanged`, `pre_synthSpeak` [S6]. Le code précise les paramètres : `pre_speech` reçoit `speechSequence` (séquence filtrée), `originalSpeechSequence`, `symbolLevel`, `priority` ; `pre_speechQueued` reçoit la séquence *entièrement traitée* juste avant sa mise en file ; `filter_speechSequence` est un filtre modifiable [S7]. `speech.speak()` notifie `pre_speech` après avoir éventuellement alimenté la visionneuse [S2].

- **Fiabilité** : élevée — on observe la séquence NVDA elle-même, avant la synthèse audio. Un add-on global peut s'enregistrer et écrire un JSONL avec `time.time_ns()` à chaque notification ; c'est exactement le mécanisme qu'utilise Speech Logger [S5].
- **Horodatage** : absent du point d'extension [S7] ; à produire par notre add-on. C'est le gain attendu de cette voie.
- **Licence / versions** : code chargé dans NVDA ⇒ GPL v2+ [S15]. Versions minimales vérifiées dans les tags : `filter_speechSequence` présent dans `release-2024.1`, `pre_speech` dans `release-2024.2`, `pre_speechQueued` dans `release-2025.1` [S8].
- **Exemple de sortie (proposition, pas un format NVDA)** :

```json
{"ts":"2026-11-22T09:14:02.481Z","point":"pre_speechQueued","sequence":["Menu","sous-menu Ouvrir"],"priorite":"NORMAL"}
```

### 1.4 NVDA Remote et accès distant intégré

L'add-on **NVDA Remote Access 2.6** permet de contrôler un autre ordinateur NVDA ; le readme documente explicitement le « Muting Speech on the Remote Computer » : la parole de la machine contrôlée est entendue sur la machine contrôleuse [S13]. Le dépôt est sous **GPL-2.0** [S13]. NVDA intègre aussi un « Remote Access » natif depuis 2025.x, désactivé par défaut et activable dans les réglages [S1]. Speech Logger sait journaliser la parole distante reçue via l'add-on NVDA Remote, mais prévient que cela n'a **pas été testé avec la fonction intégrée** (juin 2025) et qu'il faut démarrer la journalisation *après* le début de la session [S4]. Un ticket NVDA (#18544) montre qu'avec la fonction intégrée, supprimer tous les périphériques audio empêche la parole distante d'être transmise [S16].

- **Fiabilité** : moyenne ; dépend d'un relais/TLS et d'un add-on, avec un mode intégré encore perfectible [S13][S16].
- **Horodatage** : uniquement celui de Speech Logger (session) [S4][S5].
- **Licence / version** : GPL-2.0 [S13] ; add-on 2.6 documenté, version NVDA minimale non précisée par le dépôt [S13]. Sidecar uniquement.
- **Exemple de sortie** : le fichier « remote » de Speech Logger, même format que §1.2 [S4][S5].

### 1.5 API `nvdaControllerClient` — utile seulement pour synchroniser

La documentation officielle de l'API décrit une **DLL appelée par une application externe pour faire parler NVDA ou envoyer un message en braille** (`nvdaController_speakText`, `nvdaController_brailleMessage`, `nvdaController_speakSsml`, `nvdaController_testIfRunning`, `nvdaController_isSpeaking`) ; aucune fonction ne restitue ce que NVDA prononce [S11]. L'exemple Python fourni charge `nvdaControllerClient.dll`, teste si NVDA tourne, puis parle et braillle [S12]. L'API est sous **LGPL-2.1**, donc distribuable avec notre application si la DLL n'est pas modifiée [S11]. Versions : API 2.0 depuis **NVDA 2024.1** (`getProcessId`, `speakSsml`), API 3.0 depuis **NVDA 2026.3** (`isSpeaking`) [S11].

- **Fiabilité pour la capture** : nulle ; cette voie sert uniquement à injecter un marqueur sonore connu (« test client ») et à synchroniser nos fenêtres d'observation [S11][S12].
- **Exemple de sortie (exemple officiel [S12])** :

```text
This is a test client for NVDA
Time: 0.75 seconds
```

### 1.6 Pilote de synthèse « No speech » ou personnalisé

NVDA fournit un pilote factice `silence`, nom affiché « No speech », dont `speak(speechSequence)` ne fait que mémoriser le dernier index ; il est présent dans les versions vérifiées depuis 2019.3.1 [S9]. L'interface `SynthDriver` exige `speak(speechSequence)` et `cancel()` [S10]. Un pilote personnalisé installé comme add-on reçoit donc la séquence finale et peut l'écrire dans un fichier avant de ne rien dire — ce qui permet de se passer de carte audio, configuration utile en VM (§5).

- **Fiabilité** : élevée (dernier point avant l'audio) ; **horodatage** à ajouter par notre pilote.
- **Licence / version** : GPL v2+ [S15] ; API de pilote stable depuis au moins 2019.3.1 [S9][S10]. Sidecar uniquement.
- **Exemple de sortie (proposition)** :

```json
{"ts":"2026-11-22T09:14:02.481Z","utterance":"Menu sous-menu Ouvrir"}
```

### 1.7 Journal NVDA

Le journal NVDA (`%temp%\nvda.log`, rotation vers `nvda-old.log`) est un journal de diagnostic consultable via NVDA+F1 [S1]. Ce n'est **pas** une transcription de parole : les extraits de parole n'y sont pas garantis (la fonctionnalité dédiée est Speech Logger/Speech Viewer) [S1][S4]. À ne pas utiliser comme source de preuve.

---

## 2. VoiceOver (macOS)

Apple ne documente **aucune API qui expose le flux de parole de VoiceOver** : les pages officielles décrivent des *commandes utilisateur* et des *notifications émises par les applications vers VoiceOver*, mais pas de lecture de la restitution [S17][S18][S21]. Toutes les voies ci-dessous sont donc soit visuelles, soit partielles, soit indirectes ; elles produisent des preuves `Estimee`.

### 2.1 Panneau de sous-titres (caption panel)

Le guide VoiceOver indique : « The caption panel shows what VoiceOver is currently speaking » ; activation dans VoiceOver Utility › Visuals › Panels and Menus, ou `VO-Fn-Command-F10` [S17]. Commandes associées dans la liste générale : `VO-Fn-Command-F10` (afficher/masquer), `VO-Fn-Command-F11` (panneaux), `VO-Shift-Fn-F10` (redimensionner/déplacer) [S19].

- **Fiabilité** : moyenne — texte fidèle mais rendu dans une fenêtre ; il faut une capture d'écran + OCR, avec risques de rafraîchissement et de séquences courtes [S17].
- **Horodatage** : aucun [S17]. Le sidecar devra horodater à la capture d'écran.
- **Licence / version** : fonctionnalité système Apple ; page disponible depuis macOS 11 [S17]. Aucun code tiers à licencier.
- **Exemple de sortie attendue** : `Menu, bouton` (texte affiché, non récupérable autrement que visuellement) [S17].

### 2.2 Dernière phrase et fichiers de diagnostic

`VO-Z` répète la dernière phrase, `VO-Shift-C` la copie dans le presse-papiers, `VO-Shift-Z` l'enregistre comme fichier audio dans un dossier d'archives sur le bureau, « qui contient aussi des fichiers journaux pouvant aider à dépanner » [S18][S19].

- **Fiabilité** : faible — une phrase à la fois, commande interactive, fichiers de diagnostic non contractuels [S18][S19].
- **Horodatage** : aucun [S18].
- **Version** : documenté depuis macOS 11 [S18][S19].
- **Exemple de sortie (presse-papiers après VO-Shift-C)** : `Menu, bouton` [S18].

### 2.3 AXAPI — annonces d'applications

`NSAccessibility.Notification.announcementRequested` est publiée par une **application** qui veut faire une annonce : « If VoiceOver is enabled, it's presented via speech and/or braille. Otherwise, it does nothing » ; elle exige un dictionnaire `userInfo` avec la clé `announcement` et une chaîne localisée, avec une priorité optionnelle [S21]. La clé bas niveau correspondante est `kAXAnnouncementKey`, « Notification info key used to specify an announcement to be spoken with a notification » [S22]. Apple recommande ce mécanisme pour les événements qui ne mettent pas l'interface à jour [S21]. VoiceOver annonce aussi les changements de statut/progression (ex. « Installation 45 per cent completed »), configurables dans l'onglet Announcements [S23][S24], et conserve jusqu'à 30 annonces dans un historique consultable en braille [S20].

- **Fiabilité** : partielle et asymétrique — on observe les annonces *postées par les applications* (utile pour 7.5), **pas** les énoncés générés par VoiceOver lui-même lors d'un parcours Tab/flèches (utile pour 7.1) [S21][S22][S20].
- **Horodatage** : aucun dans la notification ; à poser à la réception [S21].
- **Licence / version** : API système Apple, **macOS 10.7+** [S21][S22].
- **Exemple de sortie (exemple officiel [S21])** :

```swift
let announcement = "The input was invalid."
NSAccessibility.post(element: NSApp.mainWindow as Any,
                     notification: .announcementRequested,
                     userInfo: [.announcement: announcement,
                                .priority: NSAccessibilityPriorityLevel.medium.rawValue])
```

### 2.4 AppleScript / journal — ce qui est réellement faisable

Les commandes VoiceOver sont documentées comme des combinaisons de touches, pas comme un dictionnaire AppleScript [S18][S19]. Un script AppleScript peut donc, **avec l'autorisation d'accessibilité macOS**, envoyer `VO-Shift-C` via « System Events » puis lire le presse-papiers ; c'est une automatisation de la commande utilisateur, pas une API de restitution [S18]. Le « journal » évoqué par `VO-Shift-Z` est un dossier de dépannage, pas un flux horodaté exploitable [S18].

- **Fiabilité** : faible (une phrase, dépend du focus et des permissions) ; **horodatage** : à la réception uniquement [S18].
- **Version** : macOS 11+ pour les pages consultées [S18][S19].
- **Exemple de sortie** : `tell application "System Events" ...` puis contenu du presse-papiers `Menu, bouton` [S18] — proposition d'automatisation, pas un format documenté par Apple.

---

## 3. Option Linux : Orca + speech-dispatcher

### 3.1 Ce qu'Orca sait déjà faire (et depuis quelle version)

Orca est un lecteur d'écran libre pour AT-SPI, sous **LGPL-2.1** [S32]. Ses options de lancement documentées incluent `--replace`, `--debug`, `--debug-file=FILE` et `--speech-system NAME` (`speechdispatcherfactory`, parce que c'est le système vocal par défaut, ou `spiel`) [S25]. Deux mécanismes natifs sont exploitables :

1. **Moniteur vocal graphique** (`speech_monitor.py`) : une fenêtre GTK qui affiche un « scrolling log of spoken text » ; présent déjà dans **Orca 50.3** [S30]. C'est l'équivalent fonctionnel du Speech Viewer, donc visuel et sans horodatage.
2. **Enregistreur JSONL** (`output_recorder.py`) : « Writes JSONL records to a file so external observers can consume presenter output », ouvert en mode `"w"` à bufferisation ligne par ligne [S26]. Présent à partir d'**Orca 51.alpha** (vérifié aux tags 51.alpha, 51.0, absent en 50.3) [S26]. `speech_presenter.py` y écrit un enregistrement **par énoncé** avec les champs `kind="speech"`, `text`, `language`, `dialect`, `voice_type`, `punctuation`, plus un marqueur `kind="interrupt"` [S27]. Un lecteur `output_reader.py` parse ces lignes en `SpeechRecord` et gère les interruptions [S29]. L'activation passe par une commande D-Bus **de test** `SetLogFileForTesting`, publiée uniquement quand Orca démarre avec `ORCA_TEST_RPC_SECRET` dans l'environnement, le secret étant exigé en premier argument [S28][S27].

- **Fiabilité** : élevée pour un harnais — texte exact, un enregistrement par énoncé, interruption explicite, et harnais de test officiel qui pilote cette commande [S26][S27][S29].
- **Horodatage** : **aucun champ `horodatage`** dans l'enregistrement [S26][S27]. Le sidecar Orca doit horodater à la lecture et le marquer (`horodatage_type: "reception"`). Les tests Orca utilisent des fenêtres de quiescence (`drain(quiescence_timeout, overall_timeout, first_record_timeout)`), pas des timestamps [S29].
- **Exemple de sortie (format observé dans le code [S26][S27])** :

```json
{"kind":"speech","text":"Menu  sous-menu Ouvrir","language":"fr","dialect":"","voice_type":"","punctuation":""}
{"kind":"interrupt"}
```

### 3.2 speech-dispatcher rend la chose testable en CI

Le manuel de speech-dispatcher précise que si aucun module de sortie n'est chargé, le serveur démarre, « log messages into history and communicate with clients, but no sound is produced » [S34]. Le fichier de configuration documente `LogDir "stdout"`, `CustomLogFile "protocol" "…"` pour journaliser tous les messages quelle que soit leur priorité, et un module `testing` qui « outputs the requested commands to standard output » [S33]. Le dépôt indique que les modules sont reliés au serveur par de simples tubes avec un protocole très simple, et que le projet est sous **GPL-2.0** [S35].

Il est donc possible, sur une CI Linux, de lancer Orca avec `speechdispatcherfactory`, un module de sortie muet/`testing`, un serveur X virtuel et AT-SPI, puis de lire le JSONL d'Orca sans carte son. Les tests d'intégration officiels d'Orca documentent précisément ces dépendances : Xvfb, `dbus-run-session`, PyGObject, AT-SPI (`at-spi-bus-launcher`, `at-spi2-registryd`), et la nécessité de variables comme `ACCESSIBILITY_ENABLED=1` (Chromium) ou `GNOME_ACCESSIBILITY=1` (Firefox) pour que les navigateurs apparaissent dans le registre AT-SPI [S31].

**Limite importante** : le harnais Orca « ne peut pas tourner pendant qu'une instance d'Orca est déjà active dans la session » ; chaque test lance et pilote sa propre instance [S31]. À prévoir comme contrainte d'isolement dans notre CI.

### 3.3 Que cela vaut au regard de la méthode RGAA

La méthode RGAA 4.1.2 impose, pour les tests de restitution, des combinaisons **technologie d'assistance + navigateur** : en desktop, `NVDA (dernière version) + Firefox`, `JAWS (version précédente) + Firefox ou Internet Explorer`, `VoiceOver (dernière version) + Safari` ; la version de l'AT doit être la dernière disponible en français ou la précédente [S36]. **Orca n'apparaît dans aucune combinaison de référence** [S36]. Le guide précise que ces combinaisons servent à déclarer un dispositif « compatible avec l'accessibilité » au sens WCAG, et qu'un environnement maîtrisé peut imposer sa propre base de référence [S36].

Conséquence pour notre produit : le sidecar Orca peut valider le **protocole**, l'horodatage et la corrélation action → parole dans la CI ; il ne peut **pas** produire une preuve de conformité RGAA sur une combinaison de référence. Les preuves Orca doivent rester `Estimee`/appui à la revue, et la déclaration d'environnement (#430) doit continuer à nommer explicitement `NVDA x.y + Firefox z` ou `VoiceOver x.y + Safari z` [S36].

---

## 4. Proposition de protocole sidecar (conforme P-00 §7 bis)

Le contrat P-00 §7 bis définit un manifeste `plugins/<id>/sidecar.toml`, des échanges JSON-RPC 2.0 ligne par ligne sur stdin/stdout, les méthodes `initialiser` / `analyser` / `journal` / `arreter`, le passage des fichiers lourds par **chemins** dans le répertoire temporaire de l'audit, le rejet de tout champ `status`/`verdict` (R-P1) et le passage en `review` « preuve manquante » en cas de timeout ou JSON invalide [S37]. La proposition ci-dessous s'y conforme exactement.

### 4.1 Manifeste

```toml
# plugins/screenreader-nvda/sidecar.toml — cherché dans $RGAA_PLUGINS_DIR puis ~/.config/rgaa/plugins
id = "screenreader-nvda"
version = "0.1.0"
tests = ["7.1.1", "7.1.2", "7.1.3", "7.5.1", "7.5.2", "7.5.3"]
commande = ["python3", "-I", "sidecar.py"]
mode = "persistant"
timeout_ms = 30000
binaires_requis = [{ nom = "nvda", version_min = "2024.2", verif = ["nvda", "--version"] }]
licence = "GPL-2.0 (add-on Speech Logger) ; sidecar seul, aucun code NVDA lié"
```

### 4.2 Échanges JSONL

```jsonc
// rgaa → sidecar
{"jsonrpc":"2.0","id":1,"method":"initialiser","params":{"protocole":1,"options":{"source":"speech-logger","fuseau":"UTC"}}}
// sidecar → rgaa
{"jsonrpc":"2.0","id":1,"result":{"id":"screenreader-nvda","version":"0.1.0","tests":["7.1.1","7.5.1"],"version_modele":null}}
```

`analyser` reçoit l'URL, les sujets et les **chemins** des fichiers de parole et des actions à corréler [S37] :

```jsonc
{"jsonrpc":"2.0","id":2,"method":"analyser","params":{
  "url":"https://exemple.fr/fixture-menu",
  "sujets":[{"selecteur":"#menu-button","xpath":"/html/body/button[1]","bbox":{"x":120,"y":210,"width":90,"height":32}}],
  "fichiers":{"parole":"/tmp/audit-123/analysis/nvda/speech.log","actions":"/tmp/audit-123/actions.jsonl"},
  "actions":[
    {"id":"a1","type":"focus","selecteur":"#menu-button","fenetre_ms":1500},
    {"id":"a2","type":"activation","selecteur":"#menu-button","declencheur":"Enter","fenetre_ms":2500}
  ]
}}
```

Le fichier `actions.jsonl` porte les instants des actions (`{"id":"a2","t":"2026-11-22T09:14:02.120Z",...}`) mesurés par le pilote de parcours ; le sidecar associe chaque énoncé à la fenêtre `[t, t + fenetre_ms]`, avec `horodatage_type` explicite.

Le résultat est une `Preuve` P-00 §5, **sans champ de statut** [S37] :

```jsonc
{"jsonrpc":"2.0","id":2,"result":{"preuves":[{
  "plugin":"screenreader-nvda",
  "version_plugin":"0.1.0",
  "version_modele":null,
  "tests":["7.1.1","7.5.1"],
  "sujet":{"Element":{"selecteur":"#menu-button","xpath":"/html/body/button[1]","bbox":{"x":120,"y":210,"width":90,"height":32}}},
  "portee":"Mesuree",
  "mesures":{
    "action":{"id":"a2","type":"activation","declencheur":"Enter"},
    "element":{"selecteur":"#menu-button","nom_accessible_attendu":"menu"},
    "texte_prononce":"menu, bouton, coché",
    "horodatage":"2026-11-22T09:14:02.481Z",
    "horodatage_type":"reception",
    "avant":{"texte_prononce":"menu, bouton, pas coché","horodatage":"2026-11-22T09:14:00.204Z"},
    "apres":{"texte_prononce":"menu, bouton, coché","horodatage":"2026-11-22T09:14:02.481Z"},
    "source":"nvda-speech-logger"
  },
  "indice":{"Suspect":{"raison":"état d'expansion non exposé dans l'AX tree (P-07)"}},
  "artefacts":[{"nom":"parole.jsonl","media_type":"application/x-ndjson","chemin_bundle":"analysis/screenreader-nvda/<sha256>.jsonl","sha256":"…"}],
  "empreinte_entree":"sha256:…"
}]}}
```

`journal` relaie `{"niveau","message"}` vers `tracing` ; `arreter` ferme le fichier et tue le processus [S37].

### 4.3 Règles de corrélation et de portée

1. **Quatre champs du ticket** : `action`, `element`, `texte_prononce`, `horodatage` sont portés par `mesures` ; les champs `avant`/`apres` répondent à l'acceptation 7.1 de #437.
2. **Horodatage honnête** : `horodatage_type = "source"` uniquement pour un add-on de point d'extension ou un pilote qui horodate lui-même (§1.3, §1.6) ; `"reception"` pour Speech Logger (§1.2), le panneau VoiceOver (§2.1) et Orca (§3.1), qui n'ont pas d'horodatage par énoncé [S5][S17][S26].
3. **Portée** : `Mesuree` pour un texte lu dans un fichier produit par NVDA/Orca ; `Estimee` pour une lecture d'écran du panneau VoiceOver ou du moniteur vocal [S37 et R-P2]. Dans tous les cas, 7.1/7.5 restent en `review` : la preuve n'est pas un verdict (R-P1) [S37].
4. **Échec** : timeout, sortie non nulle ou JSON invalide ⇒ test en `review` « preuve manquante (sidecar screenreader-nvda : <raison>) », audit poursuivi [S37]. Un champ `verdict`/`status` dans la réponse ⇒ preuve rejetée [S37].
5. **Variantes** : `screenreader-orca` (même code de protocole, source JSONL 51+) sert au banc CI ; `screenreader-voiceover` produit des preuves `Estimee`. Le manifeste et `version_plugin` identifient la variante, comme le prévoit P-00 §2 [S37].

---

## 5. Faisabilité dans une VM Windows de cabinet

**Verdict : faisable**, sans exécution ici, sous réserve des points ci-dessous.

| Point | Ce que disent les sources | Conséquence pratique |
|---|---|---|
| Système | NVDA 2026.x demande Windows 10/11 64 bits (ou Server 2016+), ≥ 500 Mo de stockage, 4 Go de RAM recommandés [S1] | VM Windows 10/11 64 bits, 4 Go RAM, disque suffisant |
| Installation | Copie portable possible, ou installation silencieuse (`--install-silent`) [S1] | Snapshot de VM avec NVDA + Firefox + add-on préinstallés ; la copie portable facilite le clonage |
| Audio | Le pilote « No speech » (`silence`) désactive la synthèse [S9] ; avec Speech Logger, plus besoin d'entendre la sortie [S4] | Pas de carte son virtuelle nécessaire pour la capture locale. **Attention** : le *remote intégré* peut ne plus relayer la parole si tous les périphériques audio sont absents [S16] |
| Écran verrouillé / secure desktop | Le mode sécurisé désactive Speech Viewer, le journal, les add-ons et la journalisation [S1] | Garder la session ouverte ; ne pas tester sur écran de connexion/UAC |
| Sortie de données | Speech Logger écrit dans un répertoire configurable, avec variables `%temp%`, `%userprofile%` [S4] | Écrire le journal dans un dossier partagé hôte ↔ VM ; le sidecar sur l'hôte lit le chemin, jamais de réseau [S37] |
| Licence | NVDA GPL v2+ [S15], Speech Logger GPL-2.0 [S4], NVDA Remote GPL-2.0 [S13] | Distribuer les add-ons **à part**, jamais lier leur code dans `rgaa-*` ; le sidecar reste un processus externe [S37] |
| Automatisation | NVDA expose raccourcis et points d'extension [S1][S6] ; le parcours clavier est piloté par notre infrastructure | Utiliser Speech Logger « Always » au démarrage [S4] pour éviter un geste de démarrage fragile |

**Risques résiduels à valider lors de l'implémentation** : dérive des versions NVDA/Firefox entre snapshots (d'où l'obligation d'enregistrer la version exacte dans la déclaration #430 [S36]), séquences coupées par une interruption (Orca expose un marqueur explicite [S27], NVDA pas encore), et absence d'horodatage par énoncé dans la voie la plus simple [S3][S5].

---

## 6. Décisions proposées pour la spec P-10 (à implémenter dans un ticket séparé)

1. **P-10.a — NVDA d'abord** : `screenreader-nvda` + Speech Logger en source par défaut ; add-on GPL livré à part ; preuve `Mesuree`, `horodatage_type: "reception"`.
2. **P-10.b — NVDA v2** : add-on de point d'extension `pre_speechQueued` (NVDA ≥ 2025.1 [S8]) pour un horodatage par énoncé.
3. **P-10.c — VoiceOver** : panneau de sous-titres + AXAPI pour 7.5, preuve `Estimee`, jamais verdict [S17][S21].
4. **P-10.d — Orca CI** : `screenreader-orca` branché sur le JSONL d'Orca 51+, Xvfb + AT-SPI + speech-dispatcher muet [S26][S31][S33] ; usage de harnais uniquement [S36].
5. **Le ticket #437 reste ouvert** : cette recherche ne livre pas l'implémentation ; l'acceptation (fixture bouton « menu » sans `aria-expanded`, parole avant/après Entrée, environnement ajouté à #430) reste à traiter.

---

## Sources

Toutes les URL ci-dessous ont été consultées le **2026-11-22**. Sources externes : documentation/code des éditeurs ; `S37` est la référence interne du dépôt.

| # | Source | URL | Date |
|---|---|---|---|
| S1 | NVDA 2026.3.1 User Guide (Speech Viewer, « No speech », mode sécurisé, configuration système, Remote Access, journal) et NVDA 2025.1 User Guide (accès distant intégré) | https://download.nvaccess.org/releases/2026.3.1/documentation/userGuide.html · https://download.nvaccess.org/releases/2025.1/documentation/userGuide.html | 2026-11-22 |
| S2 | NVDA — `source/speechViewer.py` (`SPEECH_ITEM_SEPARATOR`, `appendSpeechSequence`) ; présent aussi au tag `release-2019.3.1` | https://github.com/nvaccess/nvda/blob/master/source/speechViewer.py · https://github.com/nvaccess/nvda/blob/release-2019.3.1/source/speechViewer.py | 2026-11-22 |
| S3 | NVDA issue #16327 — horodatage de la visionneuse, remarque sur Speech Logger | https://github.com/nvaccess/nvda/issues/16327 | 2026-11-22 |
| S4 | Speech Logger add-on — readme (compatibilité ≥ 2019.3.1, options, timestamps, remote) | https://github.com/opensourcesys/speechLogger/blob/master/readme.md | 2026-11-22 |
| S5 | Speech Logger add-on — `addon/globalPlugins/speechLogger/__init__.py` (point d'extension, `%x`/`%X`, écriture fichier) | https://github.com/opensourcesys/speechLogger/blob/master/addon/globalPlugins/speechLogger/__init__.py | 2026-11-22 |
| S6 | NVDA Developer Guide — extension points (speech, synthDriverHandler) | https://github.com/nvaccess/nvda/blob/master/projectDocs/dev/developerGuide/developerGuide.md | 2026-11-22 |
| S7 | NVDA — `source/speech/extensions.py` (paramètres de `pre_speech`, `pre_speechQueued`, `filter_speechSequence`) | https://github.com/nvaccess/nvda/blob/master/source/speech/extensions.py | 2026-11-22 |
| S8 | NVDA — tags `release-2024.2` et `release-2025.1`, `source/speech/extensions.py` (versions minimales) | https://github.com/nvaccess/nvda/blob/release-2024.2/source/speech/extensions.py · https://github.com/nvaccess/nvda/blob/release-2025.1/source/speech/extensions.py | 2026-11-22 |
| S9 | NVDA — `source/synthDrivers/silence.py` (pilote « No speech ») ; présent aussi au tag `release-2019.3.1` | https://github.com/nvaccess/nvda/blob/master/source/synthDrivers/silence.py · https://github.com/nvaccess/nvda/blob/release-2019.3.1/source/synthDrivers/silence.py | 2026-11-22 |
| S10 | NVDA — `source/synthDriverHandler.py` (`SynthDriver.speak(speechSequence)`) | https://github.com/nvaccess/nvda/blob/master/source/synthDriverHandler.py | 2026-11-22 |
| S11 | NVDA Controller Client API — readme (LGPL-2.1, versions 2.0/3.0, fonctions) | https://github.com/nvaccess/nvda/blob/master/extras/controllerClient/readme.md | 2026-11-22 |
| S12 | NVDA Controller Client — `examples/example_python.py` (exemple de sortie) | https://github.com/nvaccess/nvda/blob/master/extras/controllerClient/examples/example_python.py | 2026-11-22 |
| S13 | NVDA Remote Access add-on — readme (v2.6, parole distante, GPL-2.0) | https://github.com/NVDARemote/NVDARemote/blob/master/readme.md | 2026-11-22 |
| S15 | NVDA License — `copying.txt` (GPL v2+ et exceptions) | https://github.com/nvaccess/nvda/blob/release-2026.3.1/copying.txt | 2026-11-22 |
| S16 | NVDA issue #18544 — parole distante absente quand tous les périphériques audio sont désactivés | https://github.com/nvaccess/nvda/issues/18544 | 2026-11-22 |
| S17 | Apple — « Use the VoiceOver caption panel on Mac » (version macOS 11) | https://support.apple.com/guide/voiceover/unac078/10/mac/11 | 2026-11-22 |
| S18 | Apple — « Use VoiceOver with the last spoken phrase on Mac » (VO-Shift-C, VO-Shift-Z, dossier de journaux) | https://support.apple.com/guide/voiceover/vo2725/10/mac/11 | 2026-11-22 |
| S19 | Apple — « VoiceOver general commands on Mac » (raccourcis panneaux et dernière phrase) | https://support.apple.com/guide/voiceover/cpvokys01/10/mac/11 | 2026-11-22 |
| S20 | Apple — « Use a braille display to review VoiceOver announcements on Mac » (30 annonces conservées) | https://support.apple.com/guide/voiceover/vo15621/10/mac/11 | 2026-11-22 |
| S21 | Apple Developer — `NSAccessibility.Notification.announcementRequested` (macOS 10.7+, `announcement`, exemple) | https://developer.apple.com/documentation/appkit/nsaccessibility-swift.struct/notification/announcementrequested | 2026-11-22 |
| S22 | Apple Developer — `kAXAnnouncementKey` | https://developer.apple.com/documentation/applicationservices/kaxannouncementkey | 2026-11-22 |
| S23 | Apple — « Use VoiceOver to learn about progress or status changes on Mac » | https://support.apple.com/guide/voiceover/vo14110/mac | 2026-11-22 |
| S24 | Apple — « Change VoiceOver Verbosity settings (Announcements tab) » | https://support.apple.com/guide/voiceover/cpvouverbann/mac | 2026-11-22 |
| S25 | GNOME — Orca User Guide, « Introduction / Load-Time Options » (`--speech-system`, `--debug-file`) | https://help.gnome.org/users/orca/stable/introduction.html.en | 2026-11-22 |
| S26 | GNOME Orca 51.0 — `src/orca/output_recorder.py` (JSONL par énoncé) | https://github.com/GNOME/orca/blob/51.0/src/orca/output_recorder.py | 2026-11-22 |
| S27 | GNOME Orca 51.0 — `src/orca/speech_presenter.py` (`SetLogFileForTesting`, `_record_speech`, champs) | https://github.com/GNOME/orca/blob/51.0/src/orca/speech_presenter.py | 2026-11-22 |
| S28 | GNOME Orca 51.0 — `src/orca/dbus_service.py` (`ORCA_TEST_RPC_SECRET`) | https://github.com/GNOME/orca/blob/51.0/src/orca/dbus_service.py | 2026-11-22 |
| S29 | GNOME Orca 51.0 — `src/orca/output_reader.py` (`SpeechRecord`, `drain`, interruptions) | https://github.com/GNOME/orca/blob/51.0/src/orca/output_reader.py | 2026-11-22 |
| S30 | GNOME Orca 50.3 — `src/orca/speech_monitor.py` (moniteur vocal graphique sans JSONL) | https://github.com/GNOME/orca/blob/50.3/src/orca/speech_monitor.py | 2026-11-22 |
| S31 | GNOME Orca — `tests/README.md` (Xvfb, AT-SPI, D-Bus, variables navigateur, isolement des instances) | https://github.com/GNOME/orca/blob/main/tests/README.md | 2026-11-22 |
| S32 | GNOME Orca — licence LGPL-2.1 (en-têtes source et fiche projet GitLab) | https://gitlab.gnome.org/GNOME/orca | 2026-11-22 |
| S33 | Speech Dispatcher — `config/speechd.conf` (`LogDir "stdout"`, `CustomLogFile`, module `testing`) | https://github.com/brailcom/speechd/blob/master/config/speechd.conf | 2026-11-22 |
| S34 | Speech Dispatcher — manuel (aucun module ⇒ pas de son mais messages journalisés) | https://github.com/brailcom/speechd/blob/master/doc/speech-dispatcher.html | 2026-11-22 |
| S35 | Speech Dispatcher — README (GPL-2.0, modules par tubes/protocole simple) | https://github.com/brailcom/speechd/blob/master/README.md | 2026-11-22 |
| S36 | RGAA 4.1.2 — méthode technique §2.4, « Environnement de test (base de référence) » | https://accessibilite.numerique.gouv.fr/methode/environnement-de-test · https://accessibilite.numerique.gouv.fr/doc/RGAA-v4.1.2.pdf | 2026-11-22 |
| S37 | Référence interne — P-00 §7 bis, protocole sidecar et modèle de `Preuve` | `docs/specs/plugins-analyse/00-contrat-plugin.md` (§5 et §7 bis) | 2026-11-22 |
