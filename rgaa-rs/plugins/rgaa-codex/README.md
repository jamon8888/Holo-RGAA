# Plugin RGAA pour Codex

Ce plugin apporte à Codex les parcours d’audit RGAA déjà disponibles dans le
plugin Claude Code. Il réutilise `rgaa-mcp` et le moteur Rust du dépôt : il ne
duplique pas les règles d’audit.

## Contenu

- Skills : audit, triage, remédiation, vérification, rapport, test guidé et
  référence des critères.
- Serveur MCP local `rgaa-mcp`, qui expose les outils décrits ci-dessous.
- Aucune modification automatique du code : les propositions de remédiation
  restent soumises à l’approbation explicite de l’utilisateur.

## Installation depuis ce dépôt

1. Installer et compiler les binaires depuis la racine du dépôt :

   ```bash
   cargo install --path rgaa-rs/crates/rgaa-mcp
   cargo install --path rgaa-rs/crates/rgaa-cli
   ```

   Le binaire `rgaa-mcp` doit être disponible dans le `PATH` de Codex. Le CLI
   `rgaa` est facultatif et sert de solution de repli pour les opérations de
   rapport.

2. Ajouter ce dépôt comme marketplace locale Codex :

```bash
codex plugin marketplace add .
codex plugin marketplace list
```

3. Dans Codex desktop, ouvrir le répertoire des plugins, sélectionner la
   marketplace `holo-rgaa-codex`, puis installer `rgaa-accessibility-codex`.
   Redémarrer l’application après les modifications locales du plugin.

Le fichier `.agents/plugins/marketplace.json` à la racine du dépôt est la
source de découverte du plugin.

## Outils MCP

Le serveur enregistre actuellement ces neuf outils :

| Outil | Rôle |
|---|---|
| `analyze` | Analyse détaillée d’une URL, par critère, avec preuves et statuts. |
| `audit_url` | Audit de site ; renvoie un résumé et les URLs échantillonnées. |
| `get_audit_result` | Relit un audit sauvegardé à partir de son identifiant. |
| `lint_static` | Première vérification statique de HTML, JSX/TSX et Vue, sans verdict de conformité. |
| `list_criteria` | Liste les 106 critères RGAA et leur classification. |
| `remediate` | Génère des propositions de correction pour 1 à 25 problèmes. |
| `source_map` | Recherche les emplacements source probables d’observations navigateur. |
| `verify_fix` | Compare une nouvelle analyse avec un audit de référence. |
| `igt` | Test guidé clavier historique ; préférer `analyze` avec `config.igt_tools: ["keyboard"]`. |

`audit_url` ne fournit qu’un résumé de site. Pour le détail par critère,
appeler `analyze` sur les URLs de `sampled_page_urls`. Les résultats
`NeedsReview` nécessitent une revue humaine ; une erreur ou une analyse
incomplète ne constitue jamais une réussite.

## Utilisation des skills

Les skills sont proposés automatiquement lorsque la demande correspond à leur
description. On peut aussi sélectionner le skill concerné dans l’interface
Codex.

Exemples :

- « Audite https://exemple.fr selon le RGAA »
- « Priorise les problèmes de cet audit »
- « Propose des corrections pour ces constats, sans les appliquer »
- « Vérifie les fichiers corrigés contre l’audit de référence »
- « Explique le critère RGAA 1.1 »

## Vérification du paquet

Depuis la racine du dépôt :

```bash
bash rgaa-rs/plugins/rgaa-codex/tests/plugin-contract.sh
```

Ce contrôle vérifie le manifeste, la configuration MCP, l’entrée marketplace et
la présence des sept skills.
