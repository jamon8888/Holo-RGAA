# PRD — Holo-RGAA Europe : SaaS minimal, plugins agents, Mistral Large 4, GitHub

| | |
|---|---|
| **Statut** | Brouillon v1 — à valider (2026-11-20) |
| **Portée** | Produit, offre, exigences fonctionnelles et non fonctionnelles, jalons. Pas de code. |
| **Entrées** | `Strategie_Rust_accessibilite_Europe.pdf` (nov. 2026, 16 p.) · code `rgaa-rs/` (lu le 2026-11-20) · documentation officielle Mistral (consultée le 2026-11-20) |
| **Documents liés** | `offre-europe-saas-plugins.md` (offre) · `p0-et-mistral-large-4.md` (analyse détaillée du code, défauts B.1–B.8) · `docs/specs/human-criteria-automation.md` (règles de revue) · `GLOSSARY.md` |
| **Vocabulaire** | Termes du glossaire en italique : *Test RGAA*, *Niveau d'automatisation* (*Automatique*, *Assisté*, *Irréductible*), *Famille de preuve*, *Calibration*. |

Convention de preuve : **[code]** constaté dans le repo · **[PDF]** affirmé par le document de stratégie, non revérifié ·
**[Mistral]** documentation officielle · **[hypothèse]** à valider · **[ouvert]** question sans réponse.

---

## 1. Résumé

Holo-RGAA devient une **plateforme d'audit et de remédiation d'accessibilité pilotée par des agents**. Le cœur
(moteur Rust, CLI, MCP, API) reste local et ouvert. Un **SaaS minimal** fournit seulement ce qui engage
l'organisation : comptes, clés, mesure et refacturation de la consommation IA (**Mistral Large 4**), historique,
rapports signés. **Toute l'expérience utilisateur passe par des plugins** exécutés dans les clients agents (Claude Code
Desktop, Codex, Claude for Chrome, Codex for Chrome) branchés sur notre MCP / CLI / API. La revue humaine et la
remédiation sont **guidées visuellement, pas à pas** dans le navigateur. Une **GitHub Action** porte l'audit et la
remédiation dans la CI des clients.

Le produit vend un **instrument de mise en conformité** (détection, correction, preuve), **jamais une certification**.

## 2. Problème et opportunité

- **Obligation légale de masse** : l'European Accessibility Act s'applique depuis le 28 juin 2025 au commerce
  électronique, à la banque, au transport, aux télécoms ; en France, sanctions jusqu'à 50 000 € par service,
  renouvelables tous les six mois [PDF §2.1].
- **Le web échoue massivement** : 94,8 % des pages d'accueil ont au moins un échec WCAG, 51 erreurs en moyenne
  (WebAIM 2025) [PDF §2.1].
- **Les agents de code aggravent la dette** : 94,59 % des modèles récents produisent du HTML à défauts critiques ou
  sérieux (Deque 2026) [PDF §2.2].
- **Souveraineté** : l'exécution locale vérifiable devient un critère d'achat (banque, santé, public) [PDF §2.3].
- **Plafond de l'automatisation** : ~57 % des problèmes sont détectables automatiquement [PDF §1.2] ; dans notre
  propre mesure, sur 117 *Tests RGAA* réputés humains, 16 sont *Automatiques*, 82 *Assistés*, 19 *Irréductibles*
  [code : `docs/specs/human-criteria-automation.md` §1]. **La part humaine est structurelle : le produit doit
  l'outiller, pas la nier.**

**Espace libre** [PDF §3.2] : aucun des Leaders Gartner 2026 (Level Access, Deque, Siteimprove) ne combine exécution
locale vérifiable et intégration agent-native profonde. Notre ajout : RGAA natif, 12 pays, et revue visuelle guidée.

## 3. Objectifs et non-objectifs

### Objectifs (18 mois)

