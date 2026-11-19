# Installation d'un serveur Holo-RGAA de bout en bout

Ce guide sert à refaire, sur un nouveau serveur, l'installation qui fonctionne
aujourd'hui : accès SSH, paquets système, toolchain Rust, CLIs d'agents,
clone du dépôt, navigateur Obscura, build, tests et installation du produit
(`rgaa-cli`, `rgaa-mcp` et les plugins Claude Code et Codex).

Il s'appuie sur l'état réel du serveur de référence, relevé le 2026-11-19, et sur
ce que la CI exécute (`.github/workflows/ci.yml`). Quand la documentation existante
(`CLAUDE.md`) décrit quelque chose qui n'est **pas** présent sur ce serveur, la
section [Écarts constatés](#12-écarts-constatés-sur-le-serveur-de-référence) le dit.

---

## 0. Serveur de référence

| Élément | Valeur |
|---|---|
| OS | Ubuntu 24.04.5 LTS |
| Architecture | x86_64 (**obligatoire** pour les binaires précompilés ; en aarch64, compiler avec `install.sh --build`) |
| CPU / RAM | 8 vCPU / 31 Go |
| Disque | 387 Go (dont environ 127 Go utilisés ; le dossier `target/` du workspace Rust prend facilement **30 à 60 Go**) |
| Utilisateur | `root` |
| Pare-feu | `ufw` inactif |

**Minimum recommandé** : 4 vCPU, **16 Go de RAM** (l'édition de liens LTO de la pile
lancedb/arrow/datafusion fait tomber les machines plus petites en OOM), 100 Go de disque.

---

## 1. Accès SSH

### 1.1 Sur votre poste : créer une clé et la poser sur le serveur

```bash
# Poste local
ssh-keygen -t ed25519 -C "holo-rgaa-<nom-du-poste>" -f ~/.ssh/holo_rgaa
ssh-copy-id -i ~/.ssh/holo_rgaa.pub root@<IP_DU_SERVEUR>
```

Le serveur de référence accepte deux clés `ssh-ed25519` dans
`/root/.ssh/authorized_keys` : celle du poste d'administration et celle d'Orca
(`orca-vps-jp`). Il faut ajouter la clé d'Orca sur le nouveau serveur si vous
pilotez les agents depuis Orca (voir §5.4).

### 1.2 Alias côté poste (`~/.ssh/config`)

```sshconfig
Host holo-rgaa
    HostName <IP_DU_SERVEUR>
    User root
    IdentityFile ~/.ssh/holo_rgaa
    ServerAliveInterval 30
```

Ensuite : `ssh holo-rgaa`.

### 1.3 Durcissement sshd (identique au serveur de référence)

Une fois la connexion par clé **vérifiée dans un second terminal**, interdire les
mots de passe :

```bash
cat >/etc/ssh/sshd_config.d/00-key-only.conf <<'EOF'
PasswordAuthentication no
PermitRootLogin prohibit-password
EOF
sshd -t && systemctl reload ssh
```

Le préfixe `00-` compte. Sur les images cloud, `50-cloud-init.conf` contient
`PasswordAuthentication yes`, et sshd retient la **première** valeur lue. Le
fichier `00-` est lu avant et l'emporte. Vérifier :

```bash
sshd -T | grep -Ei 'passwordauthentication|permitrootlogin'
# attendu : passwordauthentication no / permitrootlogin without-password
```

---

## 2. Paquets système

Ce sont les paquets qu'installe la CI, plus quelques outils utilisés par les scripts
et les hooks :

```bash
apt-get update
apt-get install -y \
  build-essential pkg-config libssl-dev binutils \
  protobuf-compiler cmake clang lld mold \
  git curl ca-certificates jq python3 shellcheck gh
```

| Paquet | Pourquoi |
|---|---|
| `protobuf-compiler`, `cmake`, `clang` | indispensables à la compilation de lancedb, de lance et d'ort (sans eux, le build échoue) |
| `libssl-dev`, `pkg-config` | `openssl-sys` |
| `mold`, `lld` | éditeur de liens rapide (mold 2.30.0 sur Ubuntu 24.04) |
| `jq` | `install.sh` (configuration MCP) et hooks `.claude/settings.json` |
| `shellcheck` | lint de `install.sh` (job CI `Installer lint`) |
| `gh` | authentification Git HTTPS et gestion des PR |

Versions relevées sur le serveur de référence : protoc 3.21.12, cmake 3.28.3,
clang 18.1.3, git 2.43.0, gh 2.45.0, Python 3.12.3.

---

## 3. Rust

### 3.1 rustup

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal
source "$HOME/.cargo/env"
```

Rien d'autre à choisir. `rgaa-rs/rust-toolchain.toml` épingle **1.98.1** (avec
clippy, rustfmt et rust-analyzer), et rustup l'installe tout seul au premier `cargo`
lancé dans `rgaa-rs/`. Hors du dépôt, `stable` reste la toolchain par défaut (1.100.0
sur le serveur de référence) ; c'est normal.

> Ne pas descendre sous 1.85. Une dépendance transitive (via lancedb) exige la
> fonctionnalité Cargo `edition2024`.

### 3.2 Outils cargo

Outils présents sur le serveur de référence :

```bash
cargo install --locked cargo-nextest   # 0.9.152 sur la référence ; lanceur de tests
cargo install --locked cargo-audit     # 0.23.0 ; audit de sécurité (job CI)
```

Outils facultatifs, cités dans `CLAUDE.md` mais **absents** de la référence :

```bash
curl -L --proto '=https' --tlsv1.2 -sSf \
  https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | bash
cargo binstall -y sccache cargo-watch cargo-hack cargo-outdated cargo-edit
```

### 3.3 Accélération du build (facultatif)

La référence fonctionne **sans** `~/.cargo/config.toml` et sans sccache. Pour les
activer sur la nouvelle machine (c'est ce que fait la CI) :

```toml
# ~/.cargo/config.toml — global à la machine, jamais commité
[build]
rustc-wrapper = "sccache"          # seulement si sccache est installé

[target.x86_64-unknown-linux-gnu]
linker = "clang"
rustflags = ["-C", "link-arg=-fuse-ld=mold"]
```

Variables utiles (voir `scripts/setup-sccache.sh`) : `SCCACHE_CACHE_SIZE=10G`,
`SCCACHE_IDLE_TIMEOUT=3600`. Taux de succès du cache : `sccache --show-stats`.

**Limiter le parallélisme si la RAM est juste.** La CI fixe `CARGO_BUILD_JOBS=2`,
parce que plusieurs liens LTO simultanés tuaient le runner. Sous 16 Go, faites de même :

```bash
echo 'export CARGO_BUILD_JOBS=2' >> ~/.bashrc
```

---

## 4. Node.js et CLIs d'agents

### 4.1 Node 22

```bash
curl -fsSL https://deb.nodesource.com/setup_22.x | bash -
apt-get install -y nodejs
node --version   # v22.x (v22.23.5 sur la référence)
```

Node 20 ou plus suffit pour le harnais des sondes JS. La CI utilise 20 et 22.

### 4.2 CLIs globales (versions de la référence)

```bash
npm install -g @anthropic-ai/claude-code   # 2.1.328 — Claude Code (+ `claude plugin validate`)
npm install -g @openai/codex               # 0.170.0 — Codex CLI
npm install -g @opencode/cli               # 2.3.2   — facultatif (opencode.json à la racine)
npm install -g @earendil-works/pi-coding-agent  # 1.5.2 — facultatif
```

Ouvrir une session pour chacune : lancer `claude`, puis `codex login`, et suivre le
lien d'authentification.

### 4.3 GitHub (HTTPS via gh)

Le dépôt est cloné en **HTTPS** et les identifiants passent par `gh`. Aucune clé SSH
de déploiement n'est nécessaire :

```bash
gh auth login            # GitHub.com → HTTPS → navigateur ou token
gh auth setup-git        # pose le credential helper ci-dessous
git config --global user.name  "<Votre nom>"
git config --global user.email "<votre-email>"
```

Configuration résultante (identique à la référence) :

```
credential.https://github.com.helper=!/usr/bin/gh auth git-credential
credential.https://gist.github.com.helper=!/usr/bin/gh auth git-credential
```

### 4.4 Orca (si les agents sont pilotés depuis Orca)

Procédure complète (mode cible SSH ou serveur headless `orca serve`, appairage
depuis l'Orca local) : voir [`orca-serveur.md`](orca-serveur.md).

Sur la référence, **Orca a été installé par Codex** (Codex CLI, §4.2) le
2026-11-19 vers 08:23 UTC. La commande exacte n'est pas récupérable : la plus
ancienne session Codex conservée sur le serveur date de 09:06, après l'installation.
Pour refaire l'installation, il faut donc d'abord installer et connecter Codex
(§4.2), puis lui demander d'installer Orca sur l'hôte. Il faut aussi que la clé
`orca-vps-jp` soit dans `authorized_keys` (§1.1).

Ce que l'installation laisse, et qu'on peut contrôler sur le nouveau serveur :

| Chemin | Contenu |
|---|---|
| `~/.orca-remote/orcad-0.1.0+<hash>/` | démon `orcad` (le fichier `.install-complete` marque une installation terminée) |
| `~/.orca-remote/runtimes/node-<hash>/` | runtime Node embarqué, indépendant du Node système |
| `~/.orca/cli/bin/orca` (et `orca-ide`) | wrappers shell vers ce Node ; CLI en version 1.4.263 sur la référence |
| `~/.orca/` | données du runtime (bases SQLite, profils, jetons) |

Le runtime écoute uniquement en local : un socket Unix et `ws://127.0.0.1:6800`,
jamais exposés publiquement. Vérification : `~/.orca/cli/bin/orca status`. Les
worktrees des agents se créent sous `/root/orca/workspaces/Holo-RGAA/<nom>`.

Ne copiez **pas** `~/.orca/` d'un serveur à l'autre : il contient des jetons et des
clés propres à l'hôte.

---

## 5. Le dépôt

```bash
git clone https://github.com/jamon8888/Holo-RGAA.git
cd Holo-RGAA
```

### 5.1 Configuration `.env`

```bash
cp .env.example .env
$EDITOR .env
```

Seules les variables du LLM sont nécessaires pour un audit complet :

```dotenv
RGAA_LLM_PROVIDER=myia
RGAA_LLM_MODEL=swift-1.5-27b
MYIA_API_KEY=<clé MyIA>          # ou RGAA_LLM_API_KEY
RGAA_TACTICAL_RPM=10
RGAA_REASONING_RPM=20
```

Les autres fournisseurs (openai, openrouter, groq, ollama, vllm, etc.) sont documentés
dans `.env.example`. Ne **jamais** commiter `.env`. Une variable déjà présente dans
l'environnement (systemd, CI) l'emporte sur `.env`.

---

## 6. Navigateur Obscura 0.2.4 (substrat épinglé)

Les tests E2E et les audits réels exigent **exactement** `obscura 0.2.4`, avec le même
contrôle que la CI :

```bash
mkdir -p ~/.local/bin && cd "$(mktemp -d)"
curl -fSL -o obscura.tar.gz \
  https://github.com/h4ckf0r0day/obscura/releases/download/v0.2.4/obscura-x86_64-linux.tar.gz
echo "757e7b597ba5cdd53af9fc701e0a9b80f0a8d788f545d4f82ec9ad32db77f000  obscura.tar.gz" | sha256sum -c -
tar -xzf obscura.tar.gz
chmod +x obscura obscura-worker
install -m 755 obscura obscura-worker ~/.local/bin/
echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.bashrc && export PATH="$HOME/.local/bin:$PATH"
obscura --version   # doit afficher : obscura 0.2.4
```

`obscura-worker` doit se trouver **à côté** d'`obscura`. Les tests de couverture
cherchent le binaire uniquement dans le `PATH` (`ObscuraBridge::new()`), et
`RGAA_OBSCURA_BIN` ne les redirige pas. Si un autre `obscura` (une 0.1.x par exemple)
apparaît plus tôt dans le `PATH`, il est utilisé sans avertissement.

`install.sh` (§9) télécharge et vérifie aussi Obscura dans `~/.local/bin`. Cette étape
manuelle sert surtout à lancer les tests depuis les sources.

---

## 7. Build et tests

Toutes les commandes se lancent depuis `rgaa-rs/`.

```bash
cd rgaa-rs
make build            # cargo build --workspace
make build-release    # premier build à froid : compter de 20 à 40 min
make check            # fmt-check + clippy -D warnings + nextest (= CI hors E2E)
make ci               # check + build-release
cargo test --workspace --doc   # doctests (nextest ne les lance pas)
```

En cas de débordement de pile dans les tests, faire comme la CI :
`export RUST_MIN_STACK=8388608`.

### 7.1 Couches de test (détail dans `docs/testing-layers.md`)

```bash
# Harnais DOM des sondes JS (Node, sans navigateur, quelques secondes)
(cd crates/rgaa-rules/tests/js && npm ci && ./run-harnesses.sh)

# E2E navigateur (exige obscura 0.2.4 dans le PATH)
RUN_E2E=1 RGAA_E2E_REQUIRED=1 OBSCURA_WORKERS=1 \
  cargo nextest run -p rgaa-obscura --no-tests=fail --test-threads 2 \
  -E 'binary(/^criterion_.+_coverage$/) + binary(=alt_carriers)'

# E2E spider (serveur HTTP local, sans réseau sortant)
RUN_E2E=1 RGAA_E2E_REQUIRED=1 \
  cargo nextest run -p rgaa-spider --no-tests=fail --test spider_tool_test

# E2E pipeline complet (exige MYIA_API_KEY, jamais vérifié en CI)
RUN_E2E=1 RGAA_E2E_REQUIRED=1 \
  cargo test --workspace --test full_audit --test obscura_audit --test fixture_classification -- --nocapture
```

Sans `RUN_E2E=1`, les tests E2E se terminent immédiatement sans rien vérifier.
`RGAA_E2E_REQUIRED=1` transforme cette sortie en échec.

---

## 8. PostgreSQL (facultatif)

Postgres n'est nécessaire que pour `rgaa-api` en stockage distant et pour le test
d'intégration `rgaa-storage` (sauté si `DATABASE_URL` n'est pas défini). Ni Docker ni
Postgres ne sont présents sur la référence.

