# MCP Bundle (.mcpb) — Installation en un clic dans Claude Desktop

**Ticket :** #444 — Installer Holo-RGAA en un clic dans Claude Desktop (MCP Bundle)
**Branche :** `research/mcp-bundle-claude-desktop`
**Dépôt étudié :** modelcontextprotocol/mcpb (https://github.com/modelcontextprotocol/mcpb)
**Branche du dépôt mcpb :** `main`
**Date de consultation :** 2026-11-21
**Auteur :** worker (dispatched, tâche #76ba0373ccc9)

---

## 1. Contexte et objectif

Ticket #444 (jamon8888/Holo-RGAA) demande d'évaluer la faisabilité de distribuer Holo-RGAA (serveur MCP `rgaa-mcp` + binaire navigateur Obscura + modèles locaux) comme **MCP Bundle** (`.mcpb`) installable en un clic dans Claude Desktop sur Windows et macOS.

Points à établir : format du manifeste, binaires natifs par plateforme, signature, mises à jour, taille maximale, configuration de la clé de l'instance, comportement en entreprise (politique d'extensions autorisées).

Référence carte #441 : socle auditeur RGAA/EAA démontrable en cabinet (Claude Desktop + Holo-RGAA).

---

## 2. Sources primaires consultées

| Source | URL | Date de consultation |
|--------|-----|----------------------|
| mcpb README (vue d'ensemble) | https://github.com/modelcontextprotocol/mcpb/blob/main/README.md | 2026-11-21 |
| MANIFEST.md (spécification du manifeste) | https://github.com/modelcontextprotocol/mcpb/blob/main/MANIFEST.md | 2026-11-21 |
| CLI.md (documentation CLI) | https://github.com/modelcontextprotocol/mcpb/blob/main/CLI.md | 2026-11-21 |
| Schéma JSON v0.4 | https://github.com/modelcontextprotocol/mcpb/blob/main/schemas/mcpb-manifest-v0.4.schema.json | 2026-11-21 |
| Code source : signature (sign.ts) | https://github.com/modelcontextprotocol/mcpb/blob/main/src/node/sign.ts | 2026-11-21 |
| Code source : pack (pack.ts) | https://github.com/modelcontextprotocol/mcpb/blob/main/src/cli/pack.ts | 2026-11-21 |
| Code source : validate (validate.ts) | https://github.com/modelcontextprotocol/mcpb/blob/main/src/node/validate.ts | 2026-11-21 |
| Code source : config / substitution (config.ts) | https://github.com/modelcontextprotocol/mcpb/blob/main/src/shared/config.ts | 2026-11-21 |
| Exemple binaire Rust (calculator-rust) | https://github.com/modelcontextprotocol/mcpb/tree/main/examples/calculator-rust | 2026-11-21 |
| Exemple UV runtime (hello-world-uv) | https://github.com/modelcontextprotocol/mcpb/tree/main/examples/hello-world-uv | 2026-11-21 |
| Ticket #444 (Holo-RGAA) | https://github.com/jamon8888/Holo-RGAA/issues/444 | 2026-11-21 |
| Carte #441 (Holo-RGAA) | https://github.com/jamon8888/Holo-RGAA/issues/441 | 2026-11-21 |

---

## 3. Format du manifeste (manifest.json)

Le `.mcpb` est une archive ZIP contenant un `manifest.json` et les fichiers du serveur MCP local. Le format est inspiré des extensions Chrome (`.crx`) et VS Code (`.vsix`).

### Version de la spécification

- Dernière version supportée : **0.4** (ajoute le type de serveur `uv` pour Python).
- Version par défaut pour les nouveaux packages : **0.3**.
- Les schémas JSON sont versionnés : `0.1`, `0.2`, `0.3`, `0.4`.
- Source : MANIFEST.md, `src/shared/constants.ts` (`LATEST_MANIFEST_VERSION = "0.4"`, `DEFAULT_MANIFEST_VERSION = "0.3"`).

### Champs requis

- `manifest_version` : `"0.3"` ou `"0.4"` (le champ déprécié `dxt_version` est accepté).
- `name` : nom machine (utilisé pour CLI, APIs).
- `version` : version sémantique (semver).
- `description` : description brève (localisable).
- `author` : objet avec `name` (requis), `email` (optionnel), `url` (optionnel).
- `server` : configuration du serveur (type, entry_point, mcp_config).

### Types de serveur (`server.type`)

| Type | Description | Runtime nécessaire |
|------|-------------|-------------------|
| `node` | Serveur Node.js, dépendances bundlées dans `node_modules` | Node.js (via `compatibility.runtimes.node`) |
| `python` | Serveur Python, dépendances bundlées dans `server/lib/` ou `server/venv/` | Python (via `compatibility.runtimes.python`) |
| `binary` | Exécutable précompilé, autonome | Aucun (static linking recommandé) |
| `uv` (v0.4+) | Python via UV, dépendances résolues par l'hôte depuis `pyproject.toml` | Aucun (UV géré par l'hôte) |

Source : MANIFEST.md section "Server Configuration", `src/schemas/0.4.ts`.

### Configuration d'exécution (`server.mcp_config`)

Définit comment l'application hôte exécute le serveur MCP. Champs : `command`, `args` (tableau), `env` (objet), `platform_overrides` (surcharges par plateforme : `win32`, `darwin`, `linux`).

Substitution de variables supportée :
- `${__dirname}` : chemin absolu du répertoire d'installation du bundle.
- `${HOME}`, `${DESKTOP}`, `${DOCUMENTS}`, `${DOWNLOADS}` : répertoires système.
- `${pathSeparator}` / `${/}` : séparateur de chemin.
- `${user_config.CLE}` : valeur fournie par l'utilisateur (expansion de tableau pour `multiple: true`).

Pour les binaires, l'hôte ajoute automatiquement `.exe` sur Windows.

Source : MANIFEST.md section "MCP Configuration", `src/shared/config.ts`.

### Configuration utilisateur (`user_config`)

Permet de collecter auprès de l'utilisateur des valeurs passées au serveur à l'exécution. Chaque option définit :
- `type` : `string`, `number`, `boolean`, `directory`, `file`.
- `title`, `description` : libellés UI.
- `required` (défaut `false`).
- `default` : valeur par défaut (supporte substitution de variables).
- `multiple` (pour `directory`/`file`, défaut `false`).
- `sensitive` (pour `string`, masque l'entrée, stockage sécurisé, défaut `false`).
- `min` / `max` (pour `number`).

Exemple pertinent pour une "clé d'instance" :

```json
"user_config": {
  "api_key": {
    "type": "string",
    "title": "Clé d'instance Holo-RGAA",
    "description": "Clé d'API pour l'instance Holo-RGAA",
    "sensitive": true,
    "required": true
  }
}
```

Puis référencée dans `mcp_config.env` : `"API_KEY": "${user_config.api_key}"`.

Source : MANIFEST.md section "User Configuration", `src/schemas/0.4.ts`.

### Compatibilité

`compatibility.platforms` : `["darwin", "win32", "linux"]` (valeurs `process.platform` / `sys.platform`). Aucune distinction d'architecture (x86 vs ARM) n'est exprimée dans le manifeste.

`compatibility.runtimes` : `python` et/ou `node` avec contrainte semver. Les binaires n'en ont pas besoin.

`compatibility.claude_desktop` : contrainte de version du client Claude Desktop (semver).

Source : MANIFEST.md section "Compatibility", schéma JSON v0.4.

### Champ `_meta`

Métadonnées spécifiques à la plateforme/client, namespace reverse-DNS (ex. `com.microsoft.windows`, `com.apple.darwin`). Valeurs : dictionnaire libre. Exemple dans les tests :

```json
"_meta": {
  "com.microsoft.windows": { "package_family_name": "Pkg_123", "channel": "stable" },
  "com.apple.darwin": { "bundle_id": "com.example.app", "notarized": true }
}
```

Source : MANIFEST.md section "Field Definitions" (`_meta`), `src/schemas/0.4.ts`, `test/schemas.test.ts` (bloc `_meta`).

---

## 4. Binaires natifs par plateforme (Windows, macOS x86/ARM)

### Support dans le format

- `server.type = "binary"` : exécutable précompilé, toutes les dépendances incluses, totalement autonome.
- `mcp_config.platform_overrides` : permet des `command`/`args`/`env` différents par plateforme (`win32`, `darwin`, `linux`).
- Sur Windows, l'hôte ajoute automatiquement `.exe` au binaire d'entrée.
- Lors du pack (`mcpb pack`), le bit exécutif (`+x`) est forcé sur l'entrée point binaire sous Unix si absent (pour éviter `EACCES` au spawn).
- Lors de l'unpack, les permissions Unix sont restaurées depuis les attributs du ZIP.

### Architecture (x86 vs ARM)

**Non trouvé** : le manifeste et le schéma ne distinguent pas l'architecture CPU (`x86_64` vs `aarch64`/`arm64`). Le champ `platforms` ne connaît que `darwin`/`win32`/`linux`. Il appartient au développeur de fournir des binaires séparés par architecture (par exemple via des entrées `platform_overrides` distinctes ou des noms de fichiers distincts), mais le format MCPB lui-même ne modélise pas l'architecture.

Exemple de structure binaire :

```
bundle.mcpb
├── manifest.json          # server.type = "binary"
├── server/
│   ├── my-server          # binaire Unix (darwin/linux)
│   └── my-server.exe      # binaire Windows
└── icon.png
```

Source : MANIFEST.md section "Server Configuration" (type binary), README section "Binary Bundles", `src/cli/pack.ts` (forçage du bit exécutif), `src/node/validate.ts` (vérification du bit exécutif).

---

## 5. Signature des bundles

### Format

- Algorithme : **PKCS#7** (Cryptographic Message Syntax), signature **détachée** (le contenu ZIP original n'est pas modifié).
- Emplacement : bloc de signature appendé à la fin du fichier `.mcpb`, délimité par les marqueurs `MCPB_SIG_V1` (en-tête) et `MCPB_SIG_END` (pied de page).
- Structure : `[ZIP original] MCPB_SIG_V1 [4 octets length little-endian] [DER-encoded PKCS#7 signature] MCPB_SIG_END`.
- Le champ `comment_length` de l'EOCD (End Of Central Directory) du ZIP est mis à jour pour inclure le bloc de signature, afin que les parseurs ZIP stricts acceptent le fichier signé.

### Certificats

- Certificat X.509 au format PEM, avec extended key usage "Code Signing" recommandé.
- Clé privée correspondante en PEM.
- Certificats intermédiaires optionnels (chaîne de confiance).
- Certificats auto-signés supportés pour le développement (`mcpb sign --self-signed`), certificats CA-issued pour la production.

### Vérification

La vérification est effectuée par l'application hôte contre le magasin de certificats du système d'exploitation :
- **macOS** : `security verify-cert -c <chain> -p codeSign`.
- **Windows** : PowerShell `X509Chain` avec `ApplicationPolicy` code signing (`1.3.6.1.5.5.7.3.3`), vérification de révocation en ligne.
- **Linux** : `openssl verify -purpose codesigning -CApath /etc/ssl/certs`.

Statuts de retour : `signed`, `self-signed`, `unsigned`.

### CLI

- `mcpb sign <fichier> [--cert] [--key] [--intermediate] [--self-signed]`.
- `mcpb verify <fichier>` : retourne statut, éditeur, émetteur, dates de validité, empreinte.
- `mcpb unsign <fichier>` : retire la signature (dev/test).

**Non trouvé** : aucune exigence de signature obligatoire par le format lui-même (les bundles non signés sont des ZIP valides). La politique de signature (obligatoire ou non) relève de l'application hôte (Claude Desktop).

Source : `src/node/sign.ts` (signMcpbFile, verifyMcpbFile, createSignatureBlock, extractSignatureBlock, verifyCertificateChain), CLI.md section "Signature", MANIFEST.md (aucun champ de signature dans le manifeste).

---

## 6. Mises à jour

**Non trouvé** : le format `.mcpb` et le référentiel `modelcontextprotocol/mcpb` ne documentent **aucun mécanisme de mise à jour automatique** intégré au bundle (pas de champ d'URL de mise à jour, pas de manifeste de mise à jour, pas de mécanisme de diffusion différée).

Le README indique que Claude pour macOS et Windows propose "automatic updates" comme fonctionnalité de l'intégration hôte, mais le mécanisme n'est pas spécifié dans ce dépôt. Le champ `version` (semver) du manifeste permettrait à un hôte de détecter une nouvelle version, mais la logique de vérification/téléchargement relève de l'hôte, non de MCPB.

Source : README.md (mention "automatic updates" comme feature de Claude Desktop), MANIFEST.md (champ `version` uniquement, sans mécanisme de mise à jour).

---

## 7. Taille des bundles

**Non trouvé** : aucune limite de taille maximale n'est spécifiée dans le format MCPB (ni dans le manifeste, ni dans le schéma, ni dans la CLI).

Données de référence (tailles observées dans les exemples du dépôt) :
- UV runtime : ~2 KB (bundle minimal, dépendances gérées par l'hôte).
- Binaire Rust (calculator-rust, stripped + LTO) : ~2.5 Mo.
- Python traditionnel : 5-10 Mo (dépendances bundlées, impossibilité de bundler portablement les dépendances compilées telles que pydantic).
- Node.js : dépend de la taille de `node_modules` (non chiffrée dans le dépôt).

La CLI `mcpb pack` affiche la taille du package et la taille décompressée. La compression ZIP utilise le niveau maximum (`level: 9` via `fflate`).

Source : README.md sections "Language Choice Recommendation" / "Bundling Dependencies" / "Binary Bundles", `src/cli/pack.ts` (`zipSync` level 9, affichage `package size` / `unpacked size`), `examples/calculator-rust/README.md` ("Binary size: ~2.5 MB"), `examples/hello-world-uv/README.md` ("Small bundle size (~2 KB)").

---

## 8. Configuration utilisateur : clé de l'instance

Le champ `user_config` permet de déclarer une option `string` marquée `sensitive: true`, collectée via l'interface de l'application hôte et injectée dans le serveur via substitution `${user_config.<cle>}` dans `mcp_config` (command args ou variables d'environnement).

Cela couvre le cas de la "clé de l'instance" (clé API Holo-RGAA) : l'utilisateur la saisit une fois lors de l'installation, elle est stockée de manière sécurisée par l'hôte, et transmise au serveur à chaque lancement.

**Non trouvé** : aucun mécanisme de chiffrement spécifique au bundle lui-même ; le stockage sécurisé relève de l'application hôte. Aucun champ dédié "instance key" n'existe — c'est une option `user_config` générique de type `string` + `sensitive`.

Source : MANIFEST.md section "User Configuration", `src/schemas/0.4.ts` (`McpbUserConfigurationOptionSchema` avec `sensitive`), `src/shared/config.ts` (substitution `${user_config.KEY}`).

---

## 9. Politique d'entreprise (extensions autorisées)

**Non trouvé** : le dépôt `modelcontextprotocol/mcpb` ne documente aucune politique d'entreprise, aucune liste d'extensions autorisées (allowlist), aucun mécanisme de gestion centralisée (MDM, GPO), aucun contrôle d'administrateur pour restreindre l'installation de bundles.

Les leviers disponibles dans le format qui pourraient servir une politique d'entreprise :
- `_meta` : métadonnées spécifiques plateforme (reverse-DNS namespaced), par ex. `com.microsoft.windows.package_family_name` pour l'intégration MSIX/Windows Store, `com.apple.darwin.bundle_id` pour macOS. Peuvent être lus par un outil de gestion, mais ce n'est pas spécifié.
- `privacy_policies` : requis quand l'extension se connecte à des services externes traitant des données utilisateur (liens vers les politiques de confidentialité de chaque service).
- Signature : un bundle signé par un certificat de code signing connu peut être vérifié contre un magasin de confiance ; un hôte pourrait refuser les bundles non signés ou auto-signés (politique de l'hôte, non de MCPB).

Source : MANIFEST.md (`_meta`, `privacy_policies`), `src/node/sign.ts` (vérification contre le magasin de certificats OS).

---

## 10. Faisabilité pour Holo-RGAA (rgaa-mcp + Obscura + modèles locaux)

### Composants concernés (dépôt Holo-RGAA)

- `rgaa-mcp` (`rgaa-rs/crates/rgaa-mcp/`) : serveur MCP stdio pour l'audit RGAA, trois outils (analyze, remediation, guided). Dépend de `rgaa-obscura`, `rgaa-orchestrator`, `rgaa-core`, etc. Point d'entrée : `src/main.rs` (`#[tokio::main]`, `service.serve(stdio())`).
- `rgaa-obscura` : pont vers le binaire Obscura (navigateur CDP), lu via `RGAA_OBSCURA_BIN` / `PATH` (`ObscuraBridge::from_env()`). Version épinglée : obscura 0.2.4.
- Modèles locaux : via `rgaa-holo` (client LLM MyIA) ou `rgaa-agent` ; nécessitent une clé API / point de terminaison configuré.

### Analyse par critère MCPB

| Critère | Faisabilité | Commentaire |
|---------|------------|-------------|
| Format du manifeste | ✅ Oui | `rgaa-mcp` est un binaire Rust autonome → `server.type = "binary"` convient. |
| Binaire natif Windows | ✅ Oui (partiel) | `server.type = "binary"` + `platform_overrides.win32`. Rust compile natif Windows. |
| Binaire natif macOS (x86/ARM) | ⚠️ Partiel | Format ne distingue pas l'architecture ; faudrait deux binaires distincts (darwin x86_64 / darwin arm64) gérés par `platform_overrides` ou noms de fichiers distincts. **Non trouvé** de support explicite ARM dans le schéma. |
| Signature | ✅ Oui | `mcpb sign` avec PKCS#7 ; cert auto-signé (dev) ou CA (prod). Vérification OS native. |
| Mises à jour | ❌ Non trouvé | Aucun mécanisme de mise à jour dans MCPB ; à gérer par l'hôte ou hors format. |
| Taille | ⚠️ À surveiller | Aucune limite spécifiée. Un binaire Rust stripped+LTO fait ~2-3 Mo (référence calculator-rust : 2.5 Mo). Obscura s'ajoute (taille non évaluée ici). |
| Clé de l'instance | ✅ Oui | `user_config` avec `type: string`, `sensitive: true`, injecté via `${user_config.api_key}`. |
| Politique entreprise | ❌ Non trouvé | Aucune allowlist/gestion centralisée dans MCPB ; levier `_meta` et signature possibles mais non spécifiés pour l'entreprise. |

### Obstruction principale : Obscura

`rgaa-mcp` lance Obscura via `ObscuraBridge::from_env()`, qui lit le binaire Obscura depuis `RGAA_OBSCURA_BIN` ou le `PATH`. Dans un bundle MCPB, le binaire Obscura devrait être empaqueté à côté de `rgaa-mcp` et référencé par `${__dirname}`. Cependant :

- Obscura est un binaire natif par plateforme (et probablement par architecture), ce qui réactive le problème de l'absence de distinction x86/ARM dans le manifeste.
- Obscura nécessite un affichage / environnement graphique (CDP, rendu de pages) qui peut ne pas fonctionner headless dans le contexte sandboxé de Claude Desktop. **Non vérifié** dans la documentation mcpb.

### Modèles locaux

Si "modèles locaux" signifie un LLM exécuté localement (OLLM, llama.cpp, etc.), celui-ci n'est pas un serveur MCP et n'entrerait pas dans `server.type`. Il faudrait un serveur MCP wrapper (déjà `rgaa-mcp` ou un autre) qui l'appelle, avec la clé/URL configurée via `user_config`. Si "modèles locaux" désigne les appels à un LLM distant (MyIA/Holo), la clé d'API passe par `user_config.sensitive`.

### Recommandation

Le format MCPB est **techniquement adapté** à la distribution de `rgaa-mcp` comme bundle `.mcpb` installable en un clic dans Claude Desktop, sous réserve de :

1. Construire `rgaa-mcp` en binaire statique (release, `strip = true`, LTO) pour chaque plateforme (Windows, macOS x86_64, macOS arm64, Linux).
2. Emballer Obscura à côté et le référencer via `${__dirname}` (à vérifier : compatibilité headless/sandbox de Claude Desktop).
3. Déclarer la clé d'instance via `user_config` (`sensitive: true`).
4. Signer avec un certificat de code signing (CA-issued pour la production).
5. Gérer les mises à jour en dehors du format (pas de mécanisme natif MCPB).
6. Vérifier la politique d'entreprise côté client (aucune allowlist MCPB native).

Les points **non trouvés** (à ne pas deviner) : taille limite, distinction x86/ARM, mécanisme de mise à jour, politique d'entreprise/allowlist, comportement headless d'Obscura sous Claude Desktop.

---

## 11. Conclusion

Le format MCP Bundle (`.mcpb`), spécifié par `modelcontextprotocol/mcpb` (version 0.3/0.4 du manifeste), fournit tout le nécessaire pour packager `rgaa-mcp` comme bundle installable en un clic dans Claude Desktop : manifeste déclaratif, type serveur `binary`, configuration utilisateur avec clé sensible (`user_config.sensitive`), signature PKCS#7 vérifiable contre le magasin de certificats OS, et substitutions de chemins (`${__dirname}`, `${user_config.*}`).

Les limites identifiées : pas de distinction d'architecture (x86 vs ARM) dans le manifeste, aucun mécanisme de mise à jour natif, aucune politique d'entreprise/allowlist, et aucune limite de taille documentée. L'intégration d'Obscura (binaire natif, affichage CDP) dans le contexte sandboxé de Claude Desktop reste à valider.

Livrable : ce fichier, sur la branche `research/mcp-bundle-claude-desktop`, poussé sur `origin`. Aucun code modifié. Aucun secret inclus.
