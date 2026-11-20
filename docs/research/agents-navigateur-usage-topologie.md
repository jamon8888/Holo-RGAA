# Agents navigateur : conditions d'usage, disponibilité UE et topologie MCP (ticket #403, part of #401)

Date de consultation de toutes les sources : **2026-11-20** · Branche : `research/agents-navigateur-topologie`.
Contexte : PRD `docs/strategy/PRD-holo-rgaa-europe.md` (branche `116zoo/business-rgaa`), §6 « Topologie à
vérifier », §13 risque « conditions d'usage », §14 question 7, exigence F-ACC-5.

Périmètre : sources primaires uniquement (conditions, politiques et documentation officielles d'Anthropic et
d'OpenAI). Les citations sont reproduites en anglais, telles que lues. Ce document rapporte des textes ; il ne
constitue pas un avis juridique. Les pages d'aide Anthropic n'affichent pas de date de mise à jour précise
(« Updated this week / today ») : le contenu peut avoir changé depuis la consultation.

Note de vocabulaire : le produit Anthropic s'appelle **Claude in Chrome** (le PRD dit « Claude for Chrome ») ;
le produit OpenAI est l'**extension Chrome de Codex** (« Codex Chrome extension »).

---

## Résumé

| Question | Claude in Chrome / computer use (Anthropic) | Codex Chrome extension / Computer Use (OpenAI) |
|---|---|---|
| (a) Client qui apporte son abonnement | Aucune clause ne vise ce montage. Restrictions trouvées : accès automatisé (conditions grand public), revente, produits concurrents, règle de divulgation IA pour les agents (AUP). | Texte des conditions non lisible (HTTP 403) ; seule l'existence d'une clause « extraction automatique ou programmatique » est confirmée indirectement. |
| (b) UE | Les 27 États membres figurent dans la liste des pays pris en charge (apps incluses). Aucune restriction régionale mentionnée pour Chrome ni computer use. | Lancé hors UE/R.-U. ; annonce du 2026-06-16 (citant le changelog Codex) : disponible EEE, R.-U., Suisse. Changelog officiel lui-même non relu. |
| (c) Topologie | Connecteurs distants (claude.ai, Cowork, Desktop, donc side panel Chrome) : **depuis l'infrastructure Anthropic**, serveur public sur 443. Claude Code (terminal) : serveur ajouté localement, connecteurs claude.ai récupérés par Claude Code. | App/CLI/IDE Codex : config `config.toml` **de l'hôte local**, OAuth avec rappel `127.0.0.1`. ChatGPT web : via plugins, serveur public HTTPS (IP ChatGPT publiées, mTLS optionnel). |
| (d) Données de page | Captures d'écran, texte de page, console, requêtes réseau, état du DOM ; copie possible de capture conservée par la vérification automatique. | Texte lu, captures, appels d'outils stockés « quand ils deviennent contexte » ; pas d'enregistrement complet séparé des actions. |

---

## (a) Conditions d'usage pour un service tiers (le client apporte son abonnement)

### Anthropic

1. **Pays et produits couverts.** La politique des pays pris en charge s'applique aux apps (Claude.ai, Claude
   Code, Cowork), à l'API et aux modèles via clouds et revendeurs ; effective le 12 nov. 2026.
   — https://www.anthropic.com/supported-countries (consulté 2026-11-20)

2. **Conditions grand public (Consumer Terms)** — version servie lors de la consultation : date d'effet
   **8 oct. 2025**.
   - Accès automatisé (section 3, item 7) : « Except when you are accessing our Services via an Anthropic API
     Key or where we otherwise explicitly permit it, » … « to access the Services through automated or
     non-human means, whether through a bot, script, or otherwise. »
   - Crawling/scraping interdit « other than as permitted under these Terms » (section 3, item 4).
   - Revente : « or resell the Services. » (section 3, item 2) ; partage de compte interdit : « You may not
     share your Account login information … or make your Account available to anyone else. » (section 2).
   - Concurrence : « To develop any products or services that compete with our Services, … »
   - Fonction « Actions » (section 4) : « enable the Services to take actions on your behalf, such as software
     manipulation, data processing, and system interactions ».
   - Intégrations tierces (section 7) : « Your use of any Third-Party Content, services, and integrations is at
     your own risk ».
   - **Usage commercial** : section 11 « Non-commercial use only » : « You agree not to use our Services for any
     commercial or business purposes ». Lors d'une seconde lecture de la même URL, la page servie s'ouvrait par
     « These Terms apply to you if you are a consumer who is resident in the United Kingdom. » — voir
     « Non trouvé / ambigu ».
   — https://www.anthropic.com/legal/consumer-terms (consulté 2026-11-20)

