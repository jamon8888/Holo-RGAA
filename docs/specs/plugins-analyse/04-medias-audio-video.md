# P-04 — Plugin `media` (médias temporels : audio et vidéo)

**Critères** : 4.2 (transcription pertinente), 4.4 (sous-titres pertinents), 4.6 (audiodescription pertinente), 4.9 (alternative pertinente). Appui à 4.1 (transcription présente si nécessaire), 4.3 (sous-titres synchronisés), 4.5 (audiodescription présente) et 4.10 (son déclenché automatiquement contrôlable).
**Dépend de** : P-00 · **Modèles** : VAD Silero, Whisper · **Feature** : `plugin-media` · **Forme** : natif + sidecars

## 1. Problème

Aujourd'hui, 4.2, 4.4 et 4.6 sont à moteur `Human`. L'auditeur écoute la vidéo en entier et compare à l'oreille. La machine peut faire le gros du travail :

- savoir s'il y a de la **parole** (une transcription est-elle « nécessaire » ?) ;
- **transcrire** ;
- **aligner** la transcription et les sous-titres fournis ;
- mesurer les **décalages** ;
- repérer les **passages non couverts**.

Le relecteur n'écoute alors que les passages signalés.

## 2. Composants (Rust d'abord, sidecar sinon)

| Étape | Natif (Rust) | Sidecar | Choix par défaut |
|-------|--------------|---------|------------------|
| Récupération du média | `ResourceFetcher` (P-00), même origine et cookies | — | natif |
| Démultiplexage et décodage audio (mp3, aac/mp4, ogg/vorbis, opus, flac, wav) | `symphonia` 0.6 | `ffmpeg` (webm, hls/m3u8, codecs absents de symphonia) | symphonia, puis ffmpeg en repli automatique |
| Rééchantillonnage en 16 kHz mono | `rubato` 5.1 | inclus dans `ffmpeg -ar 16000 -ac 1` | natif |
| Détection de parole (VAD) | `sherpa-onnx` 2.0 (Silero VAD) | — | natif |
| Transcription horodatée | `sherpa-onnx` 2.0 (modèles Whisper ONNX) ; alternative `whisper-rs` 0.16 (dépôt déplacé sur Codeberg) | `whisper.cpp` `whisper-server` (HTTP local, GPU possible) ; ou **Voxtral** (Mistral) via la passerelle LLM, *Offre Confort* uniquement | `sherpa-onnx` ; modèle à fixer par la mesure du §2 |
| Lecture des sous-titres | `webvtt-parser` 1.4 (VTT) ; SRT lu par un parseur maison (~40 lignes, `subparse` est abandonné) | — | natif |
| Synchronisation des sous-titres (recoupement) | — | `alass` (GPL-3.0, donc jamais lié) : décalage optimal audio/sous-titres | optionnel |
| Intensité sonore (4.10) | `ebur128` 0.1 (EBU R128, LUFS court terme) | — | natif |
| Images vidéo (4.4, 4.6, P-05) | — | `ffmpeg` via `ffmpeg-sidecar` 2.7 (binaire externe, sans liaison) | sidecar |

**Points à trancher dans le ticket, avec mesure** :

- **Moteur et modèle de transcription.** On compare `sherpa-onnx` et `whisper-rs`, avec les modèles Whisper `small` et `large-v3-turbo`, sur 5 fixtures FR. On mesure le taux d'erreur de mots et le facteur temps réel sur CPU 8 cœurs. Le défaut est le plus précis des couples qui tiennent 1× temps réel.
- **Voxtral.** Ses identifiants de modèle, son prix et la nature de son API (transcription horodatée ou non) sont **à vérifier** dans un ticket de recherche avant de l'activer. Comme pour Large 4, l'appel passe par la passerelle de l'instance (refacturé en crédits). Ce n'est donc pas une exception à R-P3 : c'est une étape LLM, pas une étape du plugin.

## 3. Algorithme par média

