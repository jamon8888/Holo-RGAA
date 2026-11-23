# Orca sur un VPS Hostinger : installation serveur et usage depuis le poste local

Ce guide complète `installation.md`, qui couvre SSH, les paquets, Rust et les CLIs
d'agents. Il s'appuie sur l'état relevé sur ce VPS le 2026-11-19 et sur la
documentation officielle (https://orca.dev/docs/remote-servers,
https://orca.dev/docs/ways-to-run, https://orca.dev/docs/ssh,
https://orca.dev/docs/install).

## 1. Deux façons de brancher l'Orca local sur le VPS

| Mode | Qui possède le runtime | Quand le choisir |
|---|---|---|
| **Cible SSH** (mode actuel de ce VPS) | l'Orca du poste ; le VPS exécute les worktrees et les terminaux | un seul poste qui pilote le VPS, sans port à ouvrir |
| **Remote Orca Server** (`orca serve`) | le VPS, qui possède tout le runtime | plusieurs clients (desktop, navigateur, mobile), sessions partagées, service permanent |

**Ce qui tourne aujourd'hui sur ce VPS : le mode cible SSH.** Une connexion
`sshd: root@notty` a lancé `orcad` depuis `~/.orca-remote/` (Node embarqué dans
`~/.orca-remote/runtimes/`). Le runtime écoute uniquement en loopback (socket Unix
et `ws://127.0.0.1:6800`). Il n'y a ni `orca serve` ni service systemd.
`orca host list` ne connaît que `local`. Codex a installé Orca le 19/11 vers 08:23
UTC ; la commande exacte n'a pas été conservée.

N'utilisez **qu'un seul mode à la fois** sur un hôte. Ne lancez pas un second
`orca serve` pour le même profil.

## 2. Préparer le VPS (identique pour les deux modes)

1. Accès SSH par clé et durcissement de sshd : `installation.md` §1.
2. Paquets, Node 22 et CLIs d'agents : `installation.md` §2 et §4.
   ```bash
   npm install -g @anthropic-ai/claude-code @openai/codex @opencode/cli @earendil-works/pi-coding-agent
   ```
3. **Connecter les agents sur le VPS.** Les sessions distantes utilisent le PATH, le
   home et les identifiants du serveur ; ceux du poste ne sont pas transmis.
   ```bash
   claude          # se connecter
   codex login
   gh auth login && gh auth setup-git
   ```
4. Hostinger : n'ouvrez **aucun** port Orca dans le pare-feu hPanel. Seul le port
   22 doit être ouvert.

## 3. Mode A : cible SSH (reproduit ce VPS)

Sur le **poste local** :

1. Installer Orca desktop (Linux : `orca-linux.AppImage`, ou `.deb`/`.rpm`, depuis
   https://github.com/stablyai/orca/releases/latest ; macOS et Windows depuis
   orca.dev/docs/install).
2. Avoir l'alias SSH qui fonctionne (`ssh holo-rgaa`, voir `installation.md` §1.2).
3. Dans Orca : **Settings → SSH → Add Target**. Renseigner l'hôte, l'utilisateur
   `holo-rgaa`, le port 22 et la clé (ou choisir `holo-rgaa` dans le sélecteur de la
   config OpenSSH), puis **Test** et **Save**.
4. À la première connexion, Orca déploie son serveur géré sur le VPS
   (`~/.orca-remote/orcad-*`, son Node, et la CLI `~/.orca/cli/bin/orca`).
5. Ajouter le dépôt : cloner `https://github.com/jamon8888/Holo-RGAA.git` sur le
   VPS, ou `orca project setup-clone`. À la création d'un worktree, choisir la cible
   SSH dans **Run on**. Les worktrees se créent sous
   `/home/holo-rgaa/orca/workspaces/Holo-RGAA/<nom>`.
6. Facultatif : cocher **« Allow this host's orca CLI to control Orca »** pour que
   les agents du VPS puissent utiliser `orca` (orchestration, terminaux).

Gestion courante : **Settings → Managed servers**, ou
`orca environment status|update|rollback|recover|stop|forget`. Les terminaux
distants survivent à la fermeture de l'Orca local.

## 4. Mode B : Remote Orca Server (`orca serve`, headless)

Sur le **VPS** :

0. **Installer le paquet serveur officiel (sans interface).** La CLI déployée par le
   mode A (`~/.orca/cli/bin/orca`) **ne peut pas** créer le service. Testé ici, elle
   refuse avec « This `orca` command isn't running from an installed Orca app ».
   Il faut le paquet serveur, que chaque release publie depuis la 1.4.264 :
   `orca-server-linux-x64.tar.gz`, avec son propre Node 24, `orcad` et le lanceur
   `bin/orca-ide`.
   ```bash
   V=1.4.264   # dernière release au 2026-11-19 : gh release view -R stablyai/orca
   cd /tmp
   curl -fLO https://github.com/stablyai/orca/releases/download/v$V/orca-server-linux-x64.tar.gz
   echo '8f78b5acb5e57a53aa9058edc701af4d9e4b41a0caf3f77f8ce23c2e1bbecad6  orca-server-linux-x64.tar.gz' | sha256sum -c -
   sudo mkdir -p /opt/orca && sudo tar -xzf orca-server-linux-x64.tar.gz -C /opt/orca
   sudo ln -sfn /opt/orca/orca-server-$V-linux-x64 /opt/orca/current
   sudo ln -sf /opt/orca/current/bin/orca-ide /usr/local/bin/orca-ide
   orca-ide --version
   ```
   Le lanceur résout son vrai chemin à travers le lien symbolique (`readlink -f`).
   Après un changement de `/opt/orca/current`, redémarrez le service et mettez à
   jour les clients ensemble pour éviter un décalage de version de protocole.
   Sur ARM, prendre `orca-server-linux-arm64.tar.gz`.

   Dans la suite, `orca` désigne `orca-ide`. Lancez les commandes depuis un shell de
   connexion ordinaire (`ssh holo-rgaa`), **pas** depuis un terminal ouvert par
   Orca en mode SSH.

   **À ne pas faire :** passer au mode B sur ce VPS sans arrêter le mode A. Les deux
   partagent `~/.orca`, et `orca serve` sort avec le code 3 si un autre Orca sert
   déjà ce profil. Retirez d'abord la cible SSH dans l'Orca local (Settings → SSH),
   ou utilisez `orca environment stop`.

1. **Réseau privé (recommandé) : Tailscale.**
   ```bash
   curl -fsSL https://tailscale.com/install.sh | sh
   tailscale up
   tailscale ip -4          # 100.x.y.z
   ```
2. Essai au premier plan :
   ```bash
   orca-ide serve --port 6768 --pairing-address 100.x.y.z
   ```
   La commande affiche un lien d'appairage. Il donne accès au serveur : gardez-le
   privé.
3. Service permanent (unité systemd utilisateur, `--supervise --no-pairing`) :
   ```bash
   orca-ide serve service install --port 6768 --pairing-address 100.x.y.z --dry-run   # affiche l'unité, n'installe rien
   orca-ide serve service install --port 6768 --pairing-address 100.x.y.z \
     --command /usr/local/bin/orca-ide
   sudo loginctl enable-linger holo-rgaa  # sinon le service s'arrête à la déconnexion
   # après un changement de version, redémarrer l'unité systemd générée
   systemctl --user restart <nom-de-l-unite>
   orca-ide serve service status
   orca-ide pair --address 100.x.y.z     # génère le lien sans redémarrer le serveur
   ```
4. Comptes et skills côté serveur :
   ```bash
   orca-ide account add --agent claude
   orca-ide account add --agent codex
   orca-ide account list
   orca-ide skills install --skill orca-cli --skill orchestration
   ```

Sur le **poste local** (Tailscale connecté au même tailnet) : **Settings → Remote
Orca Servers → Add Server**, donner un nom, coller le lien, **Add Server**, puis
**Connect**.

Sans Tailscale, passez par un tunnel SSH plutôt que d'ouvrir le port. Lancez
`orca-ide serve --bind 127.0.0.1 --port 6768 --pairing-address 127.0.0.1` sur le VPS,
puis `ssh -N -L 6768:127.0.0.1:6768 holo-rgaa` sur le poste. Je n'ai pas testé ce
montage, et la documentation déconseille `127.0.0.1` comme adresse annoncée à un
client distant. Tailscale reste la voie documentée.

**Ne jamais** utiliser `--allow-public` ni `--bind 0.0.0.0` avec le port ouvert sur
Internet.

## 5. Vérifications et pannes connues

```bash
orca-ide status        # runtimeState: ready, runtimeReachable: true
orca-ide agent installed   # claude, codex, opencode, pi
```

- Le message `degraded: Browser automation has no backend` est attendu sur un VPS
  sans bureau. Pour utiliser le navigateur intégré, définir
  `ORCA_BROWSER_EXECUTABLE` vers un Chromium.
- « Incompatible protocol version » : mettre à jour Orca sur le poste **et** sur le
  serveur.
- Clé d'hôte modifiée (VPS réinstallé) : `ssh-keygen -R <IP>` sur le poste.
- VPS cloné depuis une image : `orca serve reset-identity` sur chaque copie.
- Ne copiez pas `~/.orca/` ni `~/.orca-remote/` d'un serveur à l'autre : ils
  contiennent des jetons propres à l'hôte.
