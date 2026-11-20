# PRD — Holo-RGAA Europe : SaaS minimal, plugins agents, Mistral Large 4, GitHub

| | |
|---|---|
| **Statut** | v2 — décisions D1–D4 arrêtées le 2026-11-20 ; à valider pour exécution |
| **Portée** | Produit, offre, exigences fonctionnelles et non fonctionnelles, jalons. Pas de code. |
| **Entrées** | Brief du porteur (prime sur le PDF) · `Strategie_Rust_accessibilite_Europe.pdf` (nov. 2026, 16 p.) · code `rgaa-rs/` (lu le 2026-11-20) · documentation officielle Mistral (consultée le 2026-11-20) |
| **Revues** | v1 relue par une contre-revue adversariale (Sonnet 5.5), puis arbitrée (Opus 5.5). Les objections retenues et les angles morts sont intégrés ici. |
| **Documents liés** | `offre-europe-saas-plugins.md` (offre) · `p0-et-mistral-large-4.md` (analyse détaillée du code, défauts B.1–B.8) · `docs/specs/human-criteria-automation.md` (règles de revue) · `GLOSSARY.md` |
| **Vocabulaire** | Termes du glossaire en italique : *Test RGAA*, *Niveau d'automatisation* (*Automatique*, *Assisté*, *Irréductible*), *Famille de preuve*, *Calibration*. |

Convention de preuve : **[code]** constaté dans le repo · **[PDF]** affirmé par le document de stratégie, non revérifié ·
**[Mistral]** documentation officielle · **[hypothèse]** à valider · **[ouvert]** question sans réponse.

---

## 0. Décisions arrêtées

| ID | Décision | Conséquence dans ce PRD |
|---|---|---|
| **D1** | **Deux offres.** Une offre **Souveraine** (IA `off` ou `byok`, CLI + rapports HTML, aucun agent tiers requis) et une offre **Confort** (passerelle Mistral `managed` + plugins dans les agents Claude/Codex/Chrome). | §1, §6, §8. Chaque offre a ses propres arguments de vente ; « sans dépendance américaine » ne s'applique **qu'à** l'offre Souveraine. |
| **D2** | **L'agent guide, la décision est confirmée hors bande** sur une page de revue minimale, avec authentification personnelle. | §7 (R5, R7), §9.4 (F-VIS-3, F-VIS-8), §9.7 (F-SEC). Les agents n'ont pas d'outil d'écriture de statut. |
| **D3** | **Crédits prépayés avec plafond dur**, convertis depuis le journal d'usage. | §8.2, F-BILL. |
| **D4** | **Pilote M1 en `managed` sur instance dédiée par client** (pas de multi-tenant), facturation par export du journal. | §12. Multi-tenant en M2. |

**Écarts assumés au PDF** : le PDF vend une solution « jamais en SaaS obligatoire » et « sans dépendance à un éditeur
américain cloud » [PDF §4.1, §2.3]. Le brief du porteur impose un SaaS Mistral et des agents d'éditeurs américains.
D1 résout la tension : l'argument du PDF reste vrai pour l'offre Souveraine, il est **retiré** de l'offre Confort.

## 1. Résumé

Holo-RGAA devient une **plateforme d'audit et de remédiation d'accessibilité pilotée par des agents**. Le cœur
(moteur Rust, CLI, MCP, API) reste local et ouvert. Un **SaaS minimal** fournit seulement ce qui engage
l'organisation : comptes, clés, mesure et refacturation de la consommation IA (**Mistral Large 4**) en crédits prépayés,
historique, rapports signés, et la **page de revue** où l'humain confirme ses décisions. Dans l'offre Confort,
**l'expérience passe par des plugins** exécutés dans les clients agents (Claude Code Desktop, Codex, Claude for Chrome,
Codex for Chrome) branchés sur notre MCP / CLI / API ; la revue et la remédiation sont **guidées visuellement, pas à
pas**. L'offre Souveraine fournit le même moteur sans agent tiers. Une **GitHub Action** porte l'audit et la remédiation
dans la CI des clients.

Le produit vend un **instrument de mise en conformité** (détection, correction, preuve), **jamais une certification**.

## 2. Problème et opportunité

- **Obligation légale de masse** : l'European Accessibility Act (directive (UE) **2019/882** — le PDF cite par erreur
  « 2019/884 ») s'applique depuis le 28 juin 2025 au commerce électronique, à la banque, au transport, aux télécoms ;
  en France, sanctions jusqu'à 50 000 € par service, renouvelables tous les six mois [PDF §2.1].
- **Le web échoue massivement** : 94,8 % des pages d'accueil ont au moins un échec WCAG, 51 erreurs en moyenne
  (WebAIM 2025) [PDF §2.1].
- **Les agents de code aggravent la dette** : 94,59 % des modèles récents produisent du HTML à défauts critiques ou
  sérieux (Deque 2026) [PDF §2.2].
- **Souveraineté** : l'exécution locale vérifiable devient un critère d'achat (banque, santé, public) [PDF §2.3] —
  servi par l'offre Souveraine (D1).
- **Plafond de l'automatisation** : ~57 % des problèmes sont détectables automatiquement [PDF §1.2] ; dans notre
  propre mesure, sur 117 *Tests RGAA* réputés humains, 16 sont *Automatiques*, 82 *Assistés*, 19 *Irréductibles*
  [code : `docs/specs/human-criteria-automation.md` §1]. **La part humaine est structurelle : le produit doit
  l'outiller, pas la nier.**

