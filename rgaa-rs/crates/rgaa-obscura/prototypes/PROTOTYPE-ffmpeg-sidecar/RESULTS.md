# PROTOTYPE — sidecar ffmpeg : résultats (jetable)

Ticket [#340](https://github.com/jamon8888/Holo-RGAA/issues/340), carte [#330](https://github.com/jamon8888/Holo-RGAA/issues/330).
Lancer : `python3 run.py <dossier de travail>` (ffmpeg ≥ 8, numpy, tesseract). Médias **synthétiques** générés par ffmpeg ; aucun téléchargement.
Machine : 12 cœurs, ffmpeg 8.0.1, tesseract 5 (anglais seulement). Chiffres de la run du 2026-10-12.

## Verdict par question

| Question | Réponse mesurée | Réserve |
|---|---|---|
| Pistes, langues, titres, dispositions via `ffprobe` | Fiable en lecture : 5 pistes listées sur MP4/MKV, 4 sur WebM, ~80–150 ms/fichier | Aller-retour de ce que ffmpeg a lui-même écrit : ne dit rien de la fréquence de ces marqueurs dans les médias réels |
| Audiodescription / SDH | `disposition` **conservée en MP4 et MKV, perdue en WebM** ; `title` **conservé en MKV et WebM, perdu en MP4** → il faut tester les deux | HLS (`CHARACTERISTICS`) et DASH (`Role=description`) **non testés** ; l'absence de marqueur ne prouve rien |
| Extraction des sous-titres | WebVTT non vide sur MP4 (`mov_text`), MKV (`srt`), WebM (`webvtt`) : ~150–165 ms | Sous-titres de test courts, texte propre |
| Octets à lire pour l'inventaire des pistes | **128–212 Ko sur 18,1 Mo (0,7–1,2 %)** ; 1 requête (faststart) ou 3 (moov en fin) ; `-probesize` réduit **sans effet** | Boucle locale ; la latence réseau réelle domine, surtout avec `moov` en fin |
| Octets pour silence / flashs | **100 %** du fichier (le flux entier est lu) | idem |
| 4.10 durée de son | `silencedetect` : « pas de piste » et « ≤ 3 s audible » = `pass` déterministe ; > 3 s = `review` ; ~90–170 ms pour 10 s | Seulement des sinus purs ; musique de fond, bruit faible, seuil −50 dB non éprouvés |
| 13.7 flashs | Détecteur numpy (~60 lignes) : **10/10 cas justes** dont les bornes (3 vs 4 flashs/s ; 144² = 20 736 px pass / 152² = 23 104 px fail ; rampe lente = pass). **~13× temps réel en 720p ≈ 4,6 s de temps mur par minute de vidéo** | Voir « Défaut trouvé » ; synthétique ; non validé contre IRIS ni du contenu réel |
| Texte incrusté (OCR) | `tesseract` : texte anglais net lu **exactement** (~150 ms image + ~330 ms OCR) | Français **non mesuré** (pack `fra` absent) ; fonds complexes non testés |

## Défaut trouvé pendant le prototype (à retenir pour l'implémentation)

Ma première version du détecteur comptait **chaque pas d'image** dont la luminance variait de ≥ 10 %. Une rampe continue (`128+127·sin`) était alors rendue en **échec sur 230 400 px** alors qu'elle ne contient qu'environ un flash par seconde. Un fondu ou un dégradé animé aurait produit un faux `fail`. Corrigé en suivant les **extrema avec hystérésis** (on ne compte que les inversions de direction ≥ 0,10, état sombre < 0,80). Seul un test **aux bornes** l'a révélé : les cas extrêmes passaient tous.

## Artefacts de mesure repérés et corrigés (ne pas reproduire)

- Compter les octets *après* l'écriture complète sous-estimait ce que lit `ffprobe` (il ferme la connexion en route) : « 0 Ko ».
- Ne pas limiter le tampon d'envoi du serveur le surestimait (2,5–4 Mo, tampon du noyau) : il faut `SO_SNDBUF` minuscule. Conséquence : les **durées** de la section 7 sont faussées par ce tampon et ne sont pas retenues.
- « Durée audible » d'un média sans piste audio = durée totale : afficher « aucune piste ».

## Non mesuré

Whisper / transcription (absent), IRIS (non compilé), OCR français, HLS/DASH, médias réels, 4K ou haut débit, vidéos > 25 i/s (suréchantillonnage à 25 i/s : aliasing possible d'un flash à haute fréquence), réseau réel, détection du rouge saturé autrement qu'en version simplifiée.