| ID | Objectif | Mesure de succès |
|---|---|---|
| O1 | Un développeur obtient un premier audit et un premier correctif vérifié dans son agent | < 10 min depuis l'installation (critère du PDF §4.2) |
| O2 | Un auditeur traite la file de revue *Assistée* plus vite qu'un audit manuel | Temps par *Test RGAA* revu, mesuré sur pilotes ; cible à fixer après mesure de référence [hypothèse] |
| O3 | La consommation IA est refacturée avec marge positive et sans surprise | 100 % des appels de modèle tracés ; écart facture fournisseur / mesure interne < 2 % [hypothèse] |
| O4 | La CI client bloque les régressions d'accessibilité | Action GitHub installable en 1 fichier ; PR de régression marquée en échec |
| O5 | Preuve opposable | Chaque rapport exporté est signé et vérifiable hors ligne |
| O6 | Phases du PDF | P1 adoption · P2 5-10 pilotes FR + 3-5 cabinets · P3 DE/Benelux [PDF §6] |

### Non-objectifs (explicites)

- **Pas d'overlay** ni de correctif injecté à l'exécution [PDF §1.2, §5.2]. Toute correction est faite au code source.
- **Pas de « conformité garantie »**, pas de certification [PDF §5.2, §9].
- **Pas d'interface SaaS riche** : le SaaS n'a pas d'éditeur d'audit ; l'UX vit dans les plugins.
- **Aucun `pass` ni `fail` décidé par un modèle sur un test *Assisté*** (voir §7).
- **Pas de vente d'audit humain** par l'éditeur : c'est le rôle des cabinets partenaires [PDF §7.3].
- Hors périmètre v1 : application mobile, PDF/UA, sandbox de plugins WASM (absente du code), SSO (Enterprise, v2).

## 4. Personas

| Persona | Contexte | Besoin principal | Surface |
|---|---|---|---|
| **Dev produit** | Équipe 5-50 devs, utilise Claude Code ou Codex | Ne pas introduire de défauts ; corriger vite | Plugin agent, CLI, Action GitHub |
| **Auditeur·rice** (cabinet ou interne) | Réalise l'audit RGAA, signe le rapport | Preuves rassemblées, file ordonnée, décision tracée | Claude for Chrome / Codex for Chrome + plugin revue |
| **Responsable conformité** | Banque, assurance, e-commerce, public | Déclaration, taux, historique, preuve | Rapports exportés, tableau minimal du SaaS |
| **Lead DevOps** | Possède la CI | Gate fiable, binaire vérifié, pas de fuite de code | Action GitHub |
| **Admin de compte** | Côté client | Clés, budgets, factures | SaaS (console minimale) |
| **Partenaire cabinet** | Access42, Temesis, Ideance, Funka (profils illustratifs [PDF §7]) | Industrialiser l'audit, revendre | Licence partenaire, tarif revendeur IA |

## 5. Existant (constaté dans le code)

| Brique | État | Référence |
|---|---|---|
| Catalogue 106 critères, mapping axe | Livré | `rgaa-core`, `rgaa-rules`, `rgaa-data` |
| Pipeline d'audit (axe + gap-fix + LLM + clavier) | Livré | `rgaa-orchestrator`, `rgaa-obscura` (Obscura 0.2.4 épinglé, SHA-256) |
| CLI `rgaa audit analyze / igt / verify / report / review / policy` ; formats json, markdown, sarif, junit, html | Livré | `rgaa-cli/src/commands/` |
| MCP : `analyze`, `audit_url`, `get_audit_result`, `remediate`, `verify_fix`, `source_map`, `lint_static`, `list_criteria`, `igt` (déprécié) | Livré, stdio + HTTP | `rgaa-mcp/src/server.rs`, `rgaa-mcp-http` |
| Plugins : `rgaa-consultant` (Claude Code : 7 skills, 3 commandes, 4 agents, hooks), `rgaa-codex` (7 skills), `acces-accessibility` (4 skills) | Livré | `rgaa-rs/plugins/` |
| Remédiation par framework + re-vérification | Livré | `rgaa-remediation`, outil `verify_fix` |
| Revue humaine enregistrée dans l'audit | Livré (CLI) | `rgaa audit review` |
| Documents officiels 12 pays (déclaration, PDF, JSON pivot) | Livré | `rgaa-report` |
| API REST (bundles, findings, policy, batch, webhooks signés) | Livré, **mono-tenant** | `rgaa-api` |
| Multi-fournisseur LLM, preset `mistral` | Livré, **inutilisable avec Large 4 en l'état** | `rgaa-core/src/provider.rs` — défauts B.1–B.8 |
| Mesure de tokens | Partielle : événement `tracing` côté `rig` seulement | `rgaa-agent/src/metrics.rs` |
| Organisations, quotas, facturation, signature de rapports, Action GitHub, binaire lint | **Absents** | voir §9 |