**Positionnement** : aucun des Leaders Gartner 2026 (Level Access, Deque, Siteimprove) ne combine exécution locale
vérifiable et intégration agent-native profonde [PDF §3.2]. Notre valeur, commune aux deux offres : **RGAA natif,
documents officiels de 12 pays, piste d'audit signée, revue guidée**. L'argument « local vérifiable » appartient à
l'offre Souveraine seulement.

## 3. Objectifs et non-objectifs

### Objectifs (18 mois)

| ID | Objectif | Mesure de succès |
|---|---|---|
| O1 | Un développeur obtient un premier audit et un premier correctif vérifié dans son agent | < 10 min depuis l'installation, mesuré sur un corpus de démonstration fixé (critère du PDF §4.2) |
| O1b | Un auditeur ou responsable conformité **non développeur** lit un rapport et confirme une décision **sans terminal** | Parcours complet sur la page de revue, mesuré en pilote |
| O2 | Un auditeur traite la file de revue *Assistée* plus vite qu'un audit manuel | Temps par *Test RGAA* revu, mesuré sur pilotes ; cible fixée après mesure de référence [hypothèse] |
| O3 | La consommation IA est refacturée avec marge positive et sans surprise | 100 % des appels de modèle tracés ; aucun dépassement du solde prépayé ; écart facture fournisseur / mesure interne < 2 % [hypothèse de seuil] |
| O4 | La CI client bloque les régressions d'accessibilité | Action GitHub installable en 1 fichier ; PR de régression marquée en échec |
| O5 | Preuve opposable | Chaque rapport exporté est signé, vérifiable hors ligne, et chaque décision humaine est rattachée à une identité authentifiée |
| O6 | Phases du PDF | P1 adoption · P2 5-10 pilotes FR + 3-5 cabinets · P3 DE/Benelux [PDF §6] |

### Non-objectifs (explicites)

- **Pas d'overlay** ni de correctif injecté à l'exécution [PDF §1.2, §5.2]. Toute correction est faite au code source.
- **Pas de « conformité garantie »**, pas de certification [PDF §5.2, §9].
- **Pas d'éditeur d'audit dans le SaaS** : seulement la console de compte et la page de revue (§9.4, F-VIS-8).
- **Aucun `pass` ni `fail` décidé par un modèle sur un test *Assisté*** (voir §7).
- **Aucun outil d'agent n'écrit un statut** de *Test RGAA* (D2).
- **Pas de vente d'audit humain** par l'éditeur : c'est le rôle des cabinets partenaires [PDF §7.3].
- Hors périmètre v1 : application mobile, PDF/UA, sandbox de plugins WASM (absente du code), SSO (Enterprise, v2).

## 4. Personas

| Persona | Contexte | Besoin principal | Surface |
|---|---|---|---|
| **Dev produit** | Équipe 5-50 devs, utilise Claude Code ou Codex | Ne pas introduire de défauts ; corriger vite | Plugin agent, CLI, Action GitHub |
| **Auditeur·rice** (cabinet ou interne) | Réalise l'audit RGAA, signe le rapport | Preuves rassemblées, file ordonnée, décision tracée | Agent navigateur (guide) + **page de revue** (décision) |
| **Responsable conformité** | Banque, assurance, e-commerce, public | Déclaration, taux, historique, preuve | Pages de grilles HTML, page de revue, rapports exportés |
| **Lead DevOps** | Possède la CI | Gate fiable, binaire vérifié, pas de fuite de code | Action GitHub |
| **Admin de compte** | Côté client | Clés, crédits, factures | Console SaaS minimale (F-ACC, F-BILL) |
| **Partenaire cabinet** | Access42, Temesis, Ideance, Funka (profils illustratifs [PDF §7]) | Industrialiser l'audit, revendre | Licence partenaire, tarif revendeur IA |

## 5. Existant (constaté dans le code)

| Brique | État | Référence |
|---|---|---|
| Catalogue 106 critères, mapping axe | Livré | `rgaa-core`, `rgaa-rules`, `rgaa-data` |
| Pipeline d'audit (axe + gap-fix + LLM + clavier) | Livré | `rgaa-orchestrator`, `rgaa-obscura` (Obscura 0.2.4 épinglé, SHA-256) |
| CLI `rgaa audit analyze / igt / verify / report / review / policy` ; formats json, markdown, sarif, junit, html. **`audit verify` vérifie des propositions de remédiation** | Livré | `rgaa-cli/src/commands/` |
| MCP : `analyze`, `audit_url`, `get_audit_result`, `remediate`, `verify_fix`, `source_map`, `lint_static`, `list_criteria`, `igt` (déprécié). **Aucun outil d'écriture de décision.** `analyze` renvoie `status` et `justification` à l'agent | Livré, stdio + HTTP | `rgaa-mcp/src/server.rs` |
| MCP HTTP : jeton partagé `RGAA_MCP_TOKEN` **optionnel** ; sans lui, un client sans en-tête `Origin` n'est pas authentifié | Livré | `rgaa-mcp-http/src/lib.rs` |
| Plugins : `rgaa-consultant` (Claude Code : 7 skills, 3 commandes, 4 agents, hooks), `rgaa-codex` (7 skills), `acces-accessibility` (4 skills) | Livré | `rgaa-rs/plugins/` |
| Remédiation par framework + re-vérification | Livré | `rgaa-remediation`, outil `verify_fix` |
| Revue humaine enregistrée dans l'audit — **auteur en texte libre (`--author`), identité non vérifiée** | Livré (CLI) | `rgaa audit review` (`review.rs`) |
| Documents officiels 12 pays (déclaration, PDF, JSON pivot) | Livré | `rgaa-report` |
| API REST (bundles, findings, policy, batch, webhooks signés) | Livré, **mono-tenant** | `rgaa-api` |
| Multi-fournisseur LLM, preset `mistral` | Livré, **inutilisable avec Large 4 en l'état** | `rgaa-core/src/provider.rs` — défauts B.1–B.8 |
| Mesure de tokens | Partielle : événement `tracing` côté `rig` seulement | `rgaa-agent/src/metrics.rs` |
| Organisations, quotas, facturation, signature de rapports, Action GitHub, binaire lint, page de revue | **Absents** | voir §9 |

