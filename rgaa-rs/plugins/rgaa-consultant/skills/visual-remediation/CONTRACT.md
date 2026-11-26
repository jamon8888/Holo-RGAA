# Contrat de données `input.json` (version 1)

Contrat unique entre la skill `visual-remediation`, l'extension `rgaa-chrome` et la conversion de la
réponse `analyze` de `rgaa-mcp-http`. Décisions : ticket « Contrat de données » de la carte #490.

```json
{
  "contract_version": 1,
  "url": "https://site.audite/page",
  "findings": [{
    "id": "fng_abc123", "criterion_id": "1.1", "target": "img.hero",
    "description": "Image porteuse d'information sans alternative",
    "tier": "automatique | assiste | irreductible",
    "guidance": "texte libre du serveur (optionnel)",
    "proposals": [{
      "label": "Alternative descriptive", "rationale": "1.1 — …",
      "attributes": { "alt": "…" }, "css": "…", "html": "…",
      "needs_human_review": true
    }]
  }]
}
```

- `contract_version` : absent = 1 ; une version supérieure à celle que l'outil connaît est refusée.
- `tier` : niveau d'automatisation du test (`CONTEXT.md`). Le serveur le remplit depuis le catalogue
  quand il le fournit ; à défaut, l'extension le déduit du statut (`fail` → automatique,
  `needs_review` → assisté).
- Proposition : `attributes` (`null` = suppression), `css`, `html` sont combinables.
  `needs_human_review` est **obligatoire à `true`** quand la proposition invente un contenu métier
  (texte d'alternative, intitulé de lien) ; l'extension l'affiche comme « à valider ».
- Les propositions sont rédigées par la skill Claude ; le serveur ne fait que servir les findings.