**Correctif de sécurité préalable** [code] : `POST /audit` et `GET /audit/:id` sont exposés **sans authentification**
(`rgaa-api/src/lib.rs`, `legacy_routes`), avec CORS ouvert. Bloquant pour toute mise en ligne.

## 6. Architecture produit cible

```
 Clients agents (UX)                          Plan local (client)                    SaaS minimal (UE)
 ┌──────────────────────────┐   MCP/CLI   ┌──────────────────────────┐   HTTPS   ┌──────────────────────────┐
 │ Claude Code Desktop      │────────────▶│ rgaa-mcp / rgaa-cli      │──────────▶│ Comptes, organisations   │
 │ Codex                    │             │ rgaa-orchestrator        │  (option) │ Clés + scopes            │
 │ Claude for Chrome        │──MCP HTTP──▶│ Obscura (navigateur)     │           │ Passerelle LLM mesurée   │──▶ Mistral
 │ Codex for Chrome         │             │ rgaa-remediation/report  │           │ Usage, budgets, factures │    Large 4
 │ + computer use           │             └──────────────────────────┘           │ Historique, signature    │
 └──────────────────────────┘                        ▲                           └──────────────────────────┘
                                   GitHub Action ────┘ (même binaire, même MCP)
```

Principes :
- **P-1 Local par défaut.** Le scan, le DOM, les captures et le code restent chez le client. Seuls sortent : les appels
  LLM (si activés) et, si le client l'active, le bundle d'audit pour historique/signature.
- **P-2 Une seule vérité.** Plugins, Action et SaaS consomment le même bundle d'audit ; aucune surface ne recalcule un
  verdict.
- **P-3 IA mesurée.** Tout appel de modèle passe par une route qui enregistre l'usage (passerelle SaaS, ou mesure
  locale reportée si le client utilise sa propre clé).
- **P-4 Trois modes IA** : `off` (déterministe seul) · `byok` (clé Mistral du client, 0 % de marge) · `managed`
  (passerelle SaaS, refacturé).

## 7. Règles de décision non négociables

Reprises de `docs/specs/human-criteria-automation.md` §2 ; elles s'imposent à **toutes** les surfaces, y compris le
computer use et Claude for Chrome :

- **R1** Un *Test RGAA* *Assisté* rend `review`. Jamais `pass` ni `fail`, quelle que soit la confiance.
- **R2** Aucun `fail` ne provient d'un verdict de modèle (mesuré : 8 % de faux « non pertinent »).
- **R3** L'auditeur voit la preuve et les conditions structurées, **pas** le verdict du modèle (mesuré : 19 % de
  mauvaises alternatives validées à tort). Le verdict sert uniquement à **ordonner la file**.
- **R4** Le computer use **provoque des états et enrichit la preuve** ; il ne fait monter aucun test d'un niveau.
- **R5** Toute décision humaine est horodatée, attribuée (identité), et versionnée dans le bundle.
- **R6** Un patch n'est jamais appliqué sans validation humaine ; en CI, il prend la forme d'une PR, jamais d'un push.

## 8. Offre et tarification

### 8.1 Paliers

| Palier | Contenu | Prix [hypothèse, à valider par étude de prix — PDF §11.1] |
|---|---|---|
| **Community** (Apache-2.0) | Moteur, CLI, MCP, linter, plugins Claude Code/Codex, Action GitHub mode déterministe, mode IA `byok` | Gratuit |
| **Team** | + passerelle IA `managed`, historique hébergé UE, rapports HTML/PDF, file de revue partagée, support | Abonnement par dépôt ou par dev / mois + crédits IA |
| **Enterprise** | + multi-sites, rapports signés longue durée, déclarations 12 pays certifiées à jour, SSO/RBAC (v2), SLA, Jira (v2) | Contrat annuel par portefeuille + IA au coût majoré |
| **Partenaire** | Licence Team offerte, tarif revendeur IA, certification, listing | Revenue share sur licences induites [PDF §7.3] |