**Correctif de sécurité préalable** [code] : `POST /audit` et `GET /audit/:id` sont exposés **sans authentification**
(`rgaa-api/src/lib.rs`, `legacy_routes`), avec CORS ouvert. Bloquant pour toute mise en ligne.

## 6. Architecture produit cible

```
 Offre Confort : clients agents (UX)          Plan local (client)                    SaaS minimal (UE)
 ┌──────────────────────────┐   MCP/CLI   ┌──────────────────────────┐   HTTPS   ┌──────────────────────────┐
 │ Claude Code Desktop      │────────────▶│ rgaa-mcp / rgaa-cli      │──────────▶│ Comptes, clés, crédits   │
 │ Codex                    │  (lecture + │ rgaa-orchestrator        │  (option) │ Passerelle LLM mesurée   │──▶ Mistral
 │ Claude for Chrome        │   guidage)  │ Obscura (navigateur)     │           │ Historique, signature    │    Large 4
 │ Codex for Chrome         │             │ rgaa-remediation/report  │           │ PAGE DE REVUE (décision, │
 │ + computer use           │             └──────────────────────────┘           │ auth personnelle)        │
 └──────────────────────────┘                        ▲                           └──────────────────────────┘
 Offre Souveraine : CLI + rapports HTML ─────────────┤ (sans agent tiers, IA off ou byok)
                                   GitHub Action ────┘ (même binaire, même MCP)
```

Principes :
- **P-1 Flux de données déclarés.** Le scan, le DOM, les captures et le code restent chez le client **dans l'offre
  Souveraine**. Dans l'offre Confort, **dès qu'un client agent (Claude for Chrome, Codex for Chrome, computer use,
  Claude Code, Codex) lit une page, son DOM, ses captures et le contexte partent vers l'éditeur de cet agent.** Chaque
  client agent a sa fiche de flux de données publiée (F-SEC-5).
- **P-2 Une seule vérité.** Plugins, Action, page de revue et SaaS consomment le même bundle d'audit ; aucune surface
  ne recalcule un verdict.
- **P-3 IA mesurée.** Tout appel de modèle passe par une route qui enregistre l'usage (passerelle SaaS, ou mesure locale
  reportée si le client utilise sa propre clé).
- **P-4 Trois modes IA** : `off` (déterministe seul) · `byok` (clé Mistral du client, 0 % de marge) · `managed`
  (passerelle SaaS, crédits prépayés). Offre Souveraine = `off` ou `byok` ; offre Confort = `managed`.
- **P-5 Les règles sont appliquées par le serveur**, jamais par un prompt ou un écran (R7).

**Topologie à vérifier** [ouvert] : si Claude for Chrome ou Codex for Chrome joignent les serveurs MCP distants depuis
l'infrastructure de leur éditeur (et non depuis le poste), le MCP doit être exposé sur Internet ; le « plan local » de
ce schéma et l'allowlist d'origines (F-ACC-4) ne suffisent alors plus. À trancher avant M2 (question 7, §14).

## 7. Règles de décision non négociables

Reprises de `docs/specs/human-criteria-automation.md` §2, complétées par D2 ; elles s'imposent à **toutes** les
surfaces, y compris le computer use et Claude for Chrome :

- **R1** Un *Test RGAA* *Assisté* rend `review`. Jamais `pass` ni `fail`, quelle que soit la confiance.
- **R2** Aucun `fail` ne provient d'un verdict de modèle. Mesure à l'origine : 8 % de faux « non pertinent » sur la
  seule *Famille de preuve* calibrée (alternative d'image), avec un autre modèle ; **non mesuré sur Large 4**.
- **R3** L'auditeur voit la preuve et les conditions structurées, **pas** le verdict du modèle. Mesure à l'origine :
  19 % de mauvaises alternatives validées à tort, famille alternative d'image, modèle `swift-1.5-27b` ; **non mesuré
  sur Large 4**. Le verdict sert uniquement à **ordonner la file**. Comme l'agent lit la réponse MCP, **la réponse MCP
  elle-même ne contient pas le verdict modèle d'un test *Assisté*** (aujourd'hui `analyze` renvoie `status` et
  `justification`).
- **R4** Le computer use **provoque des états et enrichit la preuve** ; il ne fait monter aucun test d'un niveau.
- **R5** Toute décision humaine est horodatée, **rattachée à une identité authentifiée** (pas un texte libre), et
  versionnée dans le bundle.
- **R6** Un patch n'est jamais appliqué sans validation humaine ; en CI, il prend la forme d'une PR, jamais d'un push.
- **R7** (D2) **Aucun outil exposé à un agent n'écrit un statut.** La décision est confirmée hors bande, sur la page de
  revue, par un humain authentifié. Le serveur refuse toute écriture de statut sans cette identité.

## 8. Offre et tarification

### 8.1 Offres et paliers (D1)