3. **Conditions commerciales (Commercial Terms)** — date affichée **17 juin 2025** ; s'appliquent quand le
   client agit pour une organisation (Team, Enterprise, API).
   - A.1 : usage autorisé « including to power products and services Customer makes available to its own
     customers and end users ».
   - D.4(a) : interdiction de « resell the Services except as expressly approved by Anthropic » et
     d'« access the Services to build a competing product or service ».
   - A.2 : « Third Party Features are not Services and, accordingly, Anthropic is not responsible for them. »
   - Pas de mention de MCP ni de connecteurs ; l'AUP est incorporée par référence (D.2).
   — https://www.anthropic.com/legal/commercial-terms (consulté 2026-11-20)

4. **Politique d'usage (AUP)** — effective **12 nov. 2026**.
   - Agents : « Users are responsible for ensuring that agents they build or deploy » respectent la politique,
     y compris pour les actions « through tools, browsers, or connected systems ».
   - « Engage in actions or behaviors that circumvent the guardrails or terms of other platforms or services. »
     (abus de plateforme) ; « Bypass security controls such as authentication systems … without
     authorization. »
   - Produits intégrant Claude : les chatbots et agents « must disclose to users that they are interacting with
     AI rather than a human », au minimum en début de session ou dans l'interface ; « Users do not need to
     identify Anthropic, Claude, or the model used. »
   - Cas à haut risque : « A qualified person must meaningfully review a High-risk AI Recommendation… »
   - Aucune règle générale sur le scraping de sites tiers.
   — https://www.anthropic.com/legal/aup (consulté 2026-11-20)

5. **Restrictions produit de Claude in Chrome.** Interdits : « Engaging in stock trading or investment
   transactions », « Bypassing captchas », « Inputting sensitive data », « Gathering or scraping facial
   images » ; sites adultes et pirates bloqués ; permission demandée avant les sites financiers. Sur
   Team/Enterprise, l'admin peut restreindre les sites (allowlists/blocklists). Off par défaut pour les
   organisations HIPAA.
   — https://support.claude.com/en/articles/12902428-use-claude-in-chrome-safely ;
   https://support.claude.com/en/articles/12012173-get-started-with-claude-in-chrome (consultés 2026-11-20)

6. **Plans.** Claude in Chrome : « paid plans (Pro, Max, Ultra, Team, and Enterprise) », Chrome uniquement
   (pas d'autres Chromium, pas de mobile, selon l'article d'aide). Claude Code + Chrome exige un plan
   Anthropic direct et une connexion `/login` ; indisponible par clé API, `setup-token`, Bedrock, Vertex,
   Foundry ; désactivé si ZDR pour Claude Code.
   — https://support.claude.com/en/articles/12012173-get-started-with-claude-in-chrome ;
   https://code.claude.com/docs/en/chrome (consultés 2026-11-20)

7. **Computer use.** Pro, Max, Ultra : dans Cowork et Claude Code, Claude Desktop macOS/Windows. Team/Enterprise :
   « in beta and off until an owner of your organization turns it on », déploiement progressif, uniquement
   dans la « nouvelle expérience » unifiée ; impossible avec HIPAA, ZDR Claude Code, clés gérées par le client,
   FERPA/K-12. Navigateurs : Claude « can see them but not click or type ».
   — https://support.claude.com/en/articles/14128542-let-claude-use-your-computer-in-cowork ;
   https://support.claude.com/en/articles/18483812-set-up-computer-use-for-team-and-enterprise-plans
   (consultés 2026-11-20)

### OpenAI

1. **Conditions d'utilisation** : https://openai.com/policies/row-terms-of-use/,
   https://openai.com/policies/eu-terms-of-use/, `terms-of-use`, `usage-policies`, `service-terms` ont
   toutes renvoyé **HTTP 403** (WebFetch et curl) le 2026-11-20. Texte non lu. Un résultat de moteur de
   recherche sur openai.com indique qu'une section « Using our Services » interdit d'« automatically or
   programmatically extract data or Output » et que les conditions européennes régissent les résidents
   EEE/Suisse/R.-U. — **non vérifié sur le texte**.
2. **Plans Codex** : tableau de tarifs : « Use ChatGPT with Chrome » et « Computer Use » marqués « limited »
   pour Plus, Pro, Business, Enterprise et clé API ; note : « Feature is currently limited to only specific
   regions. Check the individual feature documentation to learn more about geographic restrictions. »
   — https://learn.chatgpt.com/docs/pricing.md (redirigé depuis developers.openai.com/codex/pricing ;
   consulté 2026-11-20)