Publier la grille Team et une grille indicative Enterprise (différenciateur face à Deque et Level Access, prix non
publics [PDF §5.2]).

### 8.2 Refacturation de la consommation IA

- **Unité facturée** : le *crédit*, converti depuis les tokens via un **barème versionné** (`price_book`) par modèle.
- **Barème initial** `mistral-large-4` [Mistral, page modèle datée du 2026-11-05] : 0,70 $ / 1M tokens en entrée,
  0,07 $ en entrée en cache, 2,10 $ en sortie. Le lancement a eu « -50 % pendant 2 semaines » ; la page ne précise
  pas si ce prix est définitif → **revue mensuelle obligatoire du barème** [ouvert].
- **Le raisonnement est facturé en sortie** : Large 4 raisonne toujours, `reasoning_effort` vaut `high` par défaut
  [Mistral, changelog 2026-10-29]. Le niveau d'effort est donc un **levier de prix** à exposer et à calibrer.
- **Formule** : prix client = Σ (tokens × prix fournisseur) × (1 + marge du palier). Marge `byok` = 0.
- **Garde-fous** : estimation affichée **avant** un audit complet ; budget par audit et par organisation vérifié
  **avant** chaque appel ; dépassement → le test reste `review` avec la cause `budget` (jamais de verdict inventé) ;
  alertes à 50/80/100 %.
- **Illustration arithmétique** [hypothèse de tailles] : 6 000 tokens en entrée + 1 500 en sortie ≈ 0,0074 $ par
  appel. **Aucune mesure réelle de tokens par audit n'existe dans le repo** : la grille ne peut pas être fixée avant la
  mesure (F-BILL-1).

## 9. Exigences fonctionnelles

Priorités : **P0** requis pour le premier pilote payant · **P1** requis pour la grille Team publique · **P2** Enterprise.

### 9.1 IA — Mistral Large 4 (`F-LLM`)

| ID | Exigence | Prio | Critère d'acceptation |
|---|---|---|---|
| F-LLM-1 | Le preset `mistral` a pour modèle par défaut **`mistral-large-4`** (alias daté, pas `mistral-large-latest`) | P0 | Config avec seule `MISTRAL_API_KEY` → modèle `mistral-large-4` |
| F-LLM-2 | Les réponses dont `content` est une **liste de blocs** (`thinking`, `text`) sont lues ; seul le texte compte | P0 | Un bloc `thinking` contenant un faux verdict n'est jamais retenu ; le défaut actuel (B.3) est couvert par un test |
| F-LLM-3 | Paramètre **`reasoning_effort`** (`low`, `medium`, `high`) configurable par tier ; `none` refusé à la configuration | P0 | Valeur visible dans la provenance du verdict ; `none` → erreur explicite avant tout appel |
| F-LLM-4 | Les clés « thinking » (`chat_template_kwargs`, `think`, `enable_thinking`) ne sont jamais envoyées à Mistral | P0 | Configuration explicite refusée avec message clair pour `mistral` |
| F-LLM-5 | **Usage lu sur les deux chemins** (transport brut `rgaa-holo` et agent `rig`) : tokens entrée, cache, sortie | P0 | Chaque appel produit un enregistrement d'usage (F-BILL-2) |
| F-LLM-6 | `max_tokens` dimensionné pour inclure le raisonnement | P0 | Taux de verdicts tronqués = 0 sur le corpus de test |
| F-LLM-7 | Sortie structurée : `json_object` + validation locale par le schéma de verdict ; `json_schema` activé seulement après test réel | P1 | Décision documentée après le test réel (forme brute non trouvée dans la doc) |
| F-LLM-8 | Image : PNG en data URL, taille et dimensions bornées avant envoi | P1 | Limites Mistral documentées ou mesurées [ouvert] |
| F-LLM-9 | **Calibration** de Large 4 par *Famille de preuve* avec le protocole existant avant toute affirmation de fiabilité | P0 | Rapport de calibration publié en interne ; tests restent `review` quoi qu'il arrive (R1) |
| F-LLM-10 | Repli : fournisseur secondaire ou modèle local (`ollama`, `vllm`) via la route de repli existante | P1 | Panne Mistral → audit déterministe complet + tests IA en `review` cause `llm_unavailable` |