| | **Offre Souveraine** | **Offre Confort** |
|---|---|---|
| Promesse | Rien ne sort sans votre accord ; aucun agent tiers requis | Audit et remédiation guidés dans vos agents, IA européenne incluse |
| IA | `off` ou `byok` (clé Mistral du client, ou modèle local) | `managed` : passerelle Mistral, crédits prépayés |
| UX | CLI, rapports et pages de grilles HTML, page de revue auto-hébergeable | Plugins Claude Code / Codex / Claude for Chrome / Codex for Chrome + page de revue SaaS |
| Argument « sans dépendance US » | **Oui** | **Non** (flux déclarés, P-1) |
| Cible prioritaire | Public, banque, santé, défense | E-commerce, ESN, équipes produit, cabinets |

| Palier | Contenu | Prix [hypothèse, à valider par étude de prix — PDF §11.1] |
|---|---|---|
| **Community** (Apache-2.0) | Moteur, CLI, MCP, linter, plugins, Action GitHub mode déterministe, `byok` | Gratuit |
| **Team** | + historique hébergé UE, rapports HTML/PDF, page de revue partagée, support ; Confort : passerelle `managed` | Abonnement par dépôt ou par dev / mois + crédits IA prépayés |
| **Enterprise** | + multi-sites, rapports signés longue durée, déclarations 12 pays à jour, instance dédiée ou auto-hébergée, SSO/RBAC (v2), SLA, Jira (v2) | Contrat annuel par portefeuille + crédits IA |
| **Partenaire** | Licence Team offerte, tarif revendeur IA, certification, listing | Revenue share sur licences induites [PDF §7.3] |

Publier la grille Team et une grille indicative Enterprise (différenciateur face à Deque et Level Access, prix non
publics [PDF §5.2]). **Marge IA cible et repères de prix** : à fixer après F-BILL-1 et des entretiens avec 5 acheteurs
(dont des cabinets partenaires) avant M2.

### 8.2 Refacturation de la consommation IA (D3)

- **Modèle** : le client **achète des crédits à l'avance**. Chaque appel de modèle débite le solde selon le journal
  d'usage (`llm_usage`) et le barème versionné (`price_book`). **Plafond dur** : solde épuisé → plus aucun appel.
- **Barème initial** `mistral-large-4` [Mistral, page modèle datée du 2026-11-05] : 0,70 $ / 1M tokens en entrée,
  0,07 $ en entrée en cache, 2,10 $ en sortie. Le lancement a eu « -50 % pendant 2 semaines » ; la page ne précise
  pas si ce prix est définitif → **revue mensuelle obligatoire du barème** [ouvert].
- **Le raisonnement est facturé en sortie** : Large 4 raisonne toujours, `reasoning_effort` vaut `high` par défaut
  [Mistral, changelog 2026-10-29]. Le niveau d'effort est donc un **levier de prix** à exposer et à calibrer.
- **Conversion** : crédits débités = Σ (tokens × prix fournisseur) × (1 + marge du palier). Marge `byok` = 0 (pas de
  crédits : le client paie Mistral directement).
- **Garde-fous** : estimation affichée **avant** un audit complet ; solde vérifié **avant** chaque appel ; solde
  épuisé → le test reste `review` avec la cause `budget` (jamais de verdict inventé) ; alertes à 50/80/100 % du solde ;
  rechargement manuel ou automatique au choix du client.
- **Illustration arithmétique** [hypothèse de tailles] : 6 000 tokens en entrée + 1 500 en sortie ≈ 0,0074 $ par
  appel. **Aucune mesure réelle de tokens par audit n'existe dans le repo** : la valeur d'un crédit ne peut pas être
  fixée avant la mesure (F-BILL-1).

## 9. Exigences fonctionnelles

Priorités : **P0** requis pour le premier pilote payant (M1) · **P1** requis pour la grille Team publique (M2) ·
**P2** Enterprise.

### 9.1 IA — Mistral Large 4 (`F-LLM`)

| ID | Exigence | Prio | Critère d'acceptation |
|---|---|---|---|
| F-LLM-1 | Le preset `mistral` a pour modèle par défaut **`mistral-large-4`** (identifiant versionné, non `-latest`). **Statut : non admis** au sens du seuil d'admission de la spec (§2.6) tant que F-LLM-9 n'est pas faite | P0 | Config avec seule `MISTRAL_API_KEY` → modèle `mistral-large-4` ; aucun pilote facturé avant la *Calibration* |
| F-LLM-2 | Les réponses dont `content` est une **liste de blocs** (`thinking`, `text`) sont lues ; seul le texte compte | P0 | Un bloc `thinking` contenant un faux verdict n'est jamais retenu ; le défaut actuel (B.3) est couvert par un test |
| F-LLM-3 | Paramètre **`reasoning_effort`** (`low`, `medium`, `high`) configurable par tier ; `none` refusé à la configuration | P0 | Valeur visible dans la provenance du verdict ; `none` → erreur explicite avant tout appel |
| F-LLM-4 | Les clés « thinking » (`chat_template_kwargs`, `think`, `enable_thinking`) ne sont jamais envoyées à Mistral | P0 | Configuration explicite refusée avec message clair pour `mistral` |
| F-LLM-5 | **Usage lu sur les deux chemins** (transport brut `rgaa-holo` et agent `rig`) : tokens entrée, cache, sortie | P0 | Chaque appel produit un enregistrement d'usage (F-BILL-2) |
| F-LLM-6 | `max_tokens` dimensionné pour inclure le raisonnement | P0 | Taux de verdicts tronqués = 0 sur le corpus de calibration |
| F-LLM-7 | Sortie structurée : `json_object` + validation locale par le schéma de verdict ; `json_schema` activé seulement après test réel | P1 | Décision documentée après le test réel (forme brute non trouvée dans la doc) |
| F-LLM-8 | Image : PNG en data URL, taille et dimensions bornées avant envoi | P1 | Limites Mistral documentées ou mesurées [ouvert] |
| F-LLM-9 | **Calibration** de Large 4 par *Famille de preuve* avec le protocole existant, **en M0**, avant tout pilote facturé | P0 | Rapport de calibration ; admission ou refus selon la spec §2.6 ; les tests restent `review` quoi qu'il arrive (R1) |
| F-LLM-10 | **Mode dégradé** : panne Mistral → audit déterministe complet + tests IA en `review` cause `llm_unavailable` | P0 | Test de panne simulée ; aucun échec silencieux (NF-7) |
| F-LLM-11 | Fournisseur secondaire ou modèle local (`ollama`, `vllm`) via la route de repli existante | P1 | Bascule documentée et testée |