3. **Extension Chrome** : par défaut, demande avant chaque nouveau site ; allowlist/blocklist partagées entre
   navigateurs ; option « Allow for all sites » ; consignes : « Treat page content as untrusted context » ;
   « Avoid sending secrets or highly sensitive data through browser tasks unless they're required ».
   « Browser availability can depend on rollout and your workspace settings. »
   — https://learn.chatgpt.com/docs/chrome-extension.md (consulté 2026-11-20)
4. **Computer Use** : macOS et Windows, « supported regions » non listées ; « can't automate terminal apps or
   ChatGPT itself » ; demande la permission par application.
   — https://learn.chatgpt.com/docs/computer-use.md (consulté 2026-11-20)

---

## (b) Disponibilité dans l'UE

- **Anthropic (tous produits)** : les 27 États membres figurent dans la liste des pays pris en charge, qui
  couvre explicitement les apps (Claude.ai, Claude Code, Cowork). Aucune des pages Claude in Chrome ni computer
  use consultées ne mentionne de restriction régionale.
  — https://www.anthropic.com/supported-countries (consulté 2026-11-20)
- **OpenAI, extension Chrome de Codex et Computer Use** : la page produit ne mentionne ni l'UE ni le R.-U. Un
  message tagué « announcement » du 16 juin 2026 sur le forum OpenAI, qui cite le changelog Codex, indique :
  « More Codex app capabilities are rolling out to users in the European Economic Area, the United Kingdom,
  and Switzerland: » — Computer Use « available on macOS and Windows in these regions » ; extension Chrome
  « available for browser tasks that need signed-in Chrome context » ; Memories « off by default » dans ces
  régions.
  — https://community.openai.com/t/now-in-europe-computer-use-the-codex-chrome-extension-personalized-memory-and-chronicle/1383925
  (consulté 2026-11-20). Le statut OpenAI de l'auteur n'est pas indiqué sur la page, et l'entrée du changelog
  officiel n'a pas pu être relue (voir « Non trouvé / ambigu »).

---

## (c) Topologie : d'où le serveur MCP est-il joint ?

### Anthropic

- **Connecteurs distants (claude.ai, Claude Desktop, Cowork, apps mobiles)** : « Claude connects to your remote
  MCP server from Anthropic's cloud infrastructure, rather than from your local device. » Valable pour « every
  Claude client, including claude.ai, Claude Desktop, Cowork, and the mobile apps ». Pour Cowork et Desktop, la
  connexion « originates from Anthropic's servers, not from your machine's network interface ».
  Conséquences écrites : « your MCP server must be reachable over the public internet from Anthropic's IP
  ranges » ; un serveur derrière VPN ou pare-feu « won't connect » ; port 443 obligatoire (« Claude can't
  connect to a URL that specifies another port ») ; pour un réseau privé, allowlister les IP d'Anthropic.
  — https://support.claude.com/en/articles/11175166-getting-started-with-custom-connectors-using-remote-mcp
  (consulté 2026-11-20)
- **Authentification des connecteurs distants** : modes « Sign in now », « Sign in when needed », « No sign in »
  (le sans-authentification existe) ; client OAuth : « Use Claude's published identity (Recommended) »,
  « Register automatically », « Use your own OAuth client » ; ou en-têtes fixes (clé API, bearer) « that Claude
  sends on every request », stockés chiffrés. Plans : Free (1 connecteur), Pro, Max, Ultra, Team, Enterprise ;
  sur Team/Enterprise seuls les Owners/Primary Owners ajoutent des connecteurs. (même source)