```bash
apt-get install -y docker.io docker-compose-v2
docker compose up -d postgres          # postgres:16-alpine sur :5432 (postgres/postgres)
export DATABASE_URL=postgres://postgres:postgres@127.0.0.1:5432/rgaa
```

Le schéma est créé par `rgaa-api` à son démarrage (migrations sqlx dans
`crates/rgaa-storage/migrations`).

---

## 9. Installation du produit (binaires et plugins)

### 9.1 Avec l'installeur (recommandé)

Depuis la racine du dépôt :

```bash
./install.sh            # binaires précompilés (release « latest ») vers ~/.local/bin
./install.sh --build    # ou : compile depuis les sources (obligatoire en aarch64)
```

L'installeur effectue les étapes suivantes :

- il installe `rgaa` (TUI), `rgaa-cli`, `rgaa-api` et `rgaa-mcp` dans `~/.local/bin` ;
- il télécharge et vérifie `obscura 0.2.4` ;
- il installe le plugin Claude Code `rgaa-accessibility` (depuis
  `rgaa-rs/plugins/rgaa-consultant`) et supprime l'ancien `~/.claude/plugins/rgaa-audit` ;
- il déclare `rgaa-mcp` dans `~/.claude/mcp.json`, avec `RGAA_OBSCURA_BIN` ;
- il prépare le plugin Codex `rgaa-accessibility-codex` dans une marketplace locale
  et l'enregistre dans `~/.codex/config.toml`.

