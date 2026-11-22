# Bibliothèques retenues — veille GitHub et crates.io

> Relevé du **2026-11-21** : API GitHub (`gh api repos/…`, `gh search repos`) et API crates.io. ★ = étoiles GitHub ; « push » = dernier push sur le dépôt ; « téléch. » = téléchargements crates.io sur 90 jours.
> À refaire avant chaque jalon. Une bibliothèque sans push depuis 12 mois passe en « à surveiller ».

## 1. Retenues

| Domaine | Bibliothèque | Forme | Version | ★ | Dernier push | Licence | Téléch. 90 j | Rôle |
|---------|--------------|-------|---------|---|--------------|---------|--------------|------|
| Couleur | [`palette`](https://github.com/Ogeon/palette) | natif | 0.7.7 | 840 | 2026-11-01 | Apache-2.0/MIT | 28 M | luminance WCAG, Lab, ΔE2000 — P-01, P-02 |
| Image | [`image`](https://github.com/image-rs/image) | natif | 0.25.10 | 5 916 | 2026-11-11 | Apache-2.0/MIT | 87 M | pixels, GIF animés — P-01/02/03/05 |
| Image | [`imageproc`](https://github.com/image-rs/imageproc) | natif | 0.27.0 | 990 | 2026-10-31 | MIT | 3,2 M | Otsu, composantes connexes, différence d'images |
| Hachage perceptuel | [`image_hasher`](https://github.com/qarmin/img_hash) (fork maintenu de `img_hash`) | natif | 3.1.1 | 50 | 2026-02-21 | Apache-2.0/MIT | 0,6 M | doublons d'images, cache de décisions — P-02/03 |
| SVG | [`resvg`](https://github.com/linebender/resvg) | natif | 0.50.0 | 4 207 | 2026-11-21 | Apache-2.0/MIT | 18 M | rasterisation SVG — P-03 |
| EXIF | [`kamadak-exif`](https://github.com/kamadak/exif-rs) | natif | 0.6.2 | 261 | 2026-11-05 | BSD-2 | 5,2 M | métadonnées d'image — P-03 |
| **OCR (principal)** | [`oar-ocr`](https://github.com/GreatV/oar-ocr) | natif (`ort` + modèles PaddleOCR) | 0.11.0 | 210 | 2026-11-18 | Apache-2.0 | 0,1 M | OCR multilingue (latin dont français), mise en page — P-03 |
| OCR (léger) | [`ocrs`](https://github.com/robertknight/ocrs) + [`rten`](https://github.com/robertknight/rten) | natif, pur Rust | 0.14.0 / 0.28.0 | 1 922 / 372 | 2026-11-08 / 2026-11-21 | Apache-2.0/MIT | 0,4 M | sans ONNX Runtime ; anglais surtout — P-03 |
| OCR (repli) | [`tesseract`](https://github.com/tesseract-ocr/tesseract) ≥ 5 | **sidecar** (binaire) | 5.x | 77 612 | 2026-11-20 | Apache-2.0 | — | 100+ langues, si les deux autres échouent — P-03 |
| Vision locale | [`fastembed-rs`](https://github.com/Anush008/fastembed-rs) | natif (`ort`) | 7.7.1 (workspace en 4.0 : montée à prévoir) | 1 052 | 2026-11-17 | Apache-2.0 | 2,9 M | CLIP image/texte, plongements texte multilingues — P-03, P-08 |
| Runtime ONNX | [`ort`](https://github.com/pykeio/ort) | natif | 2.x (ONNX Runtime 1.31) | 2 626 | 2026-11-18 | Apache-2.0/MIT | 10,8 M | commun à `oar-ocr`, `fastembed`, `sherpa-onnx` : **une seule version** à épingler dans le workspace |
| Langue | [`lingua-rs`](https://github.com/pemistahl/lingua-rs) | natif | 1.8.0 | 1 154 | 2026-11-20 | Apache-2.0 | 1,0 M | détection de langue, textes courts — P-06 |
| Similarité de chaînes | [`strsim`](https://github.com/rapidfuzz/strsim-rs) | natif | 0.11.1 | 495 | 2025-11-27 | MIT | 296 M | Levenshtein, Jaro-Winkler — P-08 (stable, peu de commits : normal) |
| Décodage audio | [`Symphonia`](https://github.com/pdeljanov/Symphonia) | natif, pur Rust | 0.6.1 | 3 491 | 2026-11-08 | MPL-2.0 | 12 M | mp3, aac, ogg, opus, flac, wav — P-04 |
| Rééchantillonnage | [`rubato`](https://github.com/HEnquist/rubato) | natif | 5.1.0 | 374 | 2026-11-12 | MIT | 6,3 M | passage en 16 kHz mono — P-04 |
| **Parole (VAD + ASR)** | [`sherpa-onnx`](https://github.com/k2-fsa/sherpa-onnx) (crate Rust officiel) | natif (FFI vers la bibliothèque C) | 2.0.2 | 16 079 | 2026-11-21 | Apache-2.0 | 0,6 M | VAD Silero **et** transcription (modèles Whisper et autres modèles ONNX) dans une seule dépendance — P-04 |
| ASR (alternative) | [`whisper-rs`](https://codeberg.org/tazz4843/whisper-rs) | natif (whisper.cpp lié) | 0.16.0 | — (dépôt GitHub **archivé** 2025-07, déplacé sur Codeberg) | 2026-03-15 | Unlicense | 1,2 M | si `sherpa-onnx` ne convient pas — P-04 |
| ASR (sidecar) | [`whisper.cpp`](https://github.com/ggml-org/whisper.cpp) `whisper-server` | **sidecar** | — | 55 399 | 2026-11-20 | MIT | — | transcription GPU/CPU hors processus — P-04 |
| Sous-titres VTT | [`webvtt-parser`](https://github.com/dmtrKovalenko/fframes) | natif | 1.4.1 | 3 121 (monorepo fframes) | 2026-11-15 | MIT | 19 k | lecture WebVTT — P-04 |
| Sync sous-titres | [`alass`](https://github.com/kaegi/alass) | **sidecar** (GPL-3.0 : jamais lié) | 2.0 | 1 469 | 2023-12-28 | GPL-3.0 | — | décalage audio/sous-titres indépendant de la langue, recoupe la mesure de P-04 §3.5 |
| Intensité sonore | [`ebur128`](https://github.com/sdroege/ebur128) | natif | 0.1.11 | 135 | 2026-10-27 | MIT | 0,4 M | LUFS EBU R128 — P-04 (4.10) |
| Vidéo | [`ffmpeg-sidecar`](https://github.com/nathanbabcock/ffmpeg-sidecar) + binaire `ffmpeg` | **sidecar** | 2.7.0 | 547 | 2026-11-04 | MIT (ffmpeg : LGPL/GPL selon le build, jamais lié) | 0,7 M | extraction d'images, formats hors Symphonia — P-04, P-05 |
| Flashs (référence) | [`IRIS`](https://github.com/electronicarts/IRIS) (Electronic Arts) | **sidecar** (C++) | — | 241 | 2025-01-14 (à surveiller) | BSD-3 | — | analyse photosensible (luminance et rouge) : **oracle** des tests de P-05 et deuxième avis optionnel sur les vidéos |

## 2. Écartées

| Bibliothèque | Raison |
|--------------|--------|
| `subparse` (SRT/ASS) | dernier push 2022, version 2021. Le SRT se lit en ~40 lignes maison ; on garde `webvtt-parser` pour le VTT |
| `voice_activity_detector` | 64 ★, dernier push 2025-08. `sherpa-onnx` fournit le même modèle Silero et il est maintenu |
| `whisper-rs` comme **premier** choix | dépôt GitHub archivé, migration Codeberg, rythme ralenti : il passe en alternative |
| `colorblind`, `color_blinder`, `colorverse`, `bevy_color_blindness` | 0 à 50 ★, abandonnées ou liées à un moteur de jeu. Les matrices Machado/Brettel sont implémentées sur `palette` (P-02) |
| `wcag-contrast`, `perceive-color`, `apcach-rs` | 0 à 4 ★, mainteneur unique, quelques mois d'existence. La formule WCAG est écrite sur `palette` (P-01) |
| `leptess`, `tesseract` (crate) | liaison C++ et Leptonica à compiler. Le binaire `tesseract` en sidecar suffit |
| `kalosm-sound` | dernier push 2025-02, périmètre trop large |
| `deepseek-ocr.rs`, `ocr-rs` (MNN) | modèles lourds ou runtime supplémentaire (MNN). `oar-ocr` couvre le besoin sur `ort`, déjà présent |
| `candle` | excellent (21 k ★), mais aucun plugin n'en a besoin tant que les modèles existent en ONNX. Réserve pour un futur modèle de légende d'image local |

## 3. Points d'attention

1. **Un seul ONNX Runtime (décision du 2026-11-22).** `sherpa-onnx` embarque son propre ONNX Runtime en C++ : il n'est donc **pas lié** et reste une option sidecar. La VAD Silero tourne sur l'`ort` du workspace, et la transcription passe par `whisper-rs` (ggml, sans ONNX) ou par le sidecar `whisper.cpp`. **Une seule version d'`ort`.** `oar-ocr`, `fastembed` et `sherpa-onnx` (qui embarque son propre runtime ONNX en C) peuvent tirer des runtimes différents. Le ticket P-00 vérifie avec `cargo tree -d` qu'un seul `ort` est compilé. Il mesure aussi la taille du binaire avec `plugins-all`.
2. **Montée de `fastembed` de 4.0 à 7.7** dans le workspace : elle touche `rgaa-agent` (RAG). C'est un ticket séparé, à faire avant P-03 et P-08.
3. **GPL.** `alass` et certains builds de `ffmpeg` sont en GPL. Ils ne sont **jamais liés** : uniquement en sidecar, installés par le client, et la licence est affichée par `rgaa plugins list`.
4. **Précision OCR en français.** Le banc d'essai de P-03 §2 compare `oar-ocr`, `ocrs` et `tesseract` sur 30 images du corpus. Celui qui fait le moins d'erreurs de caractères devient le défaut.