1. **Inventaire** (`Mesuree`, DOM) :
   - `video`, `audio`, `track[kind=captions|subtitles|descriptions]` ;
   - lecteurs iframe connus (YouTube, Vimeo, Dailymotion, PeerTube) ;
   - attribut `autoplay`, `muted`, contrôles présents ;
   - lien de transcription adjacent (texte « transcription », « version texte » à moins de 2 blocs).
   **Les lecteurs tiers ne sont pas téléchargés** (conditions d'utilisation, DRM) : leurs tests restent `review`, avec la preuve « média tiers, analyse non effectuée ». Seules les pistes `track` accessibles sont lues.
2. **Parole** : VAD sur toute la piste, avec `part_parole` et une liste de segments. Une part de parole inférieure à 2 %, sans musique informative (non détectable) : `indice = Neutre` « média sans parole détectée ; transcription peut-être non nécessaire (4.1) ».
3. **Transcription** : Whisper sur les segments de parole uniquement, avec horodatage par segment et langue détectée. Celle-ci est comparée à `lang` de la page par P-06.
4. **Comparaison avec la transcription fournie** (4.2) :
   - on normalise les deux textes (minuscules, ponctuation retirée, nombres en chiffres) ;
   - **couverture** = part des mots Whisper retrouvés, alignement par distance d'édition sur les mots ;
   - on liste les **passages manquants**, c'est-à-dire les fenêtres de plus de 15 s non couvertes, avec leurs horodatages.
5. **Comparaison avec les sous-titres** (4.3, 4.4) :
   - **couverture** textuelle, calculée comme au point 4 ;
   - **décalage** de synchronisation : médiane et p95 de l'écart entre le début de chaque cue et le segment Whisper aligné ;
   - **passages de parole sans cue** de plus de 3 s.
6. **Audiodescription** (4.5, 4.6) : on détecte une piste `descriptions` ou une version « AD ». Pour 4.6, on ne mesure que :
   - la présence de texte dans les silences de la piste principale ;
   - la couverture des silences de plus de 2 s.
   La pertinence reste humaine, aidée par le LLM.
7. **Son automatique** (4.10) : quand `autoplay` est présent sans `muted`, on calcule la durée pendant laquelle l'intensité dépasse −50 LUFS. Au-delà de 3 s, sans contrôle accessible repéré par l'arbre AX : `Suspect`.
8. **LLM** (Mistral Large 4, texte seul). Pour 4.2, 4.4 et 4.9, on envoie :
   - les passages manquants ou divergents (extrait Whisper face à l'extrait fourni) ;
   - pas le média.
   La question posée est « l'information essentielle est-elle restituée ? ». La réponse va dans `mesures.avis_modele`.

## 4. `mesures`

```json
{ "source": "https://…/video.mp4", "duree_s": 312.4, "part_parole": 0.71, "langue_detectee": "fr",
  "transcription_fournie": { "url": "…", "couverture": 0.83, "passages_manquants": [{ "debut_s": 120.0, "fin_s": 151.5, "extrait_whisper": "…" }] },
  "sous_titres": { "piste": "fr.vtt", "couverture": 0.91, "decalage_median_ms": 420, "decalage_p95_ms": 1800, "parole_sans_cue": [{ "debut_s": 40.2, "fin_s": 45.0 }] },
  "son_auto": { "autoplay": true, "muted": false, "duree_au_dessus_seuil_s": 12.0 },
  "moteur_transcription": "whisper-rs@large-v3-turbo-sha256:…" }
```

**Artefacts** :

- `transcription-whisper.vtt` ;
- `diff-transcription.html` (deux colonnes, passages manquants surlignés) ;
- le cas échéant, une vignette par passage signalé (image extraite par ffmpeg au milieu du passage).

## 5. Tests

| Cas (fixtures courtes, < 30 s, libres de droits, générées par synthèse vocale) | Attendu |
|-------------|---------|
| Audio parlé + transcription complète | couverture ≥ 0,9 |
| Même audio + transcription sans la 2e phrase | passage manquant aux bons horodatages (±1 s) |
| Vidéo + VTT décalé de +2 s | décalage médian ≈ 2000 ms (±300) |
| Musique seule | `part_parole` < 0,02 ; `Neutre` |
| `<video autoplay>` sonore, sans contrôles | `son_auto.duree_au_dessus_seuil_s` > 3, `Suspect` |
| `.webm` (opus) | décodé par symphonia ; sinon repli ffmpeg journalisé |
| iframe YouTube | aucun téléchargement ; preuve « média tiers » |

## 6. Critères d'acceptation

1. Sans `ffmpeg` installé, les formats symphonia passent, et les autres donnent « preuve manquante (ffmpeg absent) ».
2. Pour 10 min d'audio en `--release` sur CPU 8 cœurs, le traitement prend :
   - moins de 10 s pour VAD + décodage ;
   - moins de 10 min pour la transcription avec `small`.
   Le temps mesuré avec `large-v3-turbo` est noté dans le ticket.
3. Les médias de plus de 60 min sont ignorés par défaut (`opts.duree_max_s`). Le test reste en `review`, avec la mention « non analysé : durée ».
4. Aucun média ni aucune transcription n'est envoyé à la passerelle, sauf si Voxtral est explicitement choisi (Offre Confort). Le rapport l'indique.