Variables d'environnement prises en compte : `RGAA_VERSION` (tag de release) et
`RGAA_INSTALL_DIR`. Pour désinstaller : `./install.sh --uninstall`.

### 9.2 À la main

```bash
# Plugin Claude Code via la marketplace du dépôt
claude plugin marketplace add ./
claude plugin install rgaa-accessibility@holo-rgaa
claude plugin details rgaa-accessibility

# Plugin Codex
cargo install --path rgaa-rs/crates/rgaa-mcp
codex plugin marketplace add .
```

Le guide détaillé des plugins se trouve dans `docs/rgaa-plugin-install.md`.

---

## 10. Facultatif : Obscura en service systemd

Ce mode sert à faire tourner Obscura comme service partagé, au lieu de laisser
`ObscuraBridge` lancer son propre processus. Le gabarit est `deploy/obscura.service` :

```bash
useradd --system --no-create-home obscura
mkdir -p /opt/holo-rgaa && install -m 755 ~/.local/bin/obscura ~/.local/bin/obscura-worker /opt/holo-rgaa/
cp deploy/obscura.service /etc/systemd/system/
# dans le fichier, remplacer --workers 4 par le nombre de CPU (nproc)
systemctl daemon-reload && systemctl enable --now obscura
```

Le port CDP 9222 écoute uniquement en loopback. Ne **jamais** l'exposer sur `0.0.0.0`
sans authentification devant, car il n'en a aucune.

