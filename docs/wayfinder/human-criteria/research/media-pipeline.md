# Recherche : chaîne d'analyse média open source (thème 4 et 13.7)

> Ticket [#334](https://github.com/jamon8888/Holo-RGAA/issues/334) — carte [#330](https://github.com/jamon8888/Holo-RGAA/issues/330).
> Date : 2026-10-12. Vocabulaire : `CONTEXT.md` (Test RGAA, Automatique, Assisté, Irréductible).

Légende des sources : **[S]** fait établi par la source citée ; **[E]** vérifié par une expérience locale décrite ici ; **[I]** inférence de l'auteur ; **non vérifié** = non établi dans cette recherche.

## Réponse courte

- `ffprobe` (sidecar) classe chaque média (audio seul / vidéo seule / synchronisé), liste les pistes audio et de sous-titres et lit leurs *dispositions* (`descriptions`, `visual_impaired`, `captions`, `hearing_impaired`). Mais il ne les lit que si l'auteur du fichier les a renseignées, et le conteneur peut les perdre. Une absence n'est donc jamais une preuve de non-conformité.
- Flashs : **aucune implémentation Rust** trouvée sur crates.io. La référence open source est **IRIS d'Electronic Arts** (C++, **BSD-3-Clause**, compatible MIT). Le seuil WCAG est public et déterministe : un portage Rust sur les images brutes produites par `ffmpeg` est faisable [I], avec IRIS comme oracle de validation.
- Transcription : **whisper.cpp** (MIT, poids Whisper MIT) via **whisper-rs** (Unlicense), ou via le filtre `whisper` de FFmpeg 8 (non compilé dans le paquet Ubuntu). Qualité publiée en français (Fleurs, WER) : de 15,0 % (`small`) à 7,7 % (`large`). C'est assez pour un **indice** de pertinence, pas pour un verdict.
- OCR des sous-titres incrustés : Tesseract (Apache-2.0). Résultat bruité sur fond chargé (essai local) : **Assisté** au mieux.
- Bilan : grâce au sidecar, un seul test passe **Automatique**, et sous conditions : 13.7.1, côté `fail`, et côté `pass` si le média entier est analysé. 4.10.1 gagne des `pass` déterministes sur certaines branches. Le reste du thème 4 devient **Assisté**, avec une preuve bien meilleure qu'aujourd'hui. La pertinence de l'audiodescription et des alternatives non temporelles reste proche de l'**Irréductible**.

## 1. Licences et intégration de FFmpeg en sidecar

- FFmpeg est sous **LGPL 2.1 ou ultérieure**. Des composants optionnels sont sous GPL v2+ ; s'ils sont activés (`--enable-gpl`, `--enable-nonfree`), « the GPL applies to all of FFmpeg » [S] <https://ffmpeg.org/legal.html>.
- La checklist LGPL de FFmpeg exige notamment de distribuer le source exact des binaires distribués et une mention dans la documentation [S] <https://ffmpeg.org/legal.html>.
- Le binaire de la distribution sur cette machine (Ubuntu, `ffmpeg 8.0.1-3ubuntu2`) est compilé avec `--enable-gpl` [E] (`ffmpeg -version`). Il est donc sous GPL.
- Conséquence [I] : appeler `ffmpeg`/`ffprobe` comme **processus séparé** (déjà la décision de la carte) n'impose pas la GPL au code MIT de `rgaa-rs`. C'est la lecture usuelle de la FAQ GPL sur les programmes séparés qui communiquent par pipes ou arguments (<https://www.gnu.org/licenses/gpl-faq.html#MereAggregation>, **non relue dans cette session** : accès 403). En revanche, **si `install.sh` redistribue un binaire FFmpeg**, les obligations LGPL/GPL de distribution du source s'appliquent à ce binaire. S'appuyer sur le FFmpeg du système (paquet de distribution) évite cette redistribution [I].
- Crates Rust pour piloter le sidecar (crates.io, vérifié le 2026-10-12) [S] :
  - [`ffmpeg-sidecar`](https://crates.io/crates/ffmpeg-sidecar) 2.6.0, **MIT**. Elle enveloppe un binaire FFmpeg autonome et expose les images en RGB brut (« raw RGB pixels ») <https://github.com/nathanbabcock/ffmpeg-sidecar>. Sa fonction `auto_download()` télécharge un binaire, et le README ne dit rien de la licence de ce binaire [S] → **ne pas l'utiliser en production** sans avoir vérifié la provenance [I].
  - [`ffprobe`](https://crates.io/crates/ffprobe) 0.4.0, **MIT** : wrapper typé de la CLI `ffprobe`, dernière mise à jour le 2024-04-03 [S]. Un appel direct à `ffprobe -of json` + `serde` est une alternative triviale [I].
  - [`ffmpeg-next`](https://crates.io/crates/ffmpeg-next) 9.0.0, **WTFPL** : liaison en bibliothèque (FFI), donc **hors** de la décision « sidecar ». Liée à une libav GPL, elle contaminerait le binaire [I]. Écartée.

## 2. `ffprobe` : ce que l'on peut établir sur un fichier média

- Options utiles : `-show_streams`, `-show_format`, `-show_entries`, `-select_streams`, `-of json` [S] <https://ffmpeg.org/ffprobe.html>.
- Champs exploitables : `codec_type` (audio / video / subtitle), `format.duration`, `tags.language`, et les dispositions `default`, `captions`, `descriptions`, `hearing_impaired`, `visual_impaired`, `dub`, `original` [S] <https://ffmpeg.org/ffprobe.html>.
- **Expérience [E]** : fichier synthétique avec une vidéo, deux pistes audio (la 2ᵉ marquée `visual_impaired+descriptions`) et une piste de sous-titres marquée `hearing_impaired+captions`.
  - En **MP4** : `ffprobe` relit toutes les dispositions posées (`visual_impaired=1`, `descriptions=1`, `hearing_impaired=1`, `captions=1`) et `duration=2.000000`.
  - En **MKV** : `visual_impaired` et `hearing_impaired` sont relus, mais `descriptions` et `captions` **sont perdus**.
  - Leçon [I] : détecter l'audiodescription ou les sous-titres par les métadonnées donne des **positifs fiables** et des **négatifs non concluants**.
- Classification du média [I] : le DOM ne dit pas si une `<video>` contient du son, `ffprobe` si (présence d'une piste `audio`). On peut donc router vers 4.1.2/4.5.1 (vidéo seule) ou vers 4.1.3/4.3/4.5.2 (synchronisé). Une piste audio présente mais silencieuse se détecte avec `silencedetect` (voir §5).
- Limites [I] : les lecteurs tiers en `<iframe>` (YouTube, Vimeo…) et les flux protégés (DRM) ne sont pas téléchargeables ; ces médias restent en revue. Les manifestes HLS/DASH sont lisibles par `ffprobe` (protocoles d'entrée FFmpeg). En revanche, la signalisation de l'audiodescription dans HLS (attribut `CHARACTERISTICS`) n'a **pas été vérifiée** ici.

## 3. Flashs (13.7)

### Seuil normatif

- WCAG 2.3.1 : rien ne flashe plus de trois fois par seconde, **ou** le flash est sous les seuils généraux et rouges [S] <https://www.w3.org/TR/WCAG22/#three-flashes-or-below-threshold>.
- Définition des seuils [S] <https://www.w3.org/WAI/WCAG22/Understanding/three-flashes-or-below-threshold.html> :
  - au plus 3 flashs généraux et/ou rouges par seconde, ou une surface cumulée simultanée ≤ 0,006 stéradian dans tout champ de 10° (≈ 25 % du champ) ;
  - un flash général est une paire de changements opposés de luminance relative ≥ 10 % du maximum, l'état le plus sombre étant < 0,80 ;
  - un flash rouge implique un état avec R/(R+G+B) ≥ 0,8 et un écart de chromaticité > 0,2 (CIE 1976 UCS).
- RGAA 13.7 traduit la surface en **21824 pixels** (intitulés de 13.7.2 / 13.7.3, `criteres.json`). Or 21824 = 341 × 256 / 4, soit 25 % de la fenêtre de référence 341 × 256 [I, calcul].

### Implémentations open source

| Outil | Licence | État | Verdict pour nous |
|---|---|---|---|
| **IRIS** (Electronic Arts) — <https://github.com/electronicarts/IRIS> | **BSD-3-Clause** (fichier `LICENSE.txt`) [S] | C++, releases 1.0.0 et 1.1.0, dernier push le 2025-01-14 [S] | Compatible MIT. Détecte les flashs de luminance, les flashs rouges et les motifs. Sortie CSV/JSON par image, avec un statut pass / warning / extended fail / fail [S, README]. Dépendances vcpkg : OpenCV (Apache-2.0 [S]) avec la fonctionnalité `ffmpeg`, spdlog, nlohmann-json, giflib, gtest [S, `vcpkg.json`]. Presets CMake Linux présents [S]. **Build non testé ici**. |
| **PEAT** (Trace Center) — <https://trace.umd.edu/?p=705> | open source « except for a proprietary analysis engine » [S] | jugé vieillissant par Trace ; une version libre est annoncée, sans date [S] | Inutilisable (moteur propriétaire, Windows). |
| Filtre FFmpeg `photosensitivity` | celle de FFmpeg | présent dans FFmpeg 8 [E] | Il **réduit** les flashs (« Reduce various flashes in video ») avec un facteur de seuil arbitraire, sans référence à WCAG [S] <https://ffmpeg.org/ffmpeg-filters.html#photosensitivity>. Ce n'est pas un détecteur conforme. Écarté. |
| `flashdetect` (Python, GitHub `loopglitch26/flashdetect`) | non vérifiée | non vérifié | Vu seulement dans une recherche web ; non examiné. |
| **Rust** | — | **aucune crate trouvée** | Recherches crates.io `photosensitive`, `epilepsy`, `seizure`, `flashing`, `flash detection`, `wcag-flash` (2026-10-12) : aucun résultat pertinent [S]. |

### Écart IRIS / RGAA à signaler

- IRIS mesure la surface en **proportion de l'image** (`"AreaProportion": 0.25` dans `config/appsettings.json`) [S] <https://github.com/electronicarts/IRIS/blob/main/config/appsettings.json>.
- RGAA fixe une surface **absolue** (21824 px) à la taille rendue. Pour une vidéo affichée en 1280 × 720, 25 % de l'image font 230 400 px, plus de dix fois le seuil RGAA [I, calcul]. IRIS pourrait donc déclarer `pass` un flash que RGAA juge non conforme. Il faut soit paramétrer IRIS avec la taille rendue (lue dans le DOM), soit implémenter notre propre agrégation de surface.
- Le README d'IRIS précise que l'outil n'est pas destiné à « guarantee, certify or otherwise validate » la conformité [S].

### Recommandation [I]

1. `ffmpeg` (sidecar) décode le média en images RGB brutes (`ffmpeg-sidecar` ou pipe `rawvideo`), mises à l'échelle à la **taille rendue** de l'élément.
2. Un module Rust implémente la définition WCAG : luminance relative sRGB, transitions ≥ 0,1 avec un état sombre < 0,8, flash rouge, fenêtre glissante d'une seconde, surface cumulée en pixels comparée à 21824. C'est du calcul déterministe, pas un modèle.
3. IRIS sert d'**oracle** en CI sur des fixtures (vidéos synthétiques conformes et non conformes). Ce n'est pas une dépendance d'exécution.
4. Cela couvre 13.7.1 pour `<video>` et les GIF / APNG / WebP animés, décodables par FFmpeg [I] (APNG et WebP non testés ici).
5. Pour `<svg>`, `<canvas>`, les scripts (13.7.2) et le CSS (13.7.3), il faut une **séquence de captures** de la page rendue. La capacité d'Obscura à fournir un screencast à fréquence suffisante n'est **pas vérifiée**, et la carte prévoit une capture par page. Sans cette capacité, ces tests restent à la sonde statique actuelle `probe-13-7` (`review`).

## 4. Transcription locale (4.1, 4.2, 4.3, 4.4)

- **whisper.cpp** : licence **MIT** [S] (API GitHub `ggml-org/whisper.cpp`), dernière release `v1.9.5` du 2026-10-06 [S], inférence sur CPU seul supportée [S] <https://github.com/ggml-org/whisper.cpp>.
  - Modèles multilingues (donc français) : `tiny` 75 MiB, `base` 142 MiB, `small` 466 MiB, `medium` 1,5 GiB, `large` 2,9 GiB.
  - Variantes : `large-v3-turbo` 1,5 GiB, `large-v3-turbo-q5_0` 547 MiB [S] <https://github.com/ggml-org/whisper.cpp/blob/master/models/README.md>.
- **Poids Whisper** : « code and model weights are released under the MIT License » [S] <https://github.com/openai/whisper>.
- **whisper-rs** 0.16.0 (2026-03-12), **Unlicense**, dépôt <https://codeberg.org/tazz4843/whisper-rs> ; `whisper-rs-sys` 0.15.0, Unlicense [S] <https://crates.io/crates/whisper-rs>. C'est une liaison FFI vers une bibliothèque MIT, donc sans souci de licence [I]. La version de whisper.cpp embarquée par `whisper-rs-sys` n'est **pas vérifiée**.
- **Filtre FFmpeg `whisper`** : transcription dans `ffmpeg` même, sortie `text`, `srt` ou `json`, option VAD (Silero). Il nécessite `./configure --enable-whisper` [S] <https://ffmpeg.org/ffmpeg-filters.html#whisper>. Il est **absent** du FFmpeg Ubuntu de cette machine [E] → ne pas en dépendre.
- **Qualité en français** — WER (%) sur Fleurs, table 13 de Radford et al. 2022 [S] <https://arxiv.org/abs/2212.04356> :

  | Modèle | tiny | base | small | medium | large | large-v2 |
  |---|---|---|---|---|---|---|
  | WER français (%) | 41,4 | 28,5 | 15,0 | 8,7 | 7,7 | 8,3 |

  Les chiffres en français pour `large-v3` / `turbo` ne sont **pas vérifiés**. Fleurs est de la parole lue : sur des vidéos réelles (musique, bruit, plusieurs voix), il faut s'attendre à plus d'erreurs [I].
- **Coût** : le README donne la mémoire par modèle (~852 MB pour `small`, ~2,1 GB pour `medium`) [S]. Le **temps** de transcription sur la machine cible n'a **pas été mesuré**. Il faut le mesurer avec `prototype` avant de fixer un modèle par défaut.
- **Usage pour 4.2 / 4.4** [I] : transcrire la piste audio, puis comparer le résultat à la transcription fournie (4.2) ou au texte WebVTT/SRT aligné sur les horodatages (4.4). Mesures : taux de recouvrement des mots, segments de parole sans sous-titre, décalage temporel. Avec un WER de 8 à 15 %, on obtient un **indice** (« 40 % de la parole n'apparaît pas dans la transcription ») et une preuve localisée, jamais un `pass`. Le jugement de pertinence RGAA va au-delà du mot-à-mot (bruits, locuteurs, informations visuelles) → **Assisté**.

## 5. Son déclenché automatiquement (4.10)

- `silencedetect`, `volumedetect` et `ebur128` existent dans FFmpeg [E]. **Expérience [E]** : 2 s de sinus suivies de 3 s de silence donnent `silence_start: 2` et `silence_duration: 3`. On peut donc mesurer la **durée audible** d'une piste.
- Combinaison [I] : le DOM (`autoplay` présent, `muted` absent, `controls` présent ou non) + `ffprobe` (piste audio) + `silencedetect` (durée audible).
  - Les branches « durée ≤ 3 s » et « pas de piste audio / piste silencieuse » donnent un `pass` déterministe. Les `controls` natifs aussi, puisqu'ils permettent l'arrêt [I — à confirmer par une décision de la carte].
  - Le `fail` reste Assisté : un bouton d'arrêt personnalisé en JS peut exister.
  - Le son produit par JavaScript (Web Audio, `new Audio()`) n'est pas observable sans moteur média dans Obscura [I].

## 6. Sous-titres incrustés (4.3.1)

- Tesseract : **Apache-2.0**, release `5.5.3` ; `tessdata_fast` en Apache-2.0 [S] (API GitHub `tesseract-ocr`). Le filtre FFmpeg `ocr` requiert `--enable-libtesseract` [S] <https://ffmpeg.org/ffmpeg-filters.html#ocr>. Il est absent du FFmpeg Ubuntu local [E] → appeler `tesseract` en CLI sur des images extraites.
- **Expérience [E]** : sous-titre anglais incrusté par `libass` sur une mire colorée (`testsrc2`), image extraite à t = 1 s, OCR avec Tesseract (`eng`).
  - Sur le quart inférieur, Tesseract rend « TMetaquickabrown fox jumps} » pour « The quick brown fox jumps ».
  - Sur l'image entière, il rend « ine) brown fox jumps| ».
  - Le texte est **reconnaissable mais bruité**. Le pack `fra` n'est pas installé ici.
- Usage [I] : échantillonner une image toutes les ~2 s dans la bande basse, passer l'OCR, puis rapprocher le texte du transcript Whisper. Une forte correspondance est un bon **indice** de sous-titres incrustés ; l'absence de texte ne prouve rien. → **Assisté**.

## 7. Conclusion par test RGAA

« Brique » = ce que la chaîne média ajoute. Les tests marqués *hors chaîne* relèvent du DOM ou du clavier (autres tickets de #330).

| Test | Niveau atteignable | Brique | Justification courte |
|---|---|---|---|
| 4.1.1 | Assisté | ffprobe (classe audio seul) + sonde DOM `probe-4-1` | « si nécessaire » et « clairement identifiable » demandent un jugement. |
| 4.1.2 | Assisté | ffprobe (pas de piste audio = vidéo seule) + DOM | Idem ; l'existence d'une alternative se repère dans le DOM. |
| 4.1.3 | Assisté | ffprobe (piste audio `descriptions`/`visual_impaired`) + DOM | Une disposition positive prouve l'AD ; son absence n'est pas concluante (§2). |
| 4.2.1 | Assisté | whisper.cpp vs transcription fournie | Indice de recouvrement ; WER de 8 à 15 % en français. |
| 4.2.2 | Irréductible dans cette chaîne | — | Vidéo sans son : Whisper n'apporte rien. Un VLM pourrait le rendre Assisté (autre ticket). |
| 4.2.3 | Assisté | whisper.cpp vs transcription / AD | Indice partiel : couvre la parole, pas l'information visuelle. |
| 4.3.1 | Assisté (pass déterministe possible si une piste est trouvée — à trancher) | DOM `<track>` + ffprobe (pistes de sous-titres) + Tesseract | La présence est vérifiable ; l'absence n'est pas concluante (incrustés, version alternative). |
| 4.3.2 | Automatique (DOM, sans sidecar) | sonde DOM / axe | Attribut `kind="captions"` sur `<track>`. |
| 4.4.1 | Assisté | whisper.cpp vs WebVTT/SRT aligné | Recouvrement et décalage ; la pertinence reste un jugement. |
| 4.5.1 | Assisté | ffprobe (dispositions) + DOM | Comme 4.1.3. |
| 4.5.2 | Assisté | ffprobe (dispositions) + DOM | Comme 4.1.3. |
| 4.6.1 | Irréductible (jugement) — preuve : inventaire | — | Juger une audiodescription demande de voir l'image. |
| 4.6.2 | Assisté faible | whisper sur la piste AD (texte de l'AD comme preuve) | Le texte de l'AD aide l'auditeur, sans donner d'indice de pertinence. |
| 4.7.1 | Assisté | DOM (texte adjacent) ; tag `title` via ffprobe | Hors du cœur de la chaîne média. |
| 4.8.1 / 4.8.2 | Assisté (hors chaîne) | DOM | Médias non temporels : pas de fichier à décoder. |
| 4.9.1 | Irréductible (hors chaîne) | — | Équivalence de contenu et de fonctionnalités. |
| 4.10.1 | Assisté, avec `pass` déterministe sur certaines branches | DOM + ffprobe + `silencedetect` | `pass` si ≤ 3 s audibles, pas de piste audio ou `muted` ; le `fail` reste en revue. |
| 4.11.1–4.11.3 | hors chaîne | navigateur / clavier | Dépend de la navigation clavier, absente sous Obscura. |
| 4.12.1 / 4.12.2 | hors chaîne | navigateur / clavier | Idem. |
| 4.13.1 / 4.13.2 | hors chaîne | arbre d'accessibilité | Idem. |
| 13.7.1 | **Automatique** pour `fail` ; `pass` Automatique seulement si le média entier est analysé à la taille rendue, sinon Assisté | ffmpeg (décodage) + module Rust WCAG, IRIS comme oracle CI | Algorithme déterministe, seuil public. Le plafond de 50 Mo peut tronquer l'analyse. |
| 13.7.2 | Assisté (Automatique si un screencast est disponible — non vérifié) | même module sur des captures successives | Dépend d'une séquence de captures Obscura. |
| 13.7.3 | Assisté (idem) | même module + sonde statique `probe-13-7` | Idem. |

## 8. Incertitudes et points non vérifiés

- Build d'IRIS sous Linux et coût de ses dépendances (OpenCV + FFmpeg via vcpkg) : **non testés**.
- Fidélité d'un portage Rust des seuils WCAG face à IRIS : à prouver sur des fixtures (`prototype`).
- Capacité d'Obscura à produire une séquence de captures (13.7.2 / 13.7.3) : **non vérifiée**.
- Temps de transcription whisper.cpp par minute de média sur la machine cible : **non mesuré**.
- WER en français de `large-v3` / `turbo` : **non vérifié**.
- Version de whisper.cpp embarquée par `whisper-rs-sys` 0.15.0 : **non vérifiée**.
- FAQ GPL (programmes séparés) : citée de mémoire, **non relue** (gnu.org a répondu 403 pendant la recherche).
- Signalisation de l'audiodescription dans HLS/DASH et prise en charge par `ffprobe` : **non vérifiées**.
- Pack Tesseract `fra` et qualité de l'OCR en français : non testés.
- Les seuils de calibration pour promouvoir un test Assisté sont hors de ce ticket (carte #183).
