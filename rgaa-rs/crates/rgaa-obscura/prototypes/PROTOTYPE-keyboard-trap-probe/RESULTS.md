# PROTOTYPE (jetable) — sonde clavier 12.9 : verdict

Question : une sonde comportementale « piège clavier » est-elle fiable et assez bon marché sous Obscura ?
Environnement : obscura 0.2.2, Chrome headless (référence), 9 pages de test, 3 passes, 25 Tab/page.

## 1. Sonde « vrais Tab » (`probe.py`, CDP `Input.dispatchKeyEvent`)
- **Obscura ne fait pas la navigation Tab native** : le `keydown` est livré (3 Tab -> 3 événements) mais `document.activeElement` ne bouge jamais. 4 pages sans piège donnent `body > body > …` (indéterminé), et les trois pièges JS sont indiscernables d'une page à un seul élément focusable. Faux positif sur `trap-escapable`.
- **Chromium** : Tab natif correct ; 7/8 verdicts justes ; deux défauts propres à la règle « N Tab sur le même élément » : faux positif sur la sortie par Échap (`trap-escapable`) et piège asynchrone raté (`trap-refocus`, course avec l'échantillonnage).
- Verdict : **non viable sous Obscura**.

## 2. Sonde « keydown annulable » (`probe_dp.py`, indépendante du moteur)
Envoie un `keydown` Tab annulable sur l'élément focalisé, lit `defaultPrevented`, puis essaie Échap pour la sortie. Variante `sweep` : donne le focus à chaque focusable à tour de rôle.
- **Verdicts identiques sur Obscura et Chromium** (9 pages, balayage) : pièges `keydown` détectés, y compris sans `autofocus` ; sorties par Échap reconnues (pas de faux `Fail`) ; pages sans piège `ok`.
- **Raté connu** : piège par refocus asynchrone (`trap-refocus`) -> `ok`. Ne peut donc jamais fonder un `Pass` du critère : seulement un `Fail` ou un « aucun piège par gestionnaire de touche détecté ».
- **Coût** : ~420-450 ms/page dont 400 ms d'attente de chargement fixe ; l'évaluation elle-même est négligeable (1 évaluation JS, pas de boucle de Tab).

## Limites du prototype
Fixtures synthétiques (9), pas de pages réelles ; pas d'iframes ; Échap seul comme touche de sortie (RGAA admet d'autres touches documentées).