### 9.2 SaaS minimal — comptes, clés, usage, facturation (`F-ACC`, `F-BILL`)

| ID | Exigence | Prio | Critère d'acceptation |
|---|---|---|---|
| F-ACC-0 | Les routes `/audit` et `/audit/:id` exigent une clé ; CORS restreint | P0 | Requête sans clé → 401 |
| F-ACC-1 | Entité **organisation** ; toute clé, tout audit, tout finding, tout lot appartient à une organisation | P0 | Une organisation reçoit 404 sur l'audit d'une autre |
| F-ACC-2 | Le contexte d'authentification (organisation, clé, scopes) est transmis aux traitements (aujourd'hui jeté) | P0 | Chaque écriture porte l'organisation de l'appelant |
| F-ACC-3 | Le MCP HTTP authentifie par clé d'organisation (remplace le jeton unique `RGAA_MCP_TOKEN`) | P0 | Deux organisations, deux clés, aucune donnée croisée |
| F-ACC-4 | Origines des extensions (`chrome-extension://…`) ajoutables à l'allowlist par organisation | P1 | Claude for Chrome / Codex for Chrome joignent le MCP sans ouvrir CORS à tous |
| F-ACC-5 | Rotation, expiration, révocation des clés ; scopes `audit:read`, `audit:write`, `llm:use`, `admin` | P1 | Clé révoquée → 401 immédiat |
| F-BILL-1 | **Mesure de référence** : coût réel en tokens d'un audit type (15 pages) par mode IA et par `reasoning_effort` | P0 | Chiffre publié en interne avant de fixer la grille |
| F-BILL-2 | Journal `llm_usage` : organisation, audit, *Test RGAA*, fournisseur, modèle, effort, tokens (entrée, cache, sortie), latence, statut | P0 | Somme du journal = facture fournisseur à ±2 % sur un mois [hypothèse de seuil] |
| F-BILL-3 | Barème `price_book` versionné (date d'effet), relu chaque mois | P0 | Changement de prix sans redéploiement |
| F-BILL-4 | Budgets par audit et par organisation, contrôlés **avant** l'appel | P0 | Dépassement → `review` cause `budget`, jamais d'appel hors budget |
| F-BILL-5 | Estimation de coût affichée avant un audit complet | P1 | Écart estimation / réel mesuré et affiché |
| F-BILL-6 | Export de facturation mensuel ; intégration paiement (Stripe prévu dans la spec de distribution) | P1 | Facture par organisation et par mois |
| F-BILL-7 | Mode `byok` : la clé du client ne transite pas par nos serveurs ; l'usage est mesuré localement | P1 | Vérifiable : aucun appel sortant vers notre passerelle en `byok` |

### 9.3 Plugins agents (`F-PLG`)

Un **plugin = un ensemble de skills + commandes + accès MCP**, distribué pour Claude Code et Codex. Les plugins actuels
(`rgaa-consultant`, `rgaa-codex`, `acces-accessibility`) sont réorganisés en cinq plugins de domaine.

| ID | Plugin | Exigence | Prio | Critère d'acceptation |
|---|---|---|---|---|
| F-PLG-1 | **audit** | Lancer, reprendre, lister un audit (URL, site, projet) ; afficher taux et état | P0 | Parcours O1 < 10 min |
| F-PLG-2 | **remediation** | Relocaliser le finding dans le source (`source_map`), proposer un patch par framework, appliquer après accord, re-vérifier (`verify_fix`) | P0 | Patch non appliqué sans accord (R6) ; re-vérification jointe au patch |
| F-PLG-3 | **ia-assist** | File de revue des tests *Assistés* ordonnée par le signal du modèle ; présente preuve + conditions, **pas** de verdict (R3) | P0 | Aucun écran n'affiche le verdict modèle d'un test *Assisté* |
| F-PLG-4 | **reporting** | Grilles RGAA (106 critères), pages HTML avec captures, déclaration d'accessibilité par pays, export PDF, SARIF/JUnit | P0 | Rendu du bundle existant ; aucune donnée recalculée (P-2) |
| F-PLG-5 | **visual-guide** | Remédiation et revue pas à pas dans le navigateur (voir §9.4) | P1 | Parcours guidé complet sur un critère de chaque groupe G1–G5 |
| F-PLG-6 | Commun | Les plugins n'ont **aucune logique métier** propre : tout passe par MCP/CLI/API | P0 | Un même audit donne le même résultat via plugin, CLI et Action |
| F-PLG-7 | Commun | Instructions d'installation et fichiers `CLAUDE.md` / `AGENTS.md` générés pour le dépôt client | P1 | Installation en une commande pour chaque client agent |

### 9.4 Revue et remédiation visuelles guidées (`F-VIS`)

Clients : **Claude for Chrome**, **Codex for Chrome**, **computer use** — tous branchés sur notre MCP HTTP.

| ID | Exigence | Prio | Critère d'acceptation |
|---|---|---|---|
| F-VIS-1 | Pour chaque test en `review`, l'agent ouvre la page, **met en évidence l'élément** concerné et présente la preuve (capture, DOM, arbre d'accessibilité, contexte) | P1 | La preuve affichée correspond au hash de capture du bundle |
| F-VIS-2 | L'agent **provoque les états** utiles (focus clavier, survol, ouverture de menu, zoom 200 %, contraste) pour 7.4, 7.5, 11.10–11.12, 12.11 et similaires (R4) | P1 | Captures avant/après jointes à la preuve |
| F-VIS-3 | L'humain décide (`conforme`, `non conforme`, `non applicable`) avec commentaire ; la décision est écrite dans le bundle via `rgaa audit review` ou son équivalent MCP | P1 | Décision horodatée et attribuée (R5) |
| F-VIS-4 | Remédiation pas à pas : patch proposé → aperçu → accord → application → nouveau rendu → re-vérification → capture après | P1 | Chaque étape est réversible et tracée |
| F-VIS-5 | Le guide **n'écrit jamais** de verdict de lui-même ; un test *Irréductible* est présenté avec sa justification écrite et une liste de vérifications | P0 (règle) | Revue de code : aucune écriture de statut sans action humaine |
| F-VIS-6 | « Belles pages de grilles » : pages HTML de grille par critère et par page, avec captures, statut et décision humaine | P1 | Accessibles elles-mêmes (le rapport d'accessibilité doit passer notre propre audit) |
| F-VIS-7 | Mode dégradé : sans client navigateur, la même revue est possible depuis Claude Code / Codex (captures en pièces jointes) | P1 | Parcours complet sans extension navigateur |

### 9.5 Version GitHub (`F-GH`)

| ID | Exigence | Prio | Critère d'acceptation |
|---|---|---|---|
| F-GH-1 | Binaire **`rgaa-lint`** (ou `rgaa lint`) autour de `rgaa-linter` (aujourd'hui sans binaire), sorties `github`, `sarif`, `json` | P0 | Annotations sur les lignes modifiées d'une PR |
| F-GH-2 | **Action `rgaa-action`** publiée : télécharge une release **épinglée** et vérifie sa somme de contrôle | P0 | Mauvaise somme → échec de l'étape |
| F-GH-3 | Mode **PR** : lint des fichiers modifiés, SARIF vers *Code scanning*, commentaire de delta | P0 | Régression → check en échec ; aucune régression → succès |
| F-GH-4 | Mode **URL** (preview déployée) : audit Obscura, modes `changed`, `smoke`, `full`, `diff`, `policy` (existants dans le workflow interne) | P1 | Même résultat que la CLI locale sur la même URL |
| F-GH-5 | IA désactivée par défaut ; `byok` via secret client ; `managed` via clé d'organisation | P1 | PR de fork (sans secrets) → mode déterministe, pas d'échec |
| F-GH-6 | **Remédiation en CI** : branche + PR de correctifs re-vérifiés, jamais de push sur la branche protégée (R6) | P1 | PR ouverte avec rapport de re-vérification joint |
| F-GH-7 | Permissions minimales documentées ; `pull_request_target` interdit avec du code de PR | P0 | Workflow d'exemple validé par revue sécurité |
| F-GH-8 | Releases **signées** (cosign ou minisign) + **SBOM CycloneDX** ; aujourd'hui somme de contrôle seule | P0 | L'Action vérifie la signature avant exécution |
| F-GH-9 | Publication sur le GitHub Marketplace + exemple de workflow dans le README | P1 | Installation copiée-collée fonctionnelle |

### 9.6 Preuve et rapports (`F-PRV`)

| ID | Exigence | Prio | Critère d'acceptation |
|---|---|---|---|
| F-PRV-1 | Signature ed25519 détachée d'un **JSON canonique** du bundle (canonicalisation spécifiée) | P0 | `rgaa audit verify` valide hors ligne ; une modification d'un octet invalide |
| F-PRV-2 | Clé par organisation, rotation, clé publique publiée | P1 | Vérification avec une clé révoquée → avertissement explicite |
| F-PRV-3 | Horodatage par un tiers (la signature seule ne prouve pas la date) | P2 | Preuve d'antériorité vérifiable |
| F-PRV-4 | Provenance de chaque verdict : moteur, version de règles, modèle, `reasoning_effort`, décision humaine | P0 | Visible dans chaque export |
| F-PRV-5 | Garde-fous d'export existants (échantillon court, contact manquant, non-conformité non prouvée, dérogation incomplète, pack légal non revu depuis 12 mois) conservés | P0 | Aucune régression |

## 10. Exigences non fonctionnelles

| ID | Exigence |
|---|---|
| NF-1 Souveraineté | Hébergement SaaS et données dans l'UE ; Mistral comme fournisseur UE ; documentation précise des flux sortants par mode (`off`, `byok`, `managed`). |
| NF-2 RGPD | Le SaaS est sous-traitant : DPA, registre, durée de conservation des bundles et captures (les captures peuvent contenir des données personnelles des sites audités). [ouvert : durée] |
| NF-3 Zéro télémétrie par défaut | Le binaire local n'envoie rien sans activation explicite [PDF §2.3]. |
| NF-4 Chaîne d'approvisionnement | Builds reproductibles visés, SBOM, signatures, `cargo audit` en CI (déjà présent pour les vulnérabilités). |
| NF-5 Sécurité navigateur | Obscura : réseau privé refusé et accès fichiers désactivés par défaut (décision de la carte Obscura v0.2.2) ; le MCP HTTP n'est jamais ouvert sans clé. |
| NF-6 Performance | Latence p95 publiée sur un corpus ouvert [PDF §4.3] ; 8 audits concurrents par défaut (`RGAA_API_MAX_CONCURRENT_AUDITS`). |
| NF-7 Fiabilité IA | Panne ou dépassement de budget → jamais d'échec silencieux ni de verdict inventé ; les tests passent en `review` avec cause. |
| NF-8 Accessibilité des livrables | Rapports et pages de grilles conformes au RGAA eux-mêmes. |
| NF-9 Langues | FR, EN, DE pour docs et rapports [PDF §6.1]. |
| NF-10 Transparence | Publier la couverture réelle par *Niveau d'automatisation* et le taux de faux positifs [PDF §4.3]. Corriger l'écart entre le README (« 73 déterministes / 32 LLM / 1 manuel ») et la spec des niveaux. |

## 11. Indicateurs

- **Adoption** : installations CLI/MCP par mois, dépôts utilisant l'Action, organisations actives.
- **Activation** : part des installations atteignant un premier correctif vérifié (O1).
- **Revue** : temps par *Test RGAA* revu, taille de la file, part des décisions modifiées après revue.
- **IA** : tokens et coût par audit, par *Famille de preuve*, par `reasoning_effort` ; marge brute IA par palier.
- **Qualité** : accord de *Calibration* par famille ; faux positifs signalés ; verdicts tronqués (cible 0).
- **Canal** : partenaires certifiés, licences induites [PDF §10.1].
- **Économie** : mix licences / services ≥ 40 % hors services en phase 3 [PDF §6.1].

## 12. Jalons

| Jalon | Contenu | Sortie |
|---|---|---|
| **M0 — Socle** (sem. 1-4) | F-ACC-0, F-LLM-1 à 6, F-BILL-1, F-PRV-4 | Large 4 opérationnel et mesuré ; faille `/audit` fermée ; coût réel d'un audit connu |
| **M1 — Pilote payant** (M1-M3) | F-ACC-1 à 3, F-BILL-2 à 4, F-PLG-1 à 4 et 6, F-GH-1 à 3, 7, 8, F-PRV-1, F-LLM-9 | Premier pilote facturé ; Action GitHub v1 ; rapports signés |
| **M2 — Team publique** (M3-M6) | F-VIS-1 à 7, F-PLG-5 et 7, F-GH-4 à 6 et 9, F-BILL-5 à 7, F-ACC-4 et 5, F-LLM-7, 8, 10 | Grille Team publiée ; 5-10 pilotes FR ; 3-5 cabinets |
| **M3 — Enterprise** (M6-M18) | F-PRV-2 et 3, SSO/RBAC, Jira, multi-sites, packs DE/Benelux | Phase 3 du PDF |

## 13. Risques

| Risque | Gravité | Parade |
|---|---|---|
| Promesse juridique (« conforme ») | Élevée | Vocabulaire contractuel « instrument de mise en conformité » ; R1-R3 ; attestation humaine par les partenaires |
| Marge IA négative (raisonnement `high` facturé en sortie, promotions qui expirent) | Élevée | F-BILL-1 avant prix ; effort par tier ; barème versionné ; budgets |
| Comportement de Large 4 qui change vite (`none` retiré en 3 semaines) | Moyenne | Alias daté ; test réel en CI derrière drapeau ; veille du changelog |
| Dépendance aux clients agents (Claude for Chrome, Codex for Chrome) | Moyenne | F-VIS-7 mode dégradé ; MCP standard ; CLI suffisante |
| Fuite de données par la passerelle | Moyenne | `byok` et `off` ; flux documentés ; hébergement UE |
| Fiabilité du modèle surestimée | Élevée | *Calibration* par famille ; jamais de `pass`/`fail` modèle |
| Concurrent qui ajoute un MCP | Moyenne | Local vérifiable, RGAA/12 pays, preuve signée [PDF §8] |
| Adoption open source insuffisante | Élevée | Démos publiques, corpus ouvert, docs FR/EN/DE [PDF §9] |

## 14. Questions ouvertes

1. Prix définitif de `mistral-large-4` après la promotion de lancement ; limites de débit ; sortie maximale.
2. Contexte réel : 1M (fiche) contre 512K / 524 288 (sources tierces).
3. Limites d'images de Large 4 (nombre, taille, formats).
4. Forme brute de `response_format: json_schema` chez Mistral et support de `strict`.
5. Les appels échoués sont-ils facturés par Mistral ?
6. Comportement du client `rig` face aux blocs `thinking` (non testé).
7. Conditions d'usage de Claude for Chrome, Codex for Chrome et du computer use dans un service commercial revendu ;
   disponibilité dans l'UE.
8. Licence des poids Large 4 (« Mistral Ecosystem Open Weight License ») si auto-hébergement envisagé.
9. Durée de conservation des captures et bundles (RGPD).
10. Unité d'abonnement Team : par dépôt ou par développeur.
11. Petit modèle Mistral pour le tier tactique (identifiant non vérifié).

## 15. Code déjà modifié (hors PRD)

Pendant l'analyse, deux tranches TDD ont été commencées puis arrêtées à la demande (non commitées) :
`rgaa-holo/src/transport.rs` (lecture des blocs `thinking`/`text`, F-LLM-2, 2 tests) et `rgaa-core/src/provider.rs`
(défaut `mistral-large-4`, F-LLM-1, 1 test). La suite complète n'a pas été relancée après la seconde.

## Sources

- `Strategie_Rust_accessibilite_Europe.pdf` (nov. 2026, document de travail)
- [Mistral Large 4 — page modèle](https://docs.mistral.ai/models/mistral-large-4)
- [Changelog Mistral](https://docs.mistral.ai/resources/changelogs)
- [Reasoning — Mistral](https://docs.mistral.ai/inference/conversations/reasoning)
- [Structured Outputs — Mistral](https://docs.mistral.ai/inference/conversations/structured-output/custom)