### 9.2 SaaS minimal — comptes, clés, usage, facturation (`F-ACC`, `F-BILL`)

| ID | Exigence | Prio | Critère d'acceptation |
|---|---|---|---|
| F-ACC-0 | Les routes `/audit` et `/audit/:id` exigent une clé ; CORS restreint | P0 | Requête sans clé → 401 |
| F-ACC-1 | **M1 : une instance dédiée par client** (D4), isolée (base, clés, stockage) ; aucune donnée partagée entre instances | P0 | Déploiement reproductible d'une instance par client |
| F-ACC-2 | Entité **organisation** multi-tenant ; toute clé, tout audit, tout finding, tout lot appartient à une organisation | P1 | Une organisation reçoit 404 sur l'audit d'une autre |
| F-ACC-3 | Le contexte d'authentification (organisation, clé, scopes) est transmis aux traitements (aujourd'hui jeté) | P1 | Chaque écriture porte l'organisation de l'appelant |
| F-ACC-4 | Le MCP HTTP **exige** une clé (aujourd'hui optionnelle) ; en M2, clé par organisation à la place du jeton unique `RGAA_MCP_TOKEN` | P0 (clé obligatoire) · P1 (par organisation) | Sans clé → 401, avec ou sans en-tête `Origin` |
| F-ACC-5 | Origines des extensions (`chrome-extension://…`) ajoutables à l'allowlist | P1 | Dépend de la topologie (§6, question 7) |
| F-ACC-6 | Rotation, expiration, révocation des clés ; scopes `audit:read`, `audit:write`, `llm:use`, `admin` ; **aucun scope d'écriture de statut pour une clé d'agent** (R7) | P1 | Clé révoquée → 401 immédiat |
| F-ACC-7 | **Comptes personnels** pour les relecteurs (identité authentifiée, distincte des clés d'API) | P0 | Toute décision du bundle référence un compte personnel (R5) |
| F-BILL-1 | **Mesure de référence** : coût réel en tokens de 3 audits réels (15 pages) par `reasoning_effort` | P0 (M0) | Chiffre publié en interne avant de fixer la valeur d'un crédit |
| F-BILL-2 | Journal `llm_usage` : organisation, audit, *Test RGAA*, fournisseur, modèle, effort, tokens (entrée, cache, sortie), latence, statut | P0 | Somme du journal = facture fournisseur à ±2 % sur un mois [hypothèse de seuil] |
| F-BILL-3 | Barème `price_book` versionné (date d'effet), relu chaque mois | P0 | Changement de prix sans redéploiement |
| F-BILL-4 | **Solde de crédits prépayés**, débité par appel, vérifié **avant** l'appel, plafond dur (D3) | P0 | Solde nul → `review` cause `budget`, aucun appel émis |
| F-BILL-5 | **Export du journal** par client et par période, base de facturation du pilote (D4) | P0 | Export rapproché manuellement de la facture Mistral |
| F-BILL-6 | Estimation de coût affichée avant un audit complet | P1 | Écart estimation / réel mesuré et affiché |
| F-BILL-7 | Achat et rechargement de crédits en ligne (Stripe prévu dans la spec de distribution) | P1 | Recharge sans intervention humaine |
| F-BILL-8 | Mode `byok` : la clé du client ne transite pas par nos serveurs ; l'usage est mesuré localement | P1 | Vérifiable : aucun appel sortant vers notre passerelle en `byok` |

### 9.3 Plugins agents (`F-PLG`) — offre Confort

Un **plugin = un ensemble de skills + commandes + accès MCP**, distribué pour Claude Code et Codex. Les plugins actuels
(`rgaa-consultant`, `rgaa-codex`, `acces-accessibility`) sont réorganisés en cinq plugins de domaine.

| ID | Plugin | Exigence | Prio | Critère d'acceptation |
|---|---|---|---|---|
| F-PLG-1 | **audit** | Lancer, reprendre, lister un audit (URL, site, projet) ; afficher taux et état | P0 | Parcours O1 < 10 min |
| F-PLG-2 | **remediation** | Relocaliser le finding dans le source (`source_map`), proposer un patch par framework, appliquer après accord, re-vérifier (`verify_fix`) | P0 | Patch non appliqué sans accord (R6) ; re-vérification jointe au patch |
| F-PLG-3 | **ia-assist** | File de revue des tests *Assistés* ordonnée par le signal du modèle ; présente preuve + conditions ; renvoie vers la page de revue pour décider | P1 | **La réponse MCP ne contient pas le verdict modèle d'un test *Assisté*** (R3) ; test serveur |
| F-PLG-4 | **reporting** | Grilles RGAA (106 critères), pages HTML avec captures, déclaration d'accessibilité par pays, export PDF, SARIF/JUnit | P0 | Rendu du bundle existant ; aucune donnée recalculée (P-2) |
| F-PLG-5 | **visual-guide** | Remédiation et revue pas à pas dans le navigateur (voir §9.4) | P1 | Parcours guidé complet sur un critère de chaque groupe G1–G5 |
| F-PLG-6 | Commun | Les plugins n'ont **aucune logique métier** propre : tout passe par MCP/CLI/API | P0 | Un même audit donne le même résultat via plugin, CLI et Action |
| F-PLG-7 | Commun | Instructions d'installation et fichiers `CLAUDE.md` / `AGENTS.md` générés pour le dépôt client | P1 | Installation en une commande pour chaque client agent |

### 9.4 Revue et remédiation visuelles guidées (`F-VIS`)

Clients : **Claude for Chrome**, **Codex for Chrome**, **computer use** (offre Confort) — en **lecture et guidage**
seulement (R7). La décision se prend sur la **page de revue** (F-VIS-8).

| ID | Exigence | Prio | Critère d'acceptation |
|---|---|---|---|
| F-VIS-1 | Pour chaque test en `review`, l'agent ouvre la page, **met en évidence l'élément** concerné et présente la preuve (capture, DOM, arbre d'accessibilité, contexte). La capture versée au bundle est **prise par notre outil** (Obscura), puis hachée ; une capture de l'agent n'est qu'une illustration | P1 | Chaque capture du bundle a un hash calculé par notre outil |
| F-VIS-2 | L'agent **provoque les états** utiles (focus clavier, survol, ouverture de menu, zoom 200 %, contraste) pour 7.4, 7.5, 11.10–11.12, 12.11 et similaires (R4) ; notre outil rejoue l'état et capture avant/après | P1 | Captures avant/après hachées jointes à la preuve |
| F-VIS-3 | L'agent **prépare** la décision (preuve, conditions, commentaire proposé) et renvoie l'humain vers la page de revue ; **il n'écrit aucun statut** (R7) | P1 | Aucun outil MCP d'écriture de statut |
| F-VIS-4 | Remédiation pas à pas : patch proposé → aperçu → accord → application → nouveau rendu → re-vérification → capture après | P1 | Chaque étape est réversible et tracée |
| F-VIS-5 | Un test *Irréductible* est présenté avec sa justification écrite et une liste de vérifications | P0 (règle) | **Test serveur** : toute écriture de statut sans identité humaine authentifiée est refusée |
| F-VIS-6 | « Belles pages de grilles » : pages HTML de grille par critère et par page, avec captures, statut et décision humaine | P0 | Accessibles elles-mêmes (le rapport doit passer notre propre audit) |
| F-VIS-7 | Mode dégradé : sans client navigateur, la même revue est possible depuis Claude Code / Codex (captures en pièces jointes) puis la page de revue | P1 | Parcours complet sans extension navigateur |
| F-VIS-8 | **Page de revue** (D2) : liste des tests en `review` avec preuve et conditions (sans verdict modèle), décision `conforme` / `non conforme` / `non applicable` + commentaire, authentification personnelle (F-ACC-7) ; servie par le SaaS (Confort) ou auto-hébergée (Souveraine) | P0 | Décision horodatée, attribuée à un compte personnel, écrite dans le bundle ; parcours O1b |

### 9.5 Version GitHub (`F-GH`)

| ID | Exigence | Prio | Critère d'acceptation |
|---|---|---|---|
| F-GH-1 | Binaire **`rgaa-lint`** (ou `rgaa lint`) autour de `rgaa-linter` (aujourd'hui sans binaire), sorties `github`, `sarif`, `json` | P0 | Annotations sur les lignes modifiées d'une PR |
| F-GH-2 | **Action `rgaa-action`** publiée : télécharge une release **épinglée** et vérifie sa somme de contrôle | P0 | Mauvaise somme → échec de l'étape |
| F-GH-3 | Mode **PR** : lint des fichiers modifiés, SARIF vers *Code scanning*, commentaire de delta | P0 | Régression → check en échec ; aucune régression → succès |
| F-GH-4 | Mode **URL** (preview déployée) : audit Obscura, modes `changed`, `smoke`, `full`, `diff`, `policy` (existants dans le workflow interne) | P1 | Même résultat que la CLI locale sur la même URL |
| F-GH-5 | IA désactivée par défaut ; `byok` via secret client ; `managed` via clé de l'instance | P1 | PR de fork (sans secrets) → mode déterministe, pas d'échec |
| F-GH-6 | **Remédiation en CI** : branche + PR de correctifs re-vérifiés, jamais de push sur la branche protégée (R6) | P1 | PR ouverte avec rapport de re-vérification joint |
| F-GH-7 | Permissions minimales documentées ; `pull_request_target` interdit avec du code de PR | P0 | Workflow d'exemple validé par revue sécurité |
| F-GH-8 | Releases **signées** (cosign ou minisign) + **SBOM CycloneDX** ; aujourd'hui somme SHA-256 seule | P1 | L'Action vérifie la signature avant exécution |
| F-GH-9 | Publication sur le GitHub Marketplace + exemple de workflow dans le README | P1 | Installation copiée-collée fonctionnelle |

### 9.6 Preuve et rapports (`F-PRV`)

| ID | Exigence | Prio | Critère d'acceptation |
|---|---|---|---|
| F-PRV-1 | Signature ed25519 détachée d'un **JSON canonique** du bundle (canonicalisation spécifiée) ; vérification par **`rgaa report verify`** (nom distinct de `rgaa audit verify`, qui vérifie des remédiations) | P1 | Vérification hors ligne ; une modification d'un octet invalide |
| F-PRV-2 | Clé par organisation, rotation, clé publique publiée | P1 | Vérification avec une clé révoquée → avertissement explicite |
| F-PRV-3 | Horodatage par un tiers (la signature seule ne prouve pas la date) | P2 | Preuve d'antériorité vérifiable |
| F-PRV-4 | Provenance de chaque verdict : moteur, version de règles, modèle, `reasoning_effort`, décision humaine et compte personnel | P0 | Visible dans chaque export |
| F-PRV-5 | Garde-fous d'export existants (échantillon court, contact manquant, non-conformité non prouvée, dérogation incomplète, pack légal non revu depuis 12 mois) conservés | P0 | Aucune régression |

### 9.7 Sécurité des agents (`F-SEC`)

Les pages auditées sont **non fiables** : elles peuvent contenir des instructions cachées destinées aux agents qui les
lisent (injection de prompt).

| ID | Exigence | Prio | Critère d'acceptation |
|---|---|---|---|
| F-SEC-1 | Les outils MCP exposés aux agents navigateur sont **en lecture seule** ; aucune écriture de statut (R7) ; les écritures de code passent par l'accord explicite de l'humain (R6) | P0 | Inventaire des outils par scope, revu à chaque ajout |
| F-SEC-2 | Le contenu des pages auditées est transmis aux agents **comme donnée balisée**, jamais comme instruction | P1 | Revue des descriptions et réponses d'outils |
| F-SEC-3 | **Page piégée** dans le corpus de CI : instructions cachées demandant d'écrire une décision, d'exfiltrer une clé, d'appliquer un patch | P0 | Aucun effet constaté ; test bloquant en CI |
| F-SEC-4 | Le serveur refuse toute écriture de statut sans identité humaine (R5, R7) | P0 | Test serveur |
| F-SEC-5 | **Fiche de flux de données par client agent** (ce qui part vers quel éditeur) ; jointe à l'offre Confort | P1 | Publiée avant M2 |

## 10. Exigences non fonctionnelles

| ID | Exigence |
|---|---|
| NF-1 Souveraineté | Offre Souveraine : aucune donnée hors de l'infrastructure du client sans activation explicite. Offre Confort : SaaS et données dans l'UE, Mistral comme fournisseur UE, flux vers les éditeurs d'agents déclarés (P-1, F-SEC-5). |
| NF-2 RGPD | Le SaaS est sous-traitant : **DPA signé et durée de conservation par défaut fixée avant tout pilote payant** (proposition : 90 jours pour les captures [hypothèse]) ; registre ; les captures peuvent contenir des données personnelles des sites audités. |
| NF-3 Zéro télémétrie par défaut | Le binaire local n'envoie rien sans activation explicite [PDF §2.3]. |
| NF-4 Chaîne d'approvisionnement | Builds reproductibles visés, SBOM, signatures, `cargo audit` en CI (déjà présent pour les vulnérabilités). |
| NF-5 Sécurité | Cible : Obscura avec réseau privé refusé et accès fichiers désactivés par défaut (décision de la carte Obscura v0.2.2) ; MCP HTTP jamais ouvert sans clé (aujourd'hui le jeton est optionnel : F-ACC-4). |
| NF-6 Performance | Latence p95 publiée sur un corpus ouvert [PDF §4.3] ; 8 audits concurrents par défaut (`RGAA_API_MAX_CONCURRENT_AUDITS`). |
| NF-7 Fiabilité IA | Panne ou solde épuisé → jamais d'échec silencieux ni de verdict inventé ; les tests passent en `review` avec cause. |
| NF-8 Accessibilité des livrables | Rapports, pages de grilles et page de revue conformes au RGAA eux-mêmes. |
| NF-9 Langues | FR, EN, DE pour docs et rapports [PDF §6.1]. |
| NF-10 Transparence | Publier la couverture réelle par *Niveau d'automatisation* et le taux de faux positifs [PDF §4.3]. Corriger l'écart entre le README (« 73 déterministes / 32 LLM / 1 manuel ») et la spec des niveaux. |
| NF-11 Responsabilité | Clauses contractuelles : instrument de mise en conformité, non-certification, limitation de responsabilité en cas d'erreur d'audit ; attestation humaine par les partenaires. |

## 11. Indicateurs

- **Adoption** : installations CLI/MCP par mois, dépôts utilisant l'Action, organisations actives.
- **Activation** : part des installations atteignant un premier correctif vérifié (O1) ; part des relecteurs ayant
  confirmé une décision sur la page de revue (O1b).
- **Revue** : temps par *Test RGAA* revu, taille de la file, part des décisions modifiées après revue.
- **IA** : tokens et coût par audit, par *Famille de preuve*, par `reasoning_effort` ; marge brute IA par palier ;
  consommation et rechargement des crédits.
- **Qualité** : accord de *Calibration* par famille ; faux positifs signalés ; verdicts tronqués (cible 0).
- **Sécurité** : résultats du test de page piégée (cible : aucun effet).
- **Canal** : partenaires certifiés, licences induites [PDF §10.1].
- **Économie** : mix licences / services ≥ 40 % hors services en phase 3 [PDF §6.1] ; répartition Souveraine /
  Confort.

## 12. Jalons

| Jalon | Contenu | Sortie |
|---|---|---|
| **M0 — Socle** (sem. 1-4) | F-ACC-0, F-ACC-4 (clé obligatoire), F-LLM-1 à 6, **F-LLM-9**, F-LLM-10, F-BILL-1, F-PRV-4 ; DPA et durée de conservation (NF-2) ; clauses (NF-11) | Large 4 calibré (admis ou refusé) et mesuré ; failles `/audit` et MCP fermées ; coût réel de 3 audits connu |
| **M1 — Pilote payant** (M1-M3), **D4** | **Instance dédiée par client** (F-ACC-1), `managed`, **crédits prépayés** (F-BILL-2 à 5), comptes personnels (F-ACC-7), **page de revue** (F-VIS-8), pages de grilles (F-VIS-6), plugins audit, remédiation, reporting (F-PLG-1, 2, 4, 6), F-SEC-1, 3, 4, Action lint (F-GH-1 à 3, 7) | Premier pilote facturé sur crédits ; décisions humaines opposables |
| **M2 — Team publique** (M3-M6) | Multi-tenant (F-ACC-2, 3, 5, 6), F-PLG-3, 5, 7, F-VIS-1 à 4 et 7, F-SEC-2, 5, F-GH-4 à 6, 8, 9, F-PRV-1 et 2, F-BILL-6 à 8, F-LLM-7, 8, 11 ; **réponse juridique et topologie des agents navigateur avant le démarrage** | Grille Team publiée ; 5-10 pilotes FR ; 3-5 cabinets |
| **M3 — Enterprise** (M6-M18) | F-PRV-3, SSO/RBAC, Jira, multi-sites, packs DE/Benelux | Phase 3 du PDF |

## 13. Risques

| Risque | Gravité | Parade |
|---|---|---|
| Promesse juridique (« conforme ») | Élevée | Vocabulaire contractuel « instrument de mise en conformité » ; R1-R3 ; NF-11 ; attestation humaine par les partenaires |
| **Injection de prompt** depuis une page auditée vers un agent outillé | Élevée | R7, F-SEC-1 à 4 ; page piégée bloquante en CI |
| **Identité de relecteur non prouvée** (rapport signé attestant un texte libre) | Élevée | Comptes personnels (F-ACC-7), refus serveur (F-SEC-4) |
| Marge IA négative (raisonnement `high` facturé en sortie, promotions qui expirent) | Élevée | F-BILL-1 avant prix ; effort par tier ; barème versionné ; crédits prépayés |
| Fiabilité du modèle surestimée | Élevée | *Calibration* en M0 ; admission selon la spec ; jamais de `pass`/`fail` modèle |
| Conditions d'usage ou disponibilité UE des agents navigateur défavorables | Moyenne | Question juridique avant M2 ; F-VIS-7 et page de revue rendent le produit utilisable sans eux ; offre Souveraine |
| Comportement de Large 4 qui change vite (`none` retiré en 3 semaines) | Moyenne | Identifiant versionné ; test réel en CI derrière drapeau ; veille du changelog ; F-LLM-10 |
| Fuite de données vers les éditeurs d'agents | Moyenne | P-1 et F-SEC-5 (flux déclarés) ; offre Souveraine pour les acheteurs sensibles |
| Concurrent qui ajoute un MCP (Deque l'a fait) | Moyenne | Valeur commune : RGAA natif, 12 pays, piste signée, revue guidée ; « local vérifiable » réservé à l'offre Souveraine |
| Adoption open source insuffisante | Élevée | Démos publiques, corpus ouvert, docs FR/EN/DE [PDF §9] |

## 14. Questions ouvertes

1. Prix définitif de `mistral-large-4` après la promotion de lancement ; limites de débit ; sortie maximale.
2. Contexte réel : 1M (fiche) contre 512K / 524 288 (sources tierces).
3. Limites d'images de Large 4 (nombre, taille, formats).
4. Forme brute de `response_format: json_schema` chez Mistral et support de `strict`.
5. Les appels échoués sont-ils facturés par Mistral ?
6. Comportement du client `rig` face aux blocs `thinking` (non testé).
7. **Avant M2** : conditions d'usage de Claude for Chrome, Codex for Chrome et du computer use dans notre service
   (le client apporte son propre abonnement d'agent) ; disponibilité dans l'UE ; **topologie** (le MCP est-il joint
   depuis le poste ou depuis l'infrastructure de l'éditeur ?).
8. Licence des poids Large 4 (« Mistral Ecosystem Open Weight License ») si auto-hébergement envisagé (offre
   Souveraine).
9. Durée de conservation définitive des captures et bundles (90 jours proposés).
10. Unité d'abonnement Team : par dépôt ou par développeur ; marge IA cible ; valeur d'un crédit.
11. Petit modèle Mistral pour le tier tactique (identifiant non vérifié).

## 15. Code déjà modifié (hors PRD)

Pendant l'analyse, deux tranches TDD ont été commencées puis arrêtées à la demande (non commitées) :
`rgaa-holo/src/transport.rs` (lecture des blocs `thinking`/`text`, F-LLM-2, 2 tests) et `rgaa-core/src/provider.rs`
(défaut `mistral-large-4`, F-LLM-1, 1 test). La suite complète n'a pas été relancée après la seconde.

## 16. Journal des révisions

- **v1** (2026-11-20) : première version.
- **v2** (2026-11-20) : décisions D1–D4 ; objections retenues de la contre-revue et de l'arbitrage (injection de prompt,
  identité des relecteurs, règles appliquées par le serveur, P-1 corrigé, page de revue, crédits prépayés, instance
  dédiée, calibration en M0, mode dégradé en P0, RGPD avant pilote) ; corrections factuelles (identifiant non daté,
  `rgaa report verify`, origine des chiffres 8 % / 19 %, jeton MCP optionnel, auteur de revue en texte libre, directive
  2019/882, écarts au PDF).

## Sources

- `Strategie_Rust_accessibilite_Europe.pdf` (nov. 2026, document de travail)
- [Mistral Large 4 — page modèle](https://docs.mistral.ai/models/mistral-large-4)
- [Changelog Mistral](https://docs.mistral.ai/resources/changelogs)
- [Reasoning — Mistral](https://docs.mistral.ai/inference/conversations/reasoning)
- [Structured Outputs — Mistral](https://docs.mistral.ai/inference/conversations/structured-output/custom)
