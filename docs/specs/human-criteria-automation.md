# Addendum — niveau d'automatisation des tests RGAA réputés humains

**Statut** : addendum de spec. **Remplace le §9 « Critères humains par conception »** de
`docs/specs/criteria-coverage-expansion.md`.

**Source** : carte wayfinder [Automatiser au maximum les tests RGAA réputés humains](https://github.com/jamon8888/Holo-RGAA/issues/330),
19 tickets résolus. Cet addendum **consigne** ces décisions ; il n'en prend aucune. Une
contradiction entre tickets remonte en grilling au lieu d'être arbitrée ici.

**Vocabulaire** : `CONTEXT.md` — *Test RGAA*, *Niveau d'automatisation*, *Automatique*,
*Assisté*, *Irréductible*, *Famille de preuve*, *Calibration*. Les trois niveaux ne sont pas
des degrés de qualité : « Irréductible » est une réponse légitime, et chaque occurrence
porte sa justification écrite.

## 1. Périmètre et résultat

**117 tests RGAA 4.1.2**, sur les 258 du référentiel : les 106 de
[l'inventaire](../wayfinder/human-criteria/inventory.md) plus 11 tests inclus par les
grillings (3.2.1–3.2.5, 4.3.1, 4.10.1, 10.7.1, 12.3.2, 12.3.3, 13.7.1).

| Niveau | Tests | Part |
|---|---|---|
| **Automatique** | 16 | 14% |
| **Assisté** | 82 | 70% |
| **Irréductible** | 19 | 16% |

| Groupe | Tests | Automatique | Assisté | Irréductible |
|---|---|---|---|---|
| G1 — règles axe | 8 | 7 | 1 | 0 |
| G2 — pertinence | 27 | 0 | 20 | 7 |
| G3 — multimédia et flashs | 18 | 3 | 10 | 5 |
| G4 — tests visuels | 26 | 1 | 19 | 6 |
| G5 — compréhension et équivalence | 38 | 5 | 32 | 1 |
| **Total** | **117** | **16** | **82** | **19** |

Deux lectures à ne pas manquer :

- **Aucun test de pertinence n'atteint « Automatique »**, et ce n'est pas faute d'avoir
  mesuré. Le meilleur résultat obtenu, sur la seule famille calibrée, est 85 % d'accord
  (kappa 0,68) avec **19 % des mauvaises alternatives validées à tort**. C'est la direction
  dangereuse pour un audit : le `pass` automatique est exclu.
- **Le computer use ne fait monter aucun test d'un niveau.** Il provoque des états et
  enrichit la preuve d'un `review` (7.4, 7.5, 11.10–11.12, 12.11) ; il ne conclut pas.

## 2. Règles transversales

1. **Un test Assisté rend `review`.** Jamais `pass`, jamais `fail`, quelle que soit la
   confiance du modèle. C'est la définition du niveau, pas une précaution.
2. **Aucun `fail` ne provient d'un verdict de modèle.** Mesuré : 8 % de faux
   « non pertinent » sur la famille calibrée. Un `fail` erroné dans un rapport est une
   affirmation fausse opposable au client.
3. **L'auditeur voit la preuve et les conditions structurées, pas de verdict.** Pour la
   famille « alternative d'image » : complétude vis-à-vis de l'image, cohérence avec le
   contexte, chacune motivée. Justification mesurée : le ET mécanique des deux conditions
   égale *exactement* le verdict du modèle (33/39, à la carte près) — le verdict n'ajoute
   aucune information et se trompe presque toujours dans le sens indulgent. Le verdict reste
   calculé **en interne** pour ordonner la file de revue.
4. **La file de revue est ordonnée par le signal du modèle.** C'est là que se récupère le
   temps d'audit, sans rien affirmer.
5. **Calibration par famille de preuve**, cinq familles. Un test est *Assisté calibré*
   quand sa famille l'est, *non calibré* sinon. **Aucune fiabilité n'est annoncée dans un
   rapport pour un test non calibré.** État : 1 famille sur 5 calibrée.
6. **Seuil d'admission d'un modèle**, aucun modèle épinglé : ancres 4/4, accord au-dessus
   du repère trivial avec intervalles de confiance disjoints, **≤ 10 % de faux
   « pertinent »** parmi les alternatives rejetées par l'humain.
7. **API de modèle désactivée par défaut**, activable par audit. Sans activation : preuves
   seules, aucun verdict. Envoi minimal par test (texte < recadrage < pleine page),
   **jamais de page authentifiée** vers une API externe, transparence par test dans le
   rapport.
8. **Trois paliers de lecture des médias** : A (sans lecture du média) et B (≤ 1 Mo par
   plage) actifs par défaut et comptés au rapport ; C (lecture complète) sur activation par
   audit. Médias supprimés après analyse.
9. **Une règle axe expérimentale s'active une par une**, chacune derrière un plancher de
   validation : fixture conforme et fixture non conforme classées en CI. Ordre arrêté :
   5.4.1, 5.7.4, 9.1.3 → 13.9.1 → 12.8.1 → 10.8.1.
10. **Les `incomplete` d'axe deviennent des preuves `review`.** Aujourd'hui `axe.run()` est
    appelé sans options et les jette : **prérequis bloquant** de la montée en 4.14.

## 3. Tableau par test

### G1 — règles axe (8 tests)

| Test | Niveau | Technique | Bibliothèque (licence) | Preuve produite | Statut par défaut |
|---|---|---|---|---|---|
| **5.4.1** | Automatique | règle axe `table-fake-caption` [^5_4_1] | axe-core 4.14.0 (MPL-2.0) | violation axe : sélecteur, extrait HTML, message de la règle | `fail` / `pass` |
| **5.7.4** | Automatique | règle axe `td-has-header` [^5_7_4] | axe-core 4.14.0 (MPL-2.0) | violation axe : sélecteur, extrait HTML, message de la règle | `fail` / `pass` |
| **9.1.3** | Automatique | règle axe `p-as-heading` [^9_1_3] | axe-core 4.14.0 (MPL-2.0) | violation axe : sélecteur, extrait HTML, message de la règle | `fail` / `pass` |
| **13.9.1** | Automatique | règle axe `css-orientation-lock` [^13_9_1] | axe-core 4.14.0 (MPL-2.0) | violation axe + rendu dans les deux orientations (`Emulation.setDeviceMetricsOverride`) | `fail` / `pass` |
| **12.8.1** | Automatique | règle axe `focus-order-semantics` [^12_8_1] | axe-core 4.14.0 (MPL-2.0) | violation axe : sélecteur, `tabindex`, extrait HTML | `fail` / `pass` |
| **10.8.1** | Automatique | règle axe `hidden-content` [^10_8_1] | axe-core 4.14.0 (MPL-2.0) | violation axe : sélecteur, extrait HTML | `fail` / `pass` |
| **7.1.1** | Automatique | règle axe `aria-tab-name` [^7_1_1] | axe-core 4.14.0 (MPL-2.0) | violation axe : sélecteur, nom accessible calculé | `fail` / `pass` |
| **6.1.5** | Assisté | règle axe `label-content-name-mismatch` [^6_1_5] | axe-core 4.14.0 (MPL-2.0) | violation axe : texte visible vs nom accessible | `review` |

[^5_4_1]: expérimentale : activée après plancher (fixture conforme + non conforme en CI)
[^5_7_4]: expérimentale : activée après plancher
[^9_1_3]: expérimentale : activée après plancher
[^13_9_1]: expérimentale : activée après plancher
[^12_8_1]: expérimentale ; rattachement 12.8.1 retenu (étiquette axe) contre 7.3 au registre
[^10_8_1]: expérimentale, dernière de l'ordre d'activation ; aucune étiquette RGAA chez axe
[^7_1_1]: règle par défaut d'axe 4.14, étiquetée RGAA 7.1.1
[^6_1_5]: passée en règle par défaut en 4.14 ; maintenue en `review` jusqu'au plancher ; registre 11.2 vs étiquette axe 6.1.5

### G2 — pertinence (27 tests)

| Test | Niveau | Technique | Bibliothèque (licence) | Preuve produite | Statut par défaut |
|---|---|---|---|---|---|
| **1.3.1**, **1.3.2**, **1.3.3**, **1.3.4**, **1.3.5**, **1.3.6**, **1.3.7**, **1.3.8**, **1.3.9** | Assisté | VLM — famille de preuve « alternative d'image », **calibrée** [^1_3_1] | modèle servi par API (§4) — `Qwen3.8-27B` ou `Holo4-35B-A3B` (Apache-2.0) auto-hébergés, ou passerelle admise | vignette de l'image, extrait du contexte de la page, texte de l'alternative, **complétude vis-à-vis de l'image** (oui/non/sans objet) et **cohérence avec le contexte** (oui/non), chacune motivée — **sans verdict** | `review` |
| **1.4.1**, **1.4.2**, **1.4.3**, **1.4.4**, **1.4.5**, **1.4.6**, **1.4.7** | Irréductible | — [^1_4_1] | — | inventaire : CAPTCHA détecté (`iframe[src*=recaptcha]`, `h-captcha`…) et alternative présente ou non | `review` d'inventaire |
| **1.7.1**, **1.7.2**, **1.7.3**, **1.7.4**, **1.7.5**, **1.7.6** | Assisté | VLM — famille « description détaillée », **non calibrée** [^1_7_1] | modèle servi par API (§4) — `Qwen3.8-27B` ou `Holo4-35B-A3B` (Apache-2.0) auto-hébergés, ou passerelle admise | image, description détaillée (`aria-describedby`, `longdesc`, lien adjacent), conditions structurées — sans verdict | `review` |
| **5.2.1** | Assisté | LLM texte seul — famille « résumé et libellé », **non calibrée** [^5_2_1] | modèle servi par API (§4) — `Qwen3.8-27B` ou `Holo4-35B-A3B` (Apache-2.0) auto-hébergés, ou passerelle admise | structure du tableau (en-têtes, portées), texte du résumé, conditions structurées | `review` |
| **5.5.1** | Assisté | LLM texte seul — famille « résumé et libellé », **non calibrée** | modèle servi par API (§4) — `Qwen3.8-27B` ou `Holo4-35B-A3B` (Apache-2.0) auto-hébergés, ou passerelle admise | titre du tableau (`<caption>` ou `aria-labelledby`), contenu du tableau, conditions structurées | `review` |
| **8.6.1** | Assisté | LLM texte seul — famille « résumé et libellé », **non calibrée** [^8_6_1] | modèle servi par API (§4) — `Qwen3.8-27B` ou `Holo4-35B-A3B` (Apache-2.0) auto-hébergés, ou passerelle admise | titre de page, contenu principal extrait, conditions structurées | `review` |
| **11.7.1** | Assisté | LLM texte seul — famille « résumé et libellé », **non calibrée** | modèle servi par API (§4) — `Qwen3.8-27B` ou `Holo4-35B-A3B` (Apache-2.0) auto-hébergés, ou passerelle admise | texte de la `<legend>` (ou du libellé du groupe), libellés des champs regroupés, conditions structurées | `review` |
| **11.8.3** | Assisté | LLM texte seul — famille « résumé et libellé », **non calibrée** | modèle servi par API (§4) — `Qwen3.8-27B` ou `Holo4-35B-A3B` (Apache-2.0) auto-hébergés, ou passerelle admise | libellés des `<optgroup>` et des options regroupées, conditions structurées | `review` |

[^1_3_1]: calibration : 33/39 = 85 %, kappa 0,68, ancres 4/4 (repère trivial 67 %) — #339 ; critère : ADR 0001
[^1_4_1]: **justification** : juger l'alternative d'une image-test suppose de connaître la finalité du test, que la page ne déclare pas. Classé « décision de fond » à l'inventaire.
[^1_7_1]: sonde existante `probe-1-7-description-relevance` (js-static, partielle) conservée pour l'applicabilité
[^5_2_1]: `table-duplicate-name` d'axe donne un indice d'applicabilité, pas un verdict
[^8_6_1]: IBM Equal Access donne un indice complémentaire

### G3 — multimédia et flashs (18 tests)

| Test | Niveau | Technique | Bibliothèque (licence) | Preuve produite | Statut par défaut |
|---|---|---|---|---|---|
| **4.1.1**, **4.1.2**, **4.1.3** | Assisté | sidecar `ffprobe` : inventaire des pistes et marqueurs [^4_1_1] | `ffmpeg` / `ffprobe` du **système** (GPL — non redistribué, appelé en sidecar) | pistes (`v`/`a`/`s`), langues, titres, `disposition` ; **une absence ne prouve rien** | `review` |
| **4.2.1** | Assisté | transcription automatique puis comparaison à la transcription publiée — famille « transcription et sous-titres », **non calibrée** [^4_2_1] | `whisper-rs` (Unlicense) / `whisper.cpp` + `ffmpeg` / `ffprobe` du **système** (GPL — non redistribué, appelé en sidecar) | transcription produite, texte publié, écarts relevés | `review` |
| **4.2.2**, **4.2.3** | Irréductible | — [^4_2_2] | — | inventaire du média et des alternatives déclarées | `review` d'inventaire |
| **4.3.1** | Automatique | sidecar : piste de sous-titres présente **et non vide** [^4_3_1] | `ffmpeg` / `ffprobe` du **système** (GPL — non redistribué, appelé en sidecar) | piste extraite en WebVTT, nombre de cues, contenu non vide | `pass` sur preuve ; **absence de piste → `review`** |
| **4.4.1** | Assisté | comparaison des sous-titres à la bande audio transcrite — famille « transcription et sous-titres », **non calibrée** | `whisper-rs` (Unlicense) + `ffmpeg` / `ffprobe` du **système** (GPL — non redistribué, appelé en sidecar) | cues WebVTT, transcription de l'audio, écarts de synchronisation et de contenu | `review` |
| **4.5.1**, **4.5.2** | Assisté | sidecar `ffprobe` : marqueur d'audiodescription [^4_5_1] | `ffmpeg` / `ffprobe` du **système** (GPL — non redistribué, appelé en sidecar) | `disposition` (`descriptions`, `visual_impaired`) **et** `title` — les deux, car la `disposition` est perdue en WebM et le `title` en MP4 | `review` |
| **4.6.1**, **4.6.2** | Irréductible | — [^4_6_1] | — | inventaire : piste d'audiodescription marquée ou non | `review` d'inventaire |
| **4.7.1** | Assisté | inventaire DOM du texte adjacent au média | — (DOM via Obscura) | élément média, texte adjacent (légende, titre, intitulé du lien), de quoi juger s'il l'identifie clairement | `review` |
| **4.8.1**, **4.8.2** | Assisté | inventaire DOM des alternatives de média non temporel | — (DOM via Obscura) | élément média, alternative déclarée, texte adjacent | `review` |
| **4.9.1** | Irréductible | — [^4_9_1] | — | inventaire : alternative présente ou non | `review` d'inventaire |
| **4.10.1** | Automatique | sidecar `silencedetect` : durée de son **audible** [^4_10_1] | `ffmpeg` / `ffprobe` du **système** (GPL — non redistribué, appelé en sidecar) | durée audible mesurée sur la durée totale, ou absence de piste audio | `pass` déterministe si aucune piste audio, lecture non automatique, média muet, ou ≤ 3 s audible ; sinon `review` |
| **13.7.1** | Automatique | module Rust : seuil WCAG en **pixels absolus** (21 824 px = 341 × 256 / 4), suivi d'extrema et hystérésis [^13_7_1] | `image` (MIT/Apache-2.0) + `ffmpeg` / `ffprobe` du **système** (GPL — non redistribué, appelé en sidecar) ; **IRIS (BSD-3) en validation CI** | surface clignotante mesurée en px, fréquence, horodatage, et la part rouge saturée | **`fail` sur preuve** ; `pass` maintenu en `review` jusqu'à validation contre IRIS |

[^4_1_1]: palier A — 128–212 Ko lus sur 18 Mo (≈1 %), sans téléchargement du média (#340)
[^4_2_1]: WER français 15 % (small) → 7,7 % (large) ; **Whisper « à valider »** avant tout usage en production
[^4_2_2]: **justification** : juger qu'une alternative restitue un média temporel entier dépasse ce qu'une machine constate. À prototyper (#333) avant tout reclassement.
[^4_3_1]: mesuré : extraction 151–166 ms, 2 cues, contenu non vide en MP4 / MKV / WebM (#340)
[^4_5_1]: mesuré (#340) ; une absence de marqueur ne prouve pas l'absence d'audiodescription
[^4_6_1]: **justification** : la pertinence d'une audiodescription suppose de juger ce qui est utile à décrire dans une scène. Aucune mesure ne s'en approche.
[^4_9_1]: **justification** : équivalence d'accès au même contenu, non constatable.
[^4_10_1]: mesuré 10/10 sur fixtures, 89–151 ms par média (#340)
[^13_7_1]: 10/10 aux bornes après correction d'un défaut (rampe lente comptée comme flash) ; ≈13× temps réel en 720p (#340) ; palier C

### G4 — tests visuels (26 tests)

| Test | Niveau | Technique | Bibliothèque (licence) | Preuve produite | Statut par défaut |
|---|---|---|---|---|---|
| **1.8.1**, **1.8.2**, **1.8.3**, **1.8.4**, **1.8.5**, **1.8.6** | Assisté | OCR du texte en image + mesure de contraste [^1_8_1] | `ocrs` (crate MIT/Apache-2.0, **poids sans licence déclarée**) ou `tesseract` ; `image` (MIT/Apache-2.0), `imageproc` (MIT), `palette` (MIT/Apache-2.0) | recadrage de l'image, texte reconnu, contraste mesuré | `review` |
| **3.1.1**, **3.1.2**, **3.1.3**, **3.1.4**, **3.1.6** | Assisté | simulation du daltonisme (Viénot / Brettel, **à coder**) et comparaison des rendus [^3_1_1] | `image`, `palette`, `imageproc` (MIT / MIT-Apache-2.0) | rendu normal et rendu simulé, zones où l'information disparaît | `review` |
| **3.1.5** | Irréductible | — [^3_1_5] | — | inventaire des usages de la couleur | `review` d'inventaire |
| **3.2.1**, **3.2.2**, **3.2.3**, **3.2.4** | Assisté | contraste **restreint au texte en image et aux fonds complexes** [^3_2_1] | `ocrs` / `tesseract`, `image`, `palette` | recadrage, texte reconnu, couleurs échantillonnées, rapport de contraste | `review` |
| **3.2.5** | Irréductible | — [^3_2_5] | — | inventaire | `review` d'inventaire |
| **3.3.1**, **3.3.2**, **3.3.3** | Assisté | contraste des éléments d'interface non textuels | `image`, `palette`, `imageproc` | recadrage de l'élément, couleurs échantillonnées, rapport mesuré | `review` |
| **3.3.4** | Irréductible | — [^3_3_4] | — | inventaire | `review` d'inventaire |
| **10.7.1** | Automatique | différence de pixels avant / après prise de focus [^10_7_1] | `image` (MIT/Apache-2.0), `imageproc` (MIT) | capture avant focus, capture après focus, surface et position des pixels changés | **`fail` sur preuve — « Chrome requis »** ; « non testé » sous Obscura |
| **10.10.1** | Assisté | analyse d'image | `image`, `imageproc` | recadrage, mesure | `review` |
| **10.10.2**, **10.10.3**, **10.10.4** | Irréductible | — [^10_10_2] | — | inventaire | `review` d'inventaire |

[^1_8_1]: **bloquant** : poids `ocrs` ni téléchargés ni embarqués avant confirmation de licence (#335, #343)
[^3_1_1]: aucune bibliothèque Rust ne fournit la simulation : à implémenter
[^3_1_5]: **justification** : Irréductible par l'image (#343)
[^3_2_1]: inclusion décidée en #343, périmètre restreint ; le contraste du texte HTML relève d'axe
[^3_2_5]: **justification** : Irréductible par l'image (#343)
[^3_3_4]: **justification** : Irréductible par l'image (#343)
[^10_7_1]: **Obscura ne peint aucun anneau de focus natif et n'implémente pas `:focus` / `:focus-visible`** — 0 pixel changé mesuré contre Chrome. Le chemin Chrome facultatif est obligatoire pour ce test.
[^10_10_2]: **justification** : Irréductible par l'image (#343)

### G5 — compréhension et équivalence (38 tests)

| Test | Niveau | Technique | Bibliothèque (licence) | Preuve produite | Statut par défaut |
|---|---|---|---|---|---|
| **5.1.1** | Automatique | existence du résumé d'un tableau de données complexe [^5_1_1] | — (DOM via Obscura) | structure du tableau, en-têtes, portées, présence du résumé | **`fail` / `pass` si le tableau est clairement complexe ; `review` si la complexité est ambiguë** |
| **5.3.1** | Assisté | distinction tableau de données / tableau de mise en forme | axe-core 4.14.0 (MPL-2.0) — heuristique `isDataTable` | verdict de l'heuristique, structure du tableau, linéarisation | `review` |
| **7.2.1**, **7.2.2** | Assisté | comparaison du DOM **sans scripts** au DOM rendu [^7_2_1] | `html5ever` (MIT/Apache-2.0) avec `scripting_enabled=false` | les deux arbres et leurs écarts | `review` |
| **7.4.1** | Assisté | inventaire des changements de contexte [^7_4_1] | — (surcharge de `addEventListener`) | écouteurs inventoriés, déclencheurs, preuve d'état | `review` |
| **7.5.1**, **7.5.2**, **7.5.3** | Assisté | inventaire des régions de statut **par le DOM** [^7_5_1] | — (DOM via Obscura) | éléments `role="status"`/`alert`, attributs `aria-live`, contenu avant/après | `review` |
| **8.7.1** | Assisté | détection de la langue des passages | `lingua` (Apache-2.0) | passage, langue détectée et confiance, `lang` déclaré | `review` |
| **9.4.1**, **9.4.2** | Assisté | indice limité [^9_4_1] | IBM Equal Access (Apache-2.0) en complément | extrait, indice fourni | `review` |
| **10.3.1** | Assisté | comparaison ordre DOM / ordre visuel [^10_3_1] | — (`DOM.getBoxModel` sous Obscura) | ordre DOM, géométrie réelle des éléments, écarts | `review` |
| **11.3.1**, **11.3.2** | Assisté | libellés de champs [^11_3_1] | — (DOM) | champ, libellé associé, contexte | `review` |
| **11.10.1**, **11.10.2**, **11.10.3**, **11.10.4**, **11.10.5**, **11.10.6**, **11.10.7** | Assisté | contrôle de saisie | — (DOM) | champ, contraintes déclarées, messages | `review` |
| **11.11.1**, **11.11.2** | Assisté | aide à la saisie | — (DOM) | champ, aide associée | `review` |
| **11.12.1**, **11.12.2** | Assisté | applicabilité seule [^11_12_1] | — (DOM) | formulaire `post` portant `password`/`iban`/`card`, boutons « payer » / « supprimer » | `review` |
| **12.3.1** | Assisté | représentativité du plan du site [^12_3_1] | — (liens du plan en `HEAD`) | liens du plan, architecture constatée | `review` |
| **12.3.2** | Automatique | atteignabilité des cibles du plan du site [^12_3_2] | — (requêtes `HEAD` plafonnées) | chaque lien du plan et son code de réponse | `fail` / `pass` |
| **12.3.3** | Assisté | intitulés et titres des cibles | — (DOM des cibles) | intitulé du lien, titre de la page cible | `review` |
| **12.10.1** | Assisté | raccourcis clavier à une seule touche [^12_10_1] | — (surcharge de `addEventListener`) | écouteurs clavier globaux inventoriés | `review` |
| **12.11.1** | Assisté | contenus additionnels au survol / focus [^12_11_1] | — (`Input.dispatchMouseEvent`) | déclencheur, contenu apparu, moyen de le fermer | `review` |
| **13.1.4** | Irréductible | — [^13_1_4] | — | inventaire : `meta-refresh`, hooks `location.*` / `setTimeout` | `review` d'inventaire |
| **13.6.1** | Assisté | détection de contenus cryptiques (art ASCII, émoticônes) [^13_6_1] | — (aucune bibliothèque adaptée) | passage détecté, alternative présente ou non | `review` |
| **13.10.1**, **13.10.2** | Assisté | inventaire des widgets à geste | — (DOM + écouteurs) | widget, gestes écoutés, alternative constatée | `review` |
| **13.11.1** | Assisté | handlers portant l'action sur `mousedown` / `pointerdown` [^13_11_1] | — (surcharge de `addEventListener`) | élément, événement portant l'action | **`review` — jamais `fail`** |
| **13.12.1**, **13.12.2**, **13.12.3** | Automatique | inventaire des écouteurs de capteurs [^13_12_1] | — (surcharge de `addEventListener`) | écouteurs `devicemotion` / `deviceorientation` / `Accelerometer` — **l'inventaire doit être prouvé complet** | **`non applicable` si aucun écouteur de capteur ; sinon `review`** |

[^5_1_1]: Automatique **conditionnel** (#347) ; sonde `plan_snippets.rs` à aligner
[^7_2_1]: **`Emulation.setScriptExecutionDisabled` est sans effet sous Obscura 0.2.4** (vérifié) : d'où l'analyse statique au lieu du rechargement sans JS
[^7_4_1]: le computer use enrichit la preuve, **ne change pas le niveau** (#349)
[^7_5_1]: **l'arbre AX d'Obscura 0.2.4 ne porte ni les rôles de statut ni les propriétés live** (vérifié) : lecture du DOM obligatoire
[^9_4_1]: le plus proche de l'Irréductible du groupe (#337)
[^10_3_1]: **géométrie réelle disponible sous Obscura 0.2.4** (vérifié)
[^11_3_1]: sonde `plan_snippets.rs` à aligner (#347)
[^11_12_1]: **puis Irréductible après validation** : l'applicabilité se détecte, la conformité non (#347)
[^12_3_1]: classé « décision de fond » à l'inventaire
[^12_3_2]: `HEAD` **plafonné** (#344)
[^12_10_1]: sonde `plan_snippets.rs` à aligner (#347)
[^12_11_1]: le computer use enrichit la preuve, ne change pas le niveau
[^13_1_4]: **justification** : les exceptions de 13.1 (activité en cours, limite essentielle) ne sont pas constatables (#347)
[^13_6_1]: **aucune bibliothèque** identifiée (#346)
[^13_11_1]: les exceptions sont indécidables ; un échec peut être légitime sur appui (#344)
[^13_12_1]: Automatique **uniquement pour le « non applicable »** et **sous réserve** que la complétude de l'inventaire soit prouvée par fixture (#344)
## 4. Bibliothèques, licences, modèles

Licences vérifiées. `—` signifie qu'aucune bibliothèque externe n'intervient : le test
s'appuie sur le DOM, le CDP d'Obscura ou du code de ce dépôt.

| Brique | Licence | Usage | Réserve |
|---|---|---|---|
| axe-core 4.14.0 | MPL-2.0 | règles G1, `isDataTable` (5.3.1) | **embarquée** dans le dépôt (plus de CDN) ; compatible avec un binaire MIT à condition de garder `axe.min.js` intact, le texte MPL, `LICENSE-3RD-PARTY.txt` et un pointeur vers les sources |
| `ffmpeg` / `ffprobe` | GPL | tout le thème 4 et 13.7.1 | **du système, non redistribué**, appelé en sidecar |
| `whisper-rs` 0.16 / `whisper.cpp` | Unlicense | 4.2.1, 4.4.1 | **« à valider »** avant production (WER français 15 % small → 7,7 % large) |
| IRIS | BSD-3 | validation CI du détecteur de flashs | pas embarquée : oracle de comparaison |
| `image` 0.25 | MIT OR Apache-2.0 | 13.7.1, G4 | |
| `imageproc` 0.27 | MIT | G4 | |
| `palette` 0.7 | MIT OR Apache-2.0 | G4 (contraste, daltonisme) | simulation Viénot / Brettel **à coder** : aucune crate ne la fournit |
| `ocrs` 0.13 | MIT OR Apache-2.0 (crate) | 1.8.x, 3.2.x | **bloquant : la licence des poids n'est déclarée nulle part.** Ni téléchargés ni embarqués avant confirmation |
| `html5ever` 0.39 | MIT OR Apache-2.0 | 7.2.x (`scripting_enabled=false`) | |
| `lingua` | Apache-2.0 | 8.7.1 | |
| Alfa | MIT | 19 règles à questions typées, branchables sur nos réponses | évalué, non intégré |
| IBM Equal Access | Apache-2.0 | indices sur 9.4, 5.3, 13.6, 3.1, 8.6 | 10.10 inutilisable (liste de mots anglais) |
| `dssim-core` | AGPL | — | **exclu** |
| Asqatasun | AGPL | référence de comparaison | pas une brique |

**Modèles.** Aucun n'est épinglé ; le §2.6 fixe le seuil d'admission.

| Modèle | Licence | Note |
|---|---|---|
| `Qwen3.8-27B` | Apache-2.0 | auto-hébergeable |
| `Holo4-35B-A3B` | Apache-2.0 | auto-hébergeable ; Holo 4 est un affinage *computer use* de Qwen |
| `holo4-27b` | **CC-BY-NC-4.0** | **usage non commercial** : inutilisable en production |
| `swift-1.5-27b` (passerelle MyIA) | — | mesuré : admis en Assisté, échoue le seuil de faux « pertinent » à 19 % |
| `gemma4:e2b` (local) | — | lot D2 propre : 22/39 = 56 %, **sous son repère trivial de 67 %**, kappa 0,26, 17 faux « pertinent » sur 26 rejets : insuffisant pour l'alternative d'image. (Un premier chiffre, 67 % vs 70 %, venait du lot C à étiquetage vicié ; il est remplacé.) Sur la description détaillée (lot G) il atteint 92 % : lot non discriminant |

## 5. Prérequis bloquants

Aucun lot d'implémentation ne démarre avant leur résolution.

1. **`axe.run()` doit remonter les `incomplete`** — sans quoi la montée en 4.14 perd les
   preuves `review` qui justifient le §2.10.
2. **Licence des poids `ocrs`** — non déclarée. Bloque 1.8.x et 3.2.x, soit 10 tests.
3. **Validation de Whisper** — bloque 4.2.1 et 4.4.1.
4. **Validation du détecteur de flashs contre IRIS** — tant qu'elle manque, 13.7.1 rend
   `fail` sur preuve mais son `pass` reste en `review`.
5. **Chemin Chrome facultatif** — obligatoire pour 10.7.1 : Obscura 0.2.4 n'implémente ni
   `:focus` ni `:focus-visible` et ne peint aucun anneau de focus natif (0 pixel changé
   mesuré contre Chrome).
6. **Granularité `tests` du registre** — `mechanisms.toml` ne déclare ses mécanismes que
   par critère, aucun champ `tests` renseigné. Cet addendum raisonne par test : le registre
   doit suivre.
7. **Complétude de l'inventaire d'écouteurs, prouvée par fixture** — condition du
   « non applicable » Automatique de 13.12.x.
8. **Garanties des fournisseurs d'API** — non-entraînement, région UE et DPA signé exigés
   avant toute activation en production. **Les trois ne sont réunies pour aucun fournisseur
   sur les textes lus** (constat, pas jugement de conformité) : à défaut, modèle
   auto-hébergé.
9. **Second étiqueteur** — requis pour établir que le critère de pertinence de l'ADR 0001
   est celui de la profession et non celui d'un auditeur. Ne bloque pas le niveau
   « Assisté », qui tient quel que soit l'arbitrage ; bloque l'application du critère au
   code.

## 6. Ce que cet addendum ne couvre pas

- **13.7.2 et 13.7.3** — hors addendum (#342).
- **4.11 à 4.13** — renvoyés à [#247](https://github.com/jamon8888/Holo-RGAA/issues/247).
- **33 règles axe sans étiquette RGAA** — renvoyées à #247.
- **Budget LLM et précédence des verdicts** — restent sur
  [#180](https://github.com/jamon8888/Holo-RGAA/issues/180).
- **Les 10 écarts de rattachement registre / axe 4.14** — l'étiquette d'axe fait foi par
  défaut ; tout ajout du registre doit être justifié et prouvé.
- **Conformité juridique** — le §2.7 est une politique produit. La lecture des textes
  contractuels des fournisseurs est consignée dans
  [#351](https://github.com/jamon8888/Holo-RGAA/issues/351), sans conclusion juridique.

## 7. Calibration — état

| Famille de preuve | Tests | État |
|---|---|---|
| alternative d'image | 1.3.1–1.3.9 | **calibrée** : 33/39 = 85 %, kappa 0,68, ancres 4/4, repère trivial 67 % |
| description détaillée | 1.7.1–1.7.6 | non calibrée |
| transcription et sous-titres | 4.2.1, 4.4.1 | non calibrée |
| audiodescription | 4.6.1, 4.6.2, 4.9.1 | non calibrée (ces tests sont Irréductibles : la calibration ne porterait que sur la preuve d'inventaire) |
| résumé et libellé | 5.2.1, 5.5.1, 8.6.1, 11.7.1, 11.8.3 | non calibrée |

Protocole et outillage : [#354](https://github.com/jamon8888/Holo-RGAA/issues/354). Budget
~200 cartes, ~5 h d'étiquetage, à doubler pour le second étiqueteur.

**Règle d'étiquetage non négociable** : justification obligatoire sur chaque carte, verdict
favorable compris. Sans elle, le verdict ne porte aucune information propre — mesuré à
phi = −1,000 sur 39 cartes, avec une ancre factuellement fausse acceptée sans un mot.
Correctif appliqué : ancres 3/4 → 4/4.
