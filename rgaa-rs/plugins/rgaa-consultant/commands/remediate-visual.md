---
description: Captures RGAA par critère et propositions de remédiation de l'IA rendues en direct (Playwright/Chromium), avec export pour l'extension Chrome
argument-hint: [url]
---

# `/remediate-visual` — Preuves visuelles et propositions par critère

Active la skill `visual-remediation` sur une URL : capture de l'existant par critère, 2 à 3
propositions de remédiation par finding rendues en direct, galerie `index.html`.

## Usage

```
/remediate-visual [url]
```

Sans `url`, réutiliser le dernier audit disponible (`get_audit_result` ou `.rgaa/`).

## Sorties

- `.rgaa/visual/index.html` — galerie comparative ;
- `.rgaa/visual/input.json` — findings et propositions : c'est aussi le fichier à ouvrir dans
  l'extension Chrome `rgaa-chrome` (panneau latéral) pour passer en revue les critères un par un
  et prévisualiser chaque proposition dans l'onglet.

Aucune écriture dans le code source : l'application d'une proposition passe par `remediate`
(approbation), puis `verify`.