- **Side panel Claude in Chrome** : « The side panel runs as a Claude Cowork session. » ; l'extension
  s'identifie « to Anthropic's servers ». — https://support.claude.com/en/articles/12012173-get-started-with-claude-in-chrome
  Combiné à la ligne précédente (Cowork → depuis l'infra Anthropic), le side panel joindrait donc un MCP
  distant depuis l'infrastructure Anthropic. **Inférence** : aucune page ne le dit pour Chrome nommément.
- **Serveurs MCP locaux de Claude Desktop** (`claude_desktop_config.json`) : « a separate mechanism », ils « do
  use your local network », indisponibles dans claude.ai et dans les sessions Cowork cloud. (source connecteurs)
- **Claude Code + extension Chrome** : liaison par un hôte *native messaging* installé localement
  (`com.anthropic.claude_code_browser_extension.json`) **et** par `bridge.claudeusercontent.com` (« your
  organization's IP allowlist rejects the connection to `bridge.claudeusercontent.com` » figure parmi les causes
  d'erreur). Dans Claude Code, l'extension apparaît comme serveur MCP `claude-in-chrome`.
  — https://code.claude.com/docs/en/chrome (consulté 2026-11-20)
- **Claude Code, serveurs MCP HTTP ajoutés localement** : OAuth 2.0 pris en charge, rappel par défaut sur un port
  aléatoire, `--callback-port` pour un redirect URI pré-enregistré `http://localhost:PORT/callback`.
  Les connecteurs claude.ai : sessions terminal/VS Code/JetBrains/Agent SDK → « Claude Code fetches them from
  claude.ai » ; sessions cloud → « The cloud host passes them in » et « The session's proxy rewrites each
  connector's URL » ; Desktop local/SSH → « The desktop app delivers them in-process ».
  — https://code.claude.com/docs/en/mcp (consulté 2026-11-20). La page ne dit pas explicitement si un serveur
  ajouté par `claude mcp add --transport http` est joint depuis le poste.

### OpenAI

- **Codex app, CLI, extension IDE** : partagent la configuration MCP « for the same Codex host »
  (`config.toml`) ; deux transports : STDIO local et « Streamable HTTP servers: Servers that you access at an
  address » ; OAuth (dont CIMD) via `codex mcp login <server-name>`, rappel par défaut
  `http://127.0.0.1/callback`, liaison « to the local interface » ; ou `bearer_token_env_var`, `http_headers`.
  « Local MCP servers and transports may not be available in the cloud » ; « ChatGPT web doesn't read local
  Codex configuration files ».
  — https://learn.chatgpt.com/docs/extend/mcp.md (consulté 2026-11-20)
- **Extension Chrome de Codex** : la page ne mentionne pas MCP ; permission Chrome « Communicate with
  cooperating native applications » (native messaging). Le chemin extension → app Codex n'est pas décrit.
  — https://learn.chatgpt.com/docs/chrome-extension.md (consulté 2026-11-20)
- **ChatGPT web (plugins / apps)** : pour une soumission publique, « deploy the MCP server at a stable,
  publicly reachable HTTPS endpoint » ; tunnel temporaire ou endpoint local exclus ; « Secure MCP Tunnel » pour
  un serveur privé (hors soumission publique) ; OAuth 2.1 si authentification utilisateur ; mTLS géré par
  OpenAI ; « use the published ChatGPT connectors IP ranges » ; une allowlist IP ne remplace pas
  l'authentification.
  — https://developers.openai.com/apps-sdk/deploy (consulté 2026-11-20)

---

## (d) Données de page envoyées à l'éditeur

### Anthropic
- Claude in Chrome : « clicking buttons, typing text, and taking screenshots » ; lecture du texte des pages ;
  « Claude can read browser console output, including errors, network requests, and DOM state. »
  — https://support.claude.com/en/articles/12012173-get-started-with-claude-in-chrome
- « Whatever is visible in one of those tabs is captured in the screenshots and becomes part of the
  conversation. » ; à l'ouverture du side panel, captures de l'onglet actif ; vérification automatique des
  actions : « Anthropic can keep a copy of that screenshot in its record of the check. » ; sessions du side
  panel sauvegardées dans l'historique et ouvrables sur d'autres appareils. Pas de durée de conservation, pas de
  mention de l'entraînement.
  — https://support.claude.com/en/articles/12902428-use-claude-in-chrome-safely
- Computer use : captures de l'écran entier, « saved with the chat » ; même copie possible par la vérification
  automatique ; Team/Enterprise : données régies par les Commercial Terms et le DPA, pas de durée indiquée.
  — https://support.claude.com/en/articles/18483812-set-up-computer-use-for-team-and-enterprise-plans
- Claude Code + Chrome : console, DOM, enregistrements GIF (« captures everything visible in the browser,
  including account details »), envoi de fichiers locaux vers des champs de page (≤ 10 Mo).
  — https://code.claude.com/docs/en/chrome

### OpenAI
- « OpenAI stores browser activity only when it becomes part of the ChatGPT context, such as text ChatGPT reads
  from a page, screenshots, tool calls » ; « OpenAI doesn't store a separate complete record of your browser
  actions from the extension. » ; « Your ChatGPT data controls apply to content processed in context. » ; suit le
  réglage Memories. DOM non mentionné.
  — https://learn.chatgpt.com/docs/chrome-extension.md
- Computer Use : « Your ChatGPT data controls apply to content processed through ChatGPT, including screenshots
  taken by Computer Use. » Pas de durée de conservation.
  — https://learn.chatgpt.com/docs/computer-use.md

---

## Non trouvé / ambigu

1. **Conditions OpenAI non lues.** Toutes les pages `openai.com/policies/*` et l'article d'aide
   `help.openai.com/en/articles/12584461` ont renvoyé HTTP 403. Rien n'est affirmé ici sur la revente, le partage
   de compte ou l'accès automatisé côté OpenAI. À relire par un humain dans un navigateur.
2. **Version des Consumer Terms Anthropic.** La même URL a servi une version datée du 8 oct. 2025 puis un texte
   introduit par « apply to you if you are a consumer who is resident in the United Kingdom » ; la version
   servie aux résidents UE n'a pas été identifiée avec certitude. La portée de « Non-commercial use only » pour
   un client Pro/Max qui utilise Claude pour son activité n'est donc pas établie.
3. **Aucune clause, chez l'un ou l'autre éditeur, ne traite explicitement le montage « le client apporte son
   propre abonnement et le connecte au MCP d'un éditeur tiers ».** Les textes Anthropic encadrent l'accès
   automatisé (Consumer Terms), la revente (Consumer et Commercial Terms) et la divulgation IA (AUP) ; ils ne
   disent pas si un connecteur MCP tiers utilisé par l'abonné relève de « Third-Party Content / Features ».
4. **Topologie du side panel Claude in Chrome** : déduite (« runs as a Claude Cowork session » + connecteurs
   Cowork depuis l'infra Anthropic), non écrite.
5. **Extension Chrome de Codex** : la page ne dit pas si elle accède aux serveurs MCP ni comment elle parle à
   l'app Codex (seule la permission native messaging est visible). Le changelog officiel
   (`learn.chatgpt.com/docs/changelog`, 219 781 caractères) n'a été lu qu'en partie ; l'entrée EEE de juin 2026
   n'y a pas été retrouvée — la disponibilité UE repose sur un message du forum OpenAI citant ce changelog.
6. **Plateformes de Claude in Chrome** : l'article d'aide dit « not supported on other Chromium-based web
   browsers », la doc Claude Code dit que l'intégration Claude Code fonctionne avec Edge, Brave, Arc, etc.
7. **Computer use Team/Enterprise** : en bêta, « rolling out to organizations gradually » ; pas de calendrier.
8. **Plages IP Anthropic** : la page des plages IP n'a pas été consultée.
9. **Conservation et entraînement** : aucune durée de conservation ni règle d'entraînement spécifique aux
   captures n'a été trouvée sur les pages produit des deux éditeurs.

---

## Implication pour notre MCP (F-ACC-5, PRD §6)

Constats tirés des textes ci-dessus, à trancher par le produit :

- **Claude in Chrome (side panel), claude.ai, Claude Desktop, Cowork** : un connecteur MCP distant est joint
  depuis l'infrastructure Anthropic. Notre serveur devrait alors être **exposé publiquement en HTTPS sur 443**,
  joignable depuis les plages IP d'Anthropic. Une allowlist d'origines `chrome-extension://…` (F-ACC-5) ne
  s'appliquerait pas à ces appels : ils ne viennent pas du navigateur. L'authentification passerait par OAuth
  (identité publiée de Claude, enregistrement automatique ou client propre) ou par en-têtes fixes ; le mode sans
  authentification existe mais exposerait un serveur public.
- **Claude Code (terminal, IDE) et Codex (app, CLI, IDE)** : serveurs MCP configurés sur l'hôte local, avec un
  OAuth dont le rappel arrive sur `localhost` / `127.0.0.1`. Le « plan local » du PRD reste compatible avec ces
  clients ; l'extension Chrome y sert d'outil au client, pas de client MCP vers notre serveur.
- **ChatGPT web** : serveur public HTTPS, ou Secure MCP Tunnel pour un serveur privé ; mTLS OpenAI disponible ;
  IP ChatGPT publiées.
- **Fiche de flux de données (P-1, F-SEC-5)** : les textes confirment que captures, texte de page et, chez
  Anthropic, console/réseau/DOM partent vers l'éditeur de l'agent, avec une copie de capture possiblement
  conservée par la vérification automatique d'Anthropic.
- **Avant M2** : relire à la main les conditions OpenAI (403 ici) et la version UE des Consumer Terms
  d'Anthropic ; la question « abonnement du client connecté au MCP d'un éditeur » reste sans réponse écrite.
