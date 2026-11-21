# Plugins d'analyse — index des specs

> Statut : **spec prête à implémenter** · 2026-11-21 · remplace la piste *Calibration* (abandonnée, voir §4).
> Vocabulaire : `GLOSSARY.md` (*Plugin d'analyse*, *Preuve*, *Assisté*, *Famille de preuve*).

## 1. Le principe en une phrase

Pour un critère complexe, une **bibliothèque Rust mesure** d'abord (contraste, OCR, simulation de daltonisme, transcription, détection de langue…). Le modèle Mistral Large 4 ne reçoit ensuite que des **preuves déjà calculées** et ne juge que la *pertinence*. Le **relecteur humain** décide. Chaque capacité est livrée comme un **plugin d'analyse** indépendant, activable par une *feature* Cargo et exposé de la même façon dans le CLI, le MCP et les plugins agents.

```
page (Obscura/CDP) ──► Plugin d'analyse (Rust, local, sans LLM) ──► Preuve (mesures + artefacts)
                                                                     │
                                       Mistral Large 4 (pertinence) ◄┘  (seulement si le test le demande)
                                                                     │
                                   Page de revue / plugin agent ◄────┘  ──► Décision de revue (humain)
```

On en tire trois avantages :

1. **Coût.** Le LLM ne voit plus la page brute. Il reçoit un extrait OCR, un score de similarité et une vignette, soit moins de tokens par test. Voir PRD §8.2.
2. **Souveraineté.** Les mesures tournent sur le poste du client, comme les audits (décision D4 du PRD v3). Aucun appel réseau n'est fait en dehors de la passerelle LLM.
3. **Opposabilité.** Une mesure (ratio 3,1:1, transcription horodatée) est reproductible et versionnée. L'avis d'un modèle ne l'est pas.

**Rust d'abord, sidecar sinon.** Un plugin est **natif** (crate Rust) quand une bibliothèque Rust mûre existe. Sinon c'est un **sidecar** : un exécutable externe (ffmpeg, tesseract, whisper.cpp server, script Python…) piloté en JSON sur stdin/stdout, selon le protocole de P-00 §7 bis. L'orchestrateur ne fait pas la différence entre les deux.

| Capacité | Natif (Rust) | Sidecar |
|----------|--------------|---------|
| Contraste, daltonisme, langue, structure | oui (`palette`, `image`, `lingua`, CDP) | — |
| OCR | `oar-ocr` (PaddleOCR multilingue), `ocrs` (pur Rust, léger) | `tesseract` ≥ 5 (100+ langues) |
| Classification et similarité d'image | `fastembed` (CLIP) | — |
| Décodage audio | `symphonia` | `ffmpeg` (formats non couverts, vidéo) |
| Extraction d'images vidéo | — | `ffmpeg` via `ffmpeg-sidecar` |
| Parole (VAD + transcription) | `sherpa-onnx` (Silero + Whisper ONNX) ; `whisper-rs` en alternative | `whisper.cpp` server, ou Voxtral (Mistral) via la passerelle |
| Synchronisation des sous-titres | mesure maison | `alass` (GPL, jamais lié) |
| Flashs | algorithme WCAG maison | `IRIS` (Electronic Arts) comme oracle et deuxième avis |

Versions, étoiles, dernier push, licences et bibliothèques écartées : **[bibliotheques.md](bibliotheques.md)**, relevé GitHub et crates.io du 2026-11-21.

## 2. Tickets

| # | Spec | Critères RGAA servis | Bibliothèques principales | Dépend de |
|---|------|----------------------|---------------------------|-----------|
| P-00 | [Contrat de plugin d'analyse](00-contrat-plugin.md) | — (socle) | `image`, `serde`, `sha2` | — |
| P-01 | [Contraste](01-contraste.md) | 3.2, 3.3 (renforce), 10.? états survol/focus | `palette`, `image`, `imageproc` | P-00 |
| P-02 | [Vision des couleurs (daltonisme)](02-vision-couleurs.md) | 3.1, appui 3.3 et 10.9 | `palette`, `image`, `image_hasher` | P-00 |
| P-03 | [Images et alternatives](03-images-alternatives.md) | 1.2, 1.3, 1.4, 1.7, 1.8 | `oar-ocr` / `ocrs` / sidecar `tesseract`, `fastembed` (CLIP), `image_hasher`, `resvg`, `kamadak-exif` | P-00 |
| P-04 | [Médias temporels (audio et vidéo)](04-medias-audio-video.md) | 4.2, 4.4, 4.6, 4.9, appui 4.1, 4.3, 4.5, 4.10 | `symphonia`, `rubato`, `sherpa-onnx`, `webvtt-parser`, `ebur128` ; sidecars `ffmpeg`, `alass`, `whisper.cpp` | P-00 |
| P-05 | [Temps et mouvement (flashs, limites de temps)](05-temps-mouvement.md) | 13.1, 13.7, appui 13.8 | `image` (algorithme maison) ; sidecars `ffmpeg`, `IRIS` | P-00, P-04 (extraction d'images) |
| P-06 | [Langue](06-langue.md) | 8.4, 8.8, appui 8.3, 8.7 | `lingua` | P-00 |
| P-07 | [Structure visuelle et accessibilité exposée](07-structure-visuelle.md) | 5.3, 5.6, 7.1, 7.5, 9.1, 9.2, 10.3, 10.9, 10.10 | CDP (`Accessibility`, `DOMSnapshot`), `imageproc` | P-00 |
| P-08 | [Pertinence textuelle](08-pertinence-textuelle.md) | 1.5, 2.2, 5.2, 5.5, 6.1, 7.2, 8.6, 11.2, 11.7, 11.9 à 11.13, 12.3, 13.4, 13.6 | `fastembed` (texte), `strsim`, `lingua` | P-00, P-06 |
| P-09 | [Plugins agents et page de revue](09-plugins-agents-revue.md) | tous (présentation) | — (skills Claude Code / Codex, page de revue) | P-00, au moins un plugin |

Ordre conseillé : **P-00 → P-01 → P-02** (ce sont les plus rapides et ils ne dépendent d'aucun modèle), puis **P-03**, **P-06**, **P-08**, **P-07**, **P-04**, **P-05**, et **P-09** au fil de l'eau.

## 3. Couverture des 40 critères à moteur Holo ou Humain

Tous les critères dont le moteur principal (`engine_plan.json`) est `Holo` ou `Human` ont un plugin propriétaire :

| Plugin | Critères |
|--------|----------|
| P-02 | 3.1 |
| P-03 | 1.2, 1.3, 1.4, 1.7, 1.8 |
| P-04 | 4.2, 4.4, 4.6, 4.9 |
| P-05 | 13.1, 13.7 |
| P-06 | 8.4, 8.8 |
| P-07 | 5.3, 5.6, 7.1, 7.5, 9.1, 9.2, 10.3, 10.9, 10.10 |
| P-08 | 1.5, 2.2, 5.2, 5.5, 6.1, 7.2, 8.6, 11.2, 11.7, 11.9, 11.10, 11.11, 11.12, 11.13, 12.3, 13.4, 13.6 |

Total : 1 + 5 + 4 + 2 + 2 + 9 + 17 = **40**. P-01 renforce 3.2 et 3.3, dont le moteur reste `Deterministic` : il remplace le seul `color-contrast` d'axe sur les cas que celui-ci ne mesure pas (fond image, dégradé, composant non textuel).

## 4. Abandon de la calibration — ce qui change

- Décision du porteur (2026-11-21) : **pas de calibration** avant le pilote.
- Ce qui reste vrai (GLOSSARY, *Assisté*) : un test Assisté rend toujours `review`. Le niveau Assisté ne dépend pas de la calibration. **Les règles R1 à R7 du PRD sont inchangées.** C'est la revue humaine qui porte la fiabilité, pas un score d'accord.
- Conséquences sur la carte : le ticket *Calibrer Mistral Large 4* (#415) est fermé hors périmètre. *Admettre ou non Mistral Large 4* (#406) est tranché : **admis**, sur la seule mesure coût et latence (#414).
- Ce qu'on perd : on ne peut pas écrire « l'IA est d'accord à X % avec un auditeur ». Le discours commercial doit s'en tenir à « mesures reproductibles + revue humaine systématique ».

## 5. Règles communes à tous les plugins (rappel PRD §7)

- **R-P1.** Un plugin d'analyse produit des **preuves**, jamais un statut (`pass`/`fail`/`review`). C'est le moteur de règles existant (`rgaa-orchestrator`) qui décide du statut à partir des preuves (R7).
- **R-P2.** Une preuve est **`mesurée`** (calcul exact sur une entrée fiable, ex. couleur CSS sur fond uni) ou **`estimée`** (échantillonnage de pixels, OCR, modèle). Seule une preuve `mesurée` peut alimenter un test Automatique. Une preuve `estimée` mène toujours à `review`.
- **R-P3.** Aucun plugin n'appelle le réseau. Les modèles locaux (ONNX, ggml) se téléchargent une fois, par `rgaa models pull`, et sont vérifiés par empreinte SHA-256 (P-00 §6).
- **R-P4.** Chaque preuve porte `plugin`, `version_plugin`, `version_modele` (s'il y a un modèle) et l'empreinte de l'entrée. Le même rapport, rejoué, donne la même preuve.
