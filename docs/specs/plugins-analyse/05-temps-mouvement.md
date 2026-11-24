# P-05 — Plugin `motion` (flashs, mouvements, limites de temps)

**Critères** : 13.7 (changements brusques de luminosité ou effets de flash), 13.1 (contrôle des limites de temps). Appui à 13.8 (contenu en mouvement ou clignotant contrôlable).
**Dépend de** : P-00, P-04 (accès `ffmpeg`) · **Modèle** : aucun · **Feature** : `plugin-motion` · **Forme** : natif, avec un sidecar `ffmpeg` pour les vidéos

## 1. Problème

13.7 est à moteur `Human`. Le plan indique « analyse de luminance par frames ». Aucun crate Rust ne fournit d'analyse photosensible. Les outils de référence, PEAT (Trace Center) et Harding FPA, sont des applications de bureau propriétaires. Les seuils WCAG 2.1 (2.3.1) sont publics et calculables : on les implémente. 13.1 se détecte en observant la page dans le temps.

## 2. Composants

| Étape | Natif | Sidecar |
|-------|-------|---------|
| Images de la **page animée** (CSS, GIF, canvas, carrousels) | CDP `Page.startScreencast` (Obscura) ou captures à 10 i/s pendant `opts.duree_s` (défaut 10 s) | — |
| Images d'une **vidéo** | — | `ffmpeg` via `ffmpeg-sidecar` : `-vf fps=30,scale=1024:-1` |
| GIF animé | `image` 0.25 (`GifDecoder`, images et délais) | — |
| Calcul de luminance et de flashs | maison, sur `image` + `palette` | **`IRIS`** (Electronic Arts, C++, BSD-3) : deuxième avis optionnel sur les vidéos, et **oracle** des tests |

## 3. Algorithme 13.7 (WCAG 2.3.1, définitions de « general flash » et « red flash »)

On normalise chaque image à **1024×768** de référence. La zone de 10° de champ visuel vaut **341×256 px**, et le seuil de surface est de **25 % de cette zone**.

Pour chaque image *i* et chaque pixel, on calcule la luminance relative L (sRGB linéaire, P-01).

**General flash.** Une transition est une paire de changements opposés de L ≥ 0,10, où la plus sombre des deux valeurs est < 0,80. On compte les transitions par fenêtre glissante de 341×256 px, avec un pas de 64 px. Un **flash** est une paire de transitions opposées. On a un **échec probable** dès que plus de 3 flashs tombent dans **une seconde quelconque**, sur une même fenêtre, à condition que la surface concernée dépasse 25 % de la fenêtre.

**Red flash.** On retient les pixels « rouge saturé », pour lesquels `R / (R + G + B) ≥ 0,8`. Une transition est une variation de `(R − G − B) × 320` d'au moins 20 (valeur sRGB 0 à 255), selon la définition W3C. Le comptage et le seuil de surface sont ceux du general flash.

Le plugin produit une `Preuve` (`Estimee`). Ce n'est pas `Mesuree` : la surface réelle dépend de la distance de vision, et le relecteur conclut. Elle contient :

- le nombre maximal de flashs par seconde ;
- l'horodatage ;
- la fenêtre concernée ;
- le type (général ou rouge) ;
- un artefact : une bande de 8 vignettes autour du pic, et la courbe L(t) en SVG.

## 4. Algorithme 13.8 (appui)

**Détection.**

- On compare les images successives par différence absolue (`imageproc`).
- Une région qui bouge pendant plus de 5 s (carrousel, défilement, `marquee`, GIF, animation CSS `infinite`) est relevée.

**Contrôle.**

- On regarde dans l'arbre AX la présence d'un contrôle pause, arrêt ou masquage proche : rôle `button`, nom contenant « pause », « arrêter » ou « stop ».
- On teste `prefers-reduced-motion: reduce` avec `Emulation.setEmulatedMedia` : l'animation s'arrête-t-elle ?

**Indice.**

- Mouvement > 5 s, sans contrôle ni respect de `reduced-motion` : `Suspect`.
- Animation arrêtée par `reduced-motion` : `Rassurant`. C'est une bonne pratique, mais elle ne remplace pas le contrôle exigé par le RGAA.

## 5. Algorithme 13.1

- **Statique** (`Mesuree`) : `meta http-equiv="refresh"` avec un délai > 0 ; `setTimeout` ou `setInterval` qui appelle `location` ou `form.submit`. La détection se fait par instrumentation CDP `Debugger` ou par un script injecté qui *wrappe* `setTimeout` et `location.assign`.
- **Dynamique** (`Estimee`) : on observe la page pendant `opts.duree_s`. Une navigation ou un remplacement de contenu non initié par l'utilisateur est relevé, avec son délai. On cherche ensuite un mécanisme pour arrêter ou prolonger : boîte de dialogue « prolonger la session » repérée par AX.
- Le LLM n'est pas appelé.

## 6. `mesures`

```json
{ "source": "page|video:https://…|gif:https://…", "fps_analyse": 30,
  "flash_general": { "max_par_s": 5, "a_s": 3.2, "fenetre": [256, 128, 341, 256], "surface": 0.41 },
  "flash_rouge": { "max_par_s": 0 },
  "mouvement": [{ "selecteur": ".carousel", "duree_s": 10, "controle_pause": false, "reduced_motion_respecte": false }],
  "limites_temps": [{ "type": "meta-refresh", "delai_s": 30, "mecanisme_prolongation": false }] }
```

## 7. Tests

| Cas | Attendu |
|-----|---------|
| GIF noir/blanc plein écran à 5 Hz (généré) | general flash, 5 par s, surface 1,0 |
| Même GIF à 2 Hz | 2 par s, sous le seuil |
| GIF à 5 Hz sur 100×100 px | surface < 25 % de la fenêtre, sous le seuil |
| Alternance rouge saturé / noir à 4 Hz | red flash détecté |
| Carrousel automatique sans bouton pause | 13.8 `Suspect` |
| `<meta http-equiv="refresh" content="30">` | 13.1, `Mesuree`, délai 30 s |

Les fixtures sont générées par code de test, sans fichier vidéo réel. Elles sont aussi passées une fois dans le sidecar **IRIS**, qui sert d'oracle indépendant. On note l'écart entre notre comptage et celui d'IRIS. Le dépôt IRIS n'a pas reçu de push depuis 2025-01 : il est « à surveiller », d'où son rôle d'oracle et non de moteur principal.

## 8. Critères d'acceptation

1. Les 6 cas sont verts. Une vidéo de 60 s à 30 i/s (1024×768) est traitée en moins de 20 s (`--release`, hors décodage ffmpeg).
2. L'observation de la page dure 10 s par défaut et se règle par `opts.duree_s`. Elle n'est lancée que si 13.x est dans le périmètre de l'audit.
3. Le rapport porte l'avertissement « analyse indicative, la surface réelle dépend de la distance de vision ». 13.7 reste `review`.