## 11. Facultatif : basemind (index de contexte des agents)

`CLAUDE.md` le présente comme l'outil de recherche par défaut, mais il n'est **pas**
installé sur la référence. Les hooks de `.claude/settings.json` ne font alors rien,
sans erreur. Pour l'installer :

```bash
npm install -g basemind@latest     # au moins 0.29 (routage des documents markdown, JSON et YAML)
basemind --version; npm view basemind version
cd <racine du dépôt> && basemind init && basemind scan
basemind hook install              # hook pre-commit, non commité : à refaire à chaque clone
```

`basemind.toml` (commité) fixe `embedding_preset = "balanced"`. Ne passez pas à
`quality` (beaucoup trop lent sur CPU) ni à `fast` (bug de dimension sur les anciennes
versions).

---

## 12. Écarts constatés sur le serveur de référence

| `CLAUDE.md` affirme | Réalité au 2026-11-19 |
|---|---|
| sccache câblé dans `~/.cargo/config.toml` | pas de `~/.cargo/config.toml`, sccache absent |
| mold câblé via `rustflags` | mold installé (apt) mais **non câblé** |
| `cargo-binstall`, `cargo-watch`, `cargo-hack`, `cargo-outdated` installés | absents (seuls `cargo-nextest` et `cargo-audit` sont présents) |
| basemind disponible | absent |
| Obscura 0.2.4 disponible pour l'E2E | absent du `PATH` (une copie temporaire traîne seulement dans `/tmp/pr388-browser/`) |
| — | `jq`, `shellcheck`, Docker et `psql` absents |

Le build et les tests unitaires fonctionnent malgré ces écarts. L'E2E navigateur exige
l'étape 6, et `install.sh` donne un meilleur résultat avec `jq`.

---

## 13. Vérification finale

```bash
rustc --version                       # dans rgaa-rs/ : rustc 1.98.1
cargo nextest --version
node --version && claude --version && codex --version
gh auth status
obscura --version                     # obscura 0.2.4
cd rgaa-rs && make check              # vert = installation de dev complète
~/.local/bin/rgaa-cli --version && ~/.local/bin/rgaa-mcp --help | head -3
claude plugin details rgaa-accessibility
```
