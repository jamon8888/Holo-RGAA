# PRD — Holo-RGAA Europe : SaaS minimal, plugins agents, Mistral Large 4, GitHub

| | |
|---|---|
| **Statut** | **v3** (2026-11-20) — décisions D1–D4 + 10 tickets tranchés de la carte [Pilote payant M1 Holo-RGAA Europe](https://github.com/jamon8888/Holo-RGAA/issues/401). 5 tickets restent ouverts (§15) : ce PRD n'est pas encore la destination de la carte. |
| **Portée** | Produit, offre, exigences fonctionnelles et non fonctionnelles, jalons. |
| **Entrées** | Brief du porteur (prime sur le PDF) · `Strategie_Rust_accessibilite_Europe.pdf` (nov. 2026) · code `rgaa-rs/` · documentation officielle Mistral · recherches et décisions de la carte (liens dans le texte). |
| **Revues** | v1 relue par une contre-revue adversariale (Sonnet 5.5), arbitrée (Opus 5.5) ; v2 ; v3 = décisions de la carte. |
| **Documents liés** | `offre-europe-saas-plugins.md` · `p0-et-mistral-large-4.md` · `docs/specs/human-criteria-automation.md` · `GLOSSARY.md` |
| **Vocabulaire** | `GLOSSARY.md` : *Offre Souveraine*, *Offre Confort*, *Crédit*, *Relecteur*, *Décision de revue*, *Verdict*, *Test RGAA*, *Automatique* / *Assisté* / *Irréductible*, *Famille de preuve*, *Calibration*. |

Convention de preuve : **[code]** constaté dans le repo · **[PDF]** affirmé par le document de stratégie, non revérifié ·
**[Mistral]** documentation officielle · **[mesure]** mesuré sur la carte · **[recherche]** note de recherche de la carte ·
**[hypothèse]** à valider · **[ouvert]** question sans réponse.

---

## 0. Décisions arrêtées

### Décisions de cadrage (v2)

| ID | Décision |
|---|---|
| **D1** | **Deux offres** : **Souveraine** (IA `off` ou `byok`, CLI + rapports HTML, aucun agent tiers requis) et **Confort** (passerelle Mistral `managed` + plugins dans les agents du client). « Sans dépendance américaine » ne s'applique qu'à la Souveraine. |
| **D2** | **L'agent guide, la décision de revue est confirmée hors bande** sur une page de revue, par un relecteur authentifié. Aucun outil d'agent n'écrit de statut. |
| **D3** | **Crédits prépayés avec plafond dur**, débités depuis le journal d'usage. |
| **D4** | **Pilote M1 en `managed` sur instance dédiée par client**, sans multi-tenant ; facturation par export du journal. |

### Décisions de la carte (v3)

| Ticket | Décision résumée |
|---|---|
| [Forme de l'instance dédiée du pilote](https://github.com/jamon8888/Holo-RGAA/issues/408) | **Les audits tournent chez le client** (MCP + Obscura locaux). L'instance, **OVHcloud**, opérée par nous, une **VM `docker compose` par client**, porte la **passerelle LLM**, l'**historique** et la **page de revue**. Conservation : bundles 3 ans, captures 90 jours sauf preuves. Détail : §6, §9.2, NF-2. |
| [Page de revue minimale du pilote](https://github.com/jamon8888/Holo-RGAA/issues/409) | **File et détail** + avancement et raccourcis ; grille en synthèse lecture seule ; **connexion par lien e-mail** (passkey en option) ; non-conformité toujours décidée en vue détaillée avec commentaire. Détail : F-VIS-8. |
| [Retirer le verdict modèle de la réponse MCP pour les tests Assistés](https://github.com/jamon8888/Holo-RGAA/issues/410) | Réponse MCP **sans texte de verdict ni confiance** pour un test Assisté ; conditions structurées + preuve + rang ; appliqué dans le DTO pour tous les clients. Détail : R3, F-PLG-3. |
| [Plugins livrés au pilote](https://github.com/jamon8888/Holo-RGAA/issues/411) | **Un plugin par client agent** (`rgaa-consultant`, `rgaa-codex`), affiché « RGAA Audit » ; `acces-accessibility` reste interne. Détail : §9.3. |
| [Action GitHub du pilote](https://github.com/jamon8888/Holo-RGAA/issues/412) | **Lint statique seul**, `action.yml` dans ce dépôt, binaire `rgaa-lint` épinglé et vérifié. Détail : §9.5. |
| [Faits Mistral Large 4](https://github.com/jamon8888/Holo-RGAA/issues/402) · [Conditions d'usage et topologie des agents navigateur](https://github.com/jamon8888/Holo-RGAA/issues/403) · [Obligations RGPD](https://github.com/jamon8888/Holo-RGAA/issues/404) | Recherches : intégrées en §6, §8.2, §10, §14. |
| [Mesurer le coût réel de 3 audits sous Mistral Large 4](https://github.com/jamon8888/Holo-RGAA/issues/405) | Mesure partielle : **≈ 0,13 $ par page** en `low`, **83 % du coût = sortie**. Détail : §8.2. |

**Écarts assumés au PDF** : le PDF vend une solution « jamais en SaaS obligatoire » et « sans dépendance à un éditeur
américain cloud » [PDF §4.1, §2.3]. D1 garde cet argument pour l'offre Souveraine et le retire de l'offre Confort.

## 1. Résumé

Holo-RGAA devient une **plateforme d'audit et de remédiation d'accessibilité pilotée par des agents**. Le cœur (moteur
Rust, CLI, MCP, API) reste local et ouvert, et **les audits tournent chez le client** (poste ou CI). Un **SaaS minimal**,
livré au pilote comme une **instance dédiée par client**, fournit seulement ce qui engage l'organisation : la
**passerelle LLM mesurée** (Mistral Large 4, crédits prépayés), l'**historique** des bundles, et la **page de revue**
où un relecteur authentifié prend ses décisions. Dans l'offre Confort, l'expérience passe par le plugin « RGAA Audit »
dans Claude Code et Codex ; le guidage visuel dans le navigateur (Claude for Chrome, Codex for Chrome) arrive en M2.
L'offre Souveraine fournit le même moteur sans agent tiers. Une **GitHub Action** de lint statique porte la
vérification dans la CI des clients.

Le produit vend un **instrument de mise en conformité** (détection, correction, preuve), **jamais une certification**.

## 2. Problème et opportunité

- **Obligation légale de masse** : l'European Accessibility Act (directive (UE) **2019/882** — le PDF cite par erreur
  « 2019/884 ») s'applique depuis le 28 juin 2025 au commerce électronique, à la banque, au transport, aux télécoms ;
  en France, sanctions jusqu'à 50 000 € par service, renouvelables tous les six mois [PDF §2.1].
- **Le web échoue massivement** : 94,8 % des pages d'accueil ont au moins un échec WCAG (WebAIM 2025) [PDF §2.1].
- **Les agents de code aggravent la dette** : 94,59 % des modèles récents produisent du HTML à défauts critiques ou
  sérieux (Deque 2026) [PDF §2.2].
- **Souveraineté** : l'exécution locale vérifiable devient un critère d'achat (banque, santé, public) [PDF §2.3],
  servi par l'offre Souveraine.
- **Plafond de l'automatisation** : sur 117 *Tests RGAA* réputés humains, 16 sont *Automatiques*, 82 *Assistés*, 19
  *Irréductibles* [code : `docs/specs/human-criteria-automation.md` §1]. La part humaine est structurelle : le produit
  l'outille.

**Positionnement** : valeur commune aux deux offres = **RGAA natif, documents officiels de 12 pays, piste d'audit
attribuée, revue guidée**. L'argument « local vérifiable » appartient à l'offre Souveraine seulement.

## 3. Objectifs et non-objectifs

### Objectifs (18 mois)

| ID | Objectif | Mesure de succès |
|---|---|---|
| O1 | Un développeur obtient un premier audit et un premier correctif vérifié dans son agent | < 10 min depuis l'installation, sur un corpus de démonstration fixé (PDF §4.2) |
| O1b | Un relecteur non développeur lit un rapport et confirme une décision sans terminal | Parcours complet sur la page de revue, mesuré en pilote |
| O2 | Un relecteur traite la file de revue *Assistée* plus vite qu'un audit manuel | Temps par *Test RGAA* revu, mesuré sur pilote [hypothèse de cible] |
| O3 | La consommation IA est refacturée avec marge positive et sans surprise | 100 % des appels tracés ; aucun dépassement du solde ; écart journal / facture Mistral < 2 % [hypothèse de seuil] |
| O4 | La CI client bloque les régressions d'accessibilité | Action installable en 1 fichier ; PR de régression en échec |
| O5 | Preuve opposable | Chaque décision rattachée à un compte personnel, à un horodatage et au hash de la preuve examinée |
| O6 | Phases du PDF | P1 adoption · P2 5-10 pilotes FR + 3-5 cabinets · P3 DE/Benelux [PDF §6] |

### Non-objectifs

- **Pas d'overlay** ni de correctif injecté à l'exécution [PDF §1.2, §5.2].
- **Pas de « conformité garantie »**, pas de certification [PDF §5.2, §9].
- **Pas d'éditeur d'audit dans le SaaS** : console de compte et page de revue seulement.
- **Aucun `pass` ni `fail` décidé par un modèle sur un test *Assisté*** (§7).
- **Aucun outil d'agent n'écrit un statut** (D2, R7).
- **Pas de vente d'audit humain** par l'éditeur [PDF §7.3].
- Hors pilote : multi-tenant, guide visuel navigateur, signature ed25519, SSO, application mobile, PDF/UA, plugins WASM.

## 4. Personas

| Persona | Contexte | Besoin principal | Surface |
|---|---|---|---|
| **Dev produit** | Équipe 5-50 devs, Claude Code ou Codex | Ne pas introduire de défauts ; corriger vite | Plugin « RGAA Audit », CLI, Action GitHub |
| **Auditeur·rice** (cabinet ou interne) | Réalise l'audit RGAA, signe le rapport | Preuves rassemblées, file ordonnée, décision tracée | Page de revue (relecteur) ; plugin pour lancer et corriger |
| **Responsable conformité** | Banque, assurance, e-commerce, public | Déclaration, taux, historique, preuve | Rapports, pages de grilles, page de revue (relecteur) |
| **Lead DevOps** | Possède la CI | Gate fiable, binaire vérifié, pas de fuite de code | Action GitHub |
| **Admin de compte** | Côté client | Clés, crédits, factures | Console minimale de l'instance |
| **Partenaire cabinet** | Profils illustratifs : Access42, Temesis, Ideance, Funka [PDF §7] | Industrialiser l'audit, revendre | Licence partenaire, tarif revendeur IA |

Le *Relecteur* est un rôle, pas un persona : un auditeur comme un responsable conformité peut l'endosser.

## 5. Existant (constaté dans le code)

| Brique | État | Référence |
|---|---|---|
| Catalogue 106 critères, mapping axe | Livré | `rgaa-core`, `rgaa-rules`, `rgaa-data` |
| Pipeline d'audit (axe + gap-fix + LLM + clavier), agent `rig` | Livré ; une page = **22 appels LLM de 5 critères** [mesure] | `rgaa-orchestrator`, `rgaa-agent`, `rgaa-obscura` (Obscura 0.2.4) |
| CLI `rgaa audit analyze / igt / verify / report / review / policy` ; formats json, markdown, sarif, junit, html. `audit verify` vérifie des **propositions de remédiation** | Livré | `rgaa-cli/src/commands/` |
| MCP : `analyze`, `audit_url`, `get_audit_result`, `remediate`, `verify_fix`, `source_map`, `lint_static`, `list_criteria`, `igt` (déprécié). Aucun outil d'écriture de décision. Un test Assisté rend déjà `NeedsReview`, mais le verdict fuit par `description` / `details` | Livré, stdio + HTTP | `rgaa-mcp/src/server.rs`, `tools/analyze.rs` |
| MCP HTTP : jeton partagé `RGAA_MCP_TOKEN` **optionnel** | Livré | `rgaa-mcp-http/src/lib.rs` |
| Plugins `rgaa-consultant` (Claude Code), `rgaa-codex`, `acces-accessibility` | Livré | `rgaa-rs/plugins/` |
| Revue humaine enregistrée — auteur en **texte libre** (`--author`) | Livré (CLI) | `review.rs` |
| Captures : le bundle ne garde que des **références** (type, hash SHA-256, emplacement de fichier) | Livré | `rgaa-core/src/findings.rs` |
| Documents officiels 12 pays | Livré | `rgaa-report` |
| API REST (bundles, findings, policy, batch, webhooks signés) | Livré, mono-tenant ; **`/audit` et `/audit/:id` sans authentification**, CORS ouvert | `rgaa-api/src/lib.rs` |
| **Support Mistral Large 4** : blocs `thinking`/`text`, défaut `mistral-large-4`, `RGAA_LLM_REASONING_EFFORT` | **Livré** (commit `2623815`, PR #400) | `rgaa-holo/src/transport.rs`, `rgaa-core/src/{provider,completion}.rs` |
| Mesure de tokens | Événement `tracing` « model call cost » côté `rig` ; non stocké | `rgaa-agent/src/metrics.rs` |
| Route `custom` : envoie `think` / `enable_thinking`, rejetés en 400 par NVIDIA | Défaut constaté [mesure] | `rgaa-core/src/completion.rs` |
| Port Obscura fixe 9222 réutilisé s'il est déjà occupé, sans la configuration de l'appelant | Défaut constaté [mesure] | `ObscuraBridge::from_env()` |
| Organisations, crédits, passerelle LLM, page de revue, binaire lint, Action, signature | **Absents** | §9 |

## 6. Architecture produit cible (pilote)

```
 Chez le client                                                   Instance dédiée (OVHcloud, UE, opérée par nous)
 ┌──────────────────────────────────────────┐   HTTPS + clé    ┌────────────────────────────────────────┐
 │ Claude Code / Codex + plugin RGAA Audit  │─────────────────▶│ Passerelle LLM compatible OpenAI        │──▶ Mistral
 │ rgaa-mcp (stdio) · rgaa-cli              │  (RGAA_LLM_      │  - clé Mistral de l'instance            │    Large 4
 │ Obscura (navigateur) · audit local       │   BASE_URL)      │  - crédits débités avant l'appel        │
 │ GitHub Action rgaa-lint (CI)             │                  │  - journal d'usage (sans contenu)       │
 └──────────────────────────────────────────┘                  │ Historique des bundles (Postgres)       │
            │ bundle (statuts, hashes)                          │ Page de revue (relecteur, lien e-mail)  │
            └──────────────────────────────────────────────────▶│ Reverse proxy TLS, sous-domaine client  │
                                                                └────────────────────────────────────────┘
 Offre Souveraine : même poste client, IA off ou byok, page de revue auto-hébergeable, aucune instance chez nous.
```

Principes :
- **P-1 Flux de données déclarés.** Les audits tournent chez le client : code et DOM y restent. Vers l'instance
  partent les **appels LLM** (prompt et image d'un lot de critères) et le **bundle** (statuts, décisions, hashes de
  preuve). Les **captures** sont des fichiers référencés par hash : elles ne partent que si une décision de revue les
  examine [hypothèse de conception, à confirmer à l'exécution]. Quand un **client agent** lit une page (Claude Code,
  Codex, et en M2 Claude for Chrome / Codex for Chrome), le texte, les captures et, chez Anthropic, le DOM partent vers
  l'éditeur de cet agent [recherche #403].
- **P-2 Une seule vérité.** Plugins, Action, page de revue et SaaS consomment le même bundle ; aucune surface ne
  recalcule un verdict.
- **P-3 IA mesurée.** Tout appel de modèle passe par la passerelle (Confort) ou est mesuré localement (`byok`).
- **P-4 Trois modes IA** : `off` · `byok` (clé du client, 0 % de marge) · `managed` (passerelle, crédits).
- **P-5 Les règles sont appliquées par le serveur**, jamais par un prompt ou un écran (R7).

**Topologie des agents navigateur (M2)** [recherche #403] : les connecteurs distants d'Anthropic (claude.ai, Desktop,
Cowork) joignent un serveur MCP **depuis l'infrastructure d'Anthropic** : il faut alors un MCP **public en HTTPS sur
443**, avec OAuth ; l'allowlist `chrome-extension://` ne suffit pas. Claude Code et Codex lisent un MCP **local** : le
pilote n'a donc aucun MCP à exposer. Le guide visuel de M2 devra revenir sur « audit côté client ».

## 7. Règles de décision non négociables

- **R1** Un *Test RGAA* *Assisté* rend `review`. Jamais `pass` ni `fail`.
- **R2** Aucun `fail` ne provient d'un verdict de modèle (8 % de faux « non pertinent » mesurés sur la seule famille
  calibrée, alternative d'image, avec un autre modèle ; **non mesuré sur Large 4**).
- **R3** Le relecteur voit la preuve et les conditions structurées, **pas** le verdict du modèle (19 % de mauvaises
  alternatives validées à tort, famille alternative d'image, `swift-1.5-27b` ; **non mesuré sur Large 4**). Le verdict
  ordonne la file, sans être montré. **Appliqué dans la réponse MCP elle-même** : pour un test Assisté, ni texte de
  verdict, ni `confidence`, ni `raw_confidence` ; à la place, les conditions (`oui` / `non` / `sans objet`, motivées),
  la preuve et un rang de priorité ([#410](https://github.com/jamon8888/Holo-RGAA/issues/410)).
- **R4** Le computer use **provoque des états et enrichit la preuve** ; il ne fait monter aucun test d'un niveau. Une
  capture versée au bundle est **prise par notre outil, puis hachée**.
- **R5** Toute décision de revue est horodatée, rattachée à un **compte personnel authentifié** et au **hash de la
  preuve examinée**, et versionnée dans le bundle.
- **R6** Un patch n'est jamais appliqué sans validation humaine ; en CI, il prend la forme d'une PR.
- **R7** **Aucun outil exposé à un agent n'écrit un statut.** Le serveur refuse toute écriture de statut sans identité
  humaine authentifiée.

## 8. Offre et tarification

### 8.1 Offres et paliers

| | **Offre Souveraine** | **Offre Confort** |
|---|---|---|
| Promesse | Rien ne sort sans votre accord ; aucun agent tiers requis | Audit et remédiation guidés dans vos agents, IA européenne incluse |
| IA | `off` ou `byok` (clé Mistral du client, ou modèle local) | `managed` : passerelle Mistral de l'instance, crédits prépayés |
| UX | CLI, rapports et pages de grilles HTML, page de revue auto-hébergeable | Plugin « RGAA Audit » (Claude Code, Codex) + page de revue de l'instance |
| Argument « sans dépendance US » | Oui | Non (flux déclarés, P-1) |
| Cible prioritaire | Public, banque, santé, défense | E-commerce, ESN, équipes produit, cabinets |

| Palier | Contenu | Prix [hypothèse] |
|---|---|---|
| **Community** (Apache-2.0) | Moteur, CLI, MCP, linter, plugins, Action GitHub, `byok` | Gratuit |
| **Team** | + instance (historique, page de revue), passerelle `managed`, support | Abonnement + crédits IA prépayés |
| **Enterprise** | + multi-sites, déclarations 12 pays à jour, SSO/RBAC (v2), SLA | Contrat annuel + crédits IA |
| **Partenaire** | Licence Team offerte, tarif revendeur IA, certification | Revenue share [PDF §7.3] |

Valeur d'un crédit, marge cible et grille du pilote : **ouvert**, ticket
[Valeur d'un crédit, marge cible et grille du pilote](https://github.com/jamon8888/Holo-RGAA/issues/407).

### 8.2 Coût de l'IA et refacturation (D3)

**Faits Mistral Large 4** [Mistral, recherche #402] : identifiants `mistral-large-4`, `mistral-large-4-0`,
`mistral-large-latest` ; GA le 2026-11-05 ; contexte **1M** (aucune source primaire pour 512K) ; prix GA **0,70 $**
(entrée), **0,07 $** (entrée en cache), **2,10 $** (sortie) par million de tokens ; le modèle **raisonne toujours**,
`reasoning_effort` ∈ {`low`, `medium`, `high`}, défaut `high`, `none` refusé ; raisonnement **facturé en sortie**.
Non publiés : limites de débit (console seulement), sortie maximale, facturation des appels échoués.

**Mesure** [mesure #405] — endpoint NVIDIA (mêmes poids), offre gratuite, 1 page, effort `low` :

| Mesure | Valeur |
|---|---|
| Appels LLM par page | **22** (106 critères par lots de 5) |
| Tokens par appel abouti | entrée **1 503** · sortie **2 377** (moyennes) |
| Coût par appel au tarif Mistral | **≈ 0,006 $**, dont **83 % de sortie** |
| **Coût par page** | **≈ 0,13 $** en `low` |
| **Site de 15 pages** | **≈ 2 $** en `low` |

Limites : `high` non mesurable (coupé à 340 s par NVIDIA) et nettement plus cher puisque la sortie domine ; seuls 10
appels sur 22 ont abouti (biais vers le bas) ; une seule page de test courte ; latence NVIDIA non représentative. La
mesure définitive se fait sur l'API Mistral : ticket
[Mesurer coût et latence de Mistral Large 4 sur l'API Mistral, par niveau d'effort](https://github.com/jamon8888/Holo-RGAA/issues/414).

**Mécanisme des crédits** : le client achète des crédits à l'avance ; la passerelle débite chaque appel selon le journal
`llm_usage` et le barème versionné `price_book` ; **solde vérifié avant l'appel**, **plafond dur** ; solde épuisé → le
test reste `review` avec la cause `budget` ; alertes 50/80/100 % ; `byok` sans crédits. **L'effort de raisonnement et le
découpage des lots sont les deux leviers de prix** (et de latence).

## 9. Exigences fonctionnelles

Priorités : **P0** pilote M1 · **P1** M2 (grille Team) · **P2** Enterprise. **Fait** = livré dans le code.

### 9.1 IA — Mistral Large 4 (`F-LLM`)

| ID | Exigence | Prio | État / critère d'acceptation |
|---|---|---|---|
| F-LLM-1 | Preset `mistral` → `mistral-large-4` par défaut (identifiant versionné, non `-latest`). **Non admis** au sens de la spec §2.6 tant que F-LLM-9 n'est pas faite | P0 | **Fait** (code) ; admission ouverte |
| F-LLM-2 | Réponse en **liste de blocs** (`thinking`, `text`) lue, seul le texte compte | P0 | **Fait** (2 tests) ; à confirmer contre l'API Mistral réelle |
| F-LLM-3 | `RGAA_LLM_REASONING_EFFORT` (`low`/`medium`/`high`), `none` refusé, tracé dans la provenance | P0 | **Fait** (3 tests) ; à rendre réglable **par tier** |
| F-LLM-4 | Ne jamais envoyer `chat_template_kwargs` / `think` / `enable_thinking` à un fournisseur distant | P0 | NVIDIA les rejette en 400 [mesure] ; corriger la route `custom` qui les envoie |
| F-LLM-5 | **Usage lu et stocké** sur les deux chemins (entrée, cache, sortie, raisonnement) | P0 | Aujourd'hui : événement `tracing` côté `rig` seulement |
| F-LLM-6 | `max_tokens` dimensionné pour le raisonnement | P0 | 0 verdict tronqué sur le corpus |
| F-LLM-7 | Sortie structurée : `json_object` + validation locale ; `json_schema` (`name` + `schema` obligatoires, `strict` facultatif, effet non documenté [recherche #402]) après test réel | P1 | Décision après test |
| F-LLM-8 | Image : PNG/JPEG/WEBP, taille bornée (sources Mistral contradictoires : 8 images / 10 Mo ou 20 Mo) | P1 | Limite mesurée sur l'API |
| F-LLM-9 | **Calibration** de Large 4 par *Famille de preuve*, **avant tout pilote facturé** | P0 | **Bloquée** : aucune étiquette humaine dans le dépôt ([#415](https://github.com/jamon8888/Holo-RGAA/issues/415)) |
| F-LLM-10 | Mode dégradé : panne Mistral → audit déterministe complet + tests IA en `review` cause `llm_unavailable` | P0 | Test de panne simulée |
| F-LLM-11 | Fournisseur secondaire ou modèle local via la route de repli | P1 | — |
| F-LLM-12 | Découpage des lots et effort **par critère** réglables (levier de coût et de latence) | P1 | Après la mesure sur l'API Mistral |

### 9.2 Instance dédiée, comptes, passerelle, crédits (`F-INST`, `F-ACC`, `F-BILL`)

Décisions : [Forme de l'instance dédiée du pilote](https://github.com/jamon8888/Holo-RGAA/issues/408).

| ID | Exigence | Prio | Critère d'acceptation |
|---|---|---|---|
| F-INST-1 | Instance **OVHcloud**, région UE, **une VM par client**, `docker compose` (`rgaa-api`, Postgres, page de revue ; **pas d'Obscura**), reverse proxy TLS | P0 | Déploiement reproductible ; **sans** mots de passe par défaut ni pgAdmin exposé |
| F-INST-2 | **Opérée par nous** : installation, mises à jour, sauvegardes, supervision | P0 | Runbook écrit |
| F-INST-3 | **Sous-domaine par client**, HTTPS seulement ; liste blanche d'IP en option | P0 | — |
| F-INST-4 | **Sauvegardes** : dump Postgres chiffré quotidien, même région, 30 jours ; **test de restauration avant ouverture** | P0 | Restauration réussie consignée |
| F-INST-5 | **Supervision** : disponibilité, erreurs, solde de crédits, coût par jour ; journaux **sans** contenu de page, prompt ni réponse | P0 | Revue des journaux |
| F-INST-6 | **Mises à jour** : version épinglée, fenêtre annoncée, après CI verte, retour arrière par l'image précédente | P0 | — |
| F-ACC-0 | `/audit` et `/audit/:id` exigent une clé ; CORS restreint | P0 | Sans clé → 401 |
| F-ACC-4 | MCP HTTP : clé **obligatoire** | P0 | Sans clé → 401 |
| F-ACC-7 | **Comptes personnels** de relecteurs, connexion par **lien e-mail** (passkey en option) | P0 | Chaque décision référence un compte |
| F-ACC-2/3/5/6 | Multi-tenant, contexte d'auth propagé, allowlist d'origines, rotation et scopes de clés | P1 | Carte M2 |
| F-BILL-1 | Mesure du coût réel | P0 | **Partielle** (§8.2) ; définitive : [#414](https://github.com/jamon8888/Holo-RGAA/issues/414) |
| F-BILL-2 | Journal `llm_usage` (instance, audit, test, modèle, effort, tokens entrée/cache/sortie, latence, statut) | P0 | Somme du journal = relevé Mistral de la clé de l'instance à ±2 % |
| F-BILL-3 | Barème `price_book` versionné, relu chaque mois | P0 | — |
| F-BILL-4 | **Passerelle compatible OpenAI** (`/v1/chat/completions`) : détient la clé Mistral (**une par instance**), **débite le solde avant l'appel**, plafond dur | P0 | Solde nul → `review` cause `budget`, aucun appel émis ; côté client, seul `RGAA_LLM_BASE_URL` change |
| F-BILL-5 | Export du journal par période, base de la facture du pilote | P0 | Rapproché de la console Mistral |
| F-BILL-6/7/8 | Estimation avant audit, achat en ligne (Stripe), `byok` mesuré localement | P1 | Carte M2 |

### 9.3 Plugins agents (`F-PLG`)

Décisions : [Plugins livrés au pilote](https://github.com/jamon8888/Holo-RGAA/issues/411).

| ID | Exigence | Prio | Critère d'acceptation |
|---|---|---|---|
| F-PLG-1 | **Un plugin par client agent** : `rgaa-consultant` (Claude Code) et `rgaa-codex` (Codex), affichés **« RGAA Audit »**, identifiants techniques inchangés | P0 | Installation en une commande par client |
| F-PLG-2 | **Skills mis en avant** : audit, remédiation, vérification, rapport, triage ; `guided-test` et `criteria` disponibles sans être mis en avant | P0 | Parcours O1 < 10 min |
| F-PLG-3 | Les plugins n'affichent **aucun verdict modèle** d'un test Assisté (la réponse MCP n'en contient plus, R3) et renvoient vers la page de revue pour décider | P0 | Mis à jour dans le même changement que le DTO MCP |
| F-PLG-4 | Aucune logique métier dans les plugins : tout passe par MCP/CLI/API | P0 | Même résultat via plugin, CLI et Action |
| F-PLG-5 | `acces-accessibility` **n'est pas livré** au pilote (outil interne) | P0 | — |
| F-PLG-6 | Guide visuel navigateur, installation générée (`CLAUDE.md` / `AGENTS.md`) | P1 | Carte M2 |

### 9.4 Page de revue et guidage visuel (`F-VIS`)

Décisions : [Page de revue minimale du pilote](https://github.com/jamon8888/Holo-RGAA/issues/409) ; prototype sur la
branche jetable `prototype/page-revue`.

| ID | Exigence | Prio | Critère d'acceptation |
|---|---|---|---|
| F-VIS-8 | **Page de revue** : **file et détail** (file ordonnée par priorité à gauche ; preuve, faits extraits et conditions à droite) ; **barre d'avancement** ; **raccourcis** 1 / 2 / 3 ; décisions Conforme / Non conforme / Non applicable ; **commentaire obligatoire** pour Non conforme ; annonce en tête que le verdict modèle n'est pas affiché | P0 | Parcours O1b ; page elle-même accessible (NF-8) |
| F-VIS-9 | Chaque décision écrite dans le bundle porte : test, page, décision, commentaire, **compte personnel**, méthode d'authentification, horodatage, **hash de la preuve examinée** | P0 | Test serveur : refus sans compte personnel |
| F-VIS-10 | **Vue grille** en synthèse **lecture seule** pour le responsable conformité ; une non-conformité se décide toujours en vue détaillée | P1 | Aucune décision possible depuis la grille |
| F-VIS-6 | Pages de grilles HTML avec captures, statut et décision humaine | P0 | Accessibles |
| F-VIS-1 à 4, 7 | Agent navigateur : mise en évidence, états provoqués (capture refaite par notre outil), remédiation pas à pas, mode dégradé | P1 | Carte M2, après la topologie (§6) |

### 9.5 Version GitHub (`F-GH`)

Décisions : [Action GitHub du pilote](https://github.com/jamon8888/Holo-RGAA/issues/412).

| ID | Exigence | Prio | Critère d'acceptation |
|---|---|---|---|
| F-GH-1 | Binaire **`rgaa-lint`** autour de `rgaa-linter` (inexistant aujourd'hui), sorties `github`, `sarif`, `json` ; ajouté aux artefacts de `release.yml` | P0 | Annotations sur les lignes modifiées |
| F-GH-2 | **`action.yml` dans ce dépôt** ; télécharge `rgaa-lint` d'une **release épinglée**, **SHA-256 vérifié** ; pas de compilation dans la CI du client | P0 | Mauvaise somme → échec |
| F-GH-3 | **Lint statique seulement** (HTML, JSX/TSX, Vue), sans navigateur ni IA ; SARIF vers *Code scanning* ; check en échec en cas de régression | P0 | — |
| F-GH-7 | Permissions minimales ; `pull_request_target` interdit avec du code de PR | P0 | Revue sécurité |
| F-GH-4/5/6/8/9 | Audit d'URL, IA en CI, PR de remédiation, signature + SBOM, Marketplace (dépôt dédié) | P1 | Carte M2 |

### 9.6 Preuve et rapports (`F-PRV`)

| ID | Exigence | Prio |
|---|---|---|
| F-PRV-4 | Provenance de chaque verdict : moteur, version de règles, modèle, `reasoning_effort` (**fait** dans la provenance LLM), décision humaine et compte | P0 |
| F-PRV-5 | Garde-fous d'export existants conservés | P0 |
| F-PRV-1/2 | Signature ed25519, `rgaa report verify`, clés par organisation | P1 |
| F-PRV-3 | Horodatage par un tiers | P2 |

### 9.7 Sécurité des agents (`F-SEC`)

| ID | Exigence | Prio |
|---|---|---|
| F-SEC-1 | Outils MCP exposés aux agents **sans écriture de statut** (R7) ; écritures de code soumises à l'accord humain (R6) | P0 |
| F-SEC-3 | **Page piégée** dans le corpus de CI. Le corpus contient déjà `1.1-prompt-injection-adversarial-fail.html` : à étendre (écrire une décision, exfiltrer une clé, appliquer un patch) | P0 |
| F-SEC-4 | Refus serveur de toute écriture de statut sans identité humaine | P0 |
| F-SEC-2/5 | Contenu balisé comme donnée ; fiche de flux par client agent | P1 |

## 10. Exigences non fonctionnelles

| ID | Exigence |
|---|---|
| NF-1 Souveraineté | Souveraine : aucune donnée hors du client sans accord. Confort : instance et données dans l'UE (OVHcloud), Mistral fournisseur UE, flux vers les éditeurs d'agents déclarés (P-1). Avant toute activation de l'envoi de captures à une API de modèle : **DPA, région UE et non-entraînement exigés** ; **jamais de page authentifiée** envoyée à une API externe (décision de la carte « Automatiser au maximum les tests RGAA réputés humains »). |
| NF-2 RGPD | Le SaaS est **sous-traitant** (art. 28) : **DPA** couvrant l'art. 28.3 a–h, **registre** (art. 30.2, sans exemption des < 250 salariés pour un traitement continu) [recherche #404]. Base légale des données de tiers visibles : à la charge du client. **Conservation** (#408, à valider par un juriste) : bundles **3 ans** (durée du schéma pluriannuel RGAA, art. 47 loi 2005-102) ; captures **90 jours**, sauf celles rattachées à une non-conformité ou à une décision de revue ; journal d'usage : durée comptable, sans contenu. AIPD : non tranchée. Offre Confort : vérifier la certification DPF des éditeurs d'agents. |
| NF-3 Zéro télémétrie par défaut | Le binaire local n'envoie rien sans activation explicite. |
| NF-4 Chaîne d'approvisionnement | Binaires épinglés et vérifiés (SHA-256) ; signatures et SBOM en M2 ; `cargo audit` en CI. |
| NF-5 Sécurité | Obscura : réseau privé refusé par défaut ; **port CDP à rendre non partagé** (défaut constaté, §5) ; MCP HTTP jamais sans clé. |
| NF-6 Performance | Latence par appel et par page mesurée sur l'API Mistral (#414) ; à l'effort `high`, la **durée d'un audit** peut freiner avant le coût. |
| NF-7 Fiabilité IA | Panne ou solde épuisé → jamais d'échec silencieux ni de verdict inventé ; `review` avec cause. |
| NF-8 Accessibilité des livrables | Rapports, pages de grilles et page de revue conformes au RGAA eux-mêmes. |
| NF-9 Langues | FR, EN, DE pour docs et rapports [PDF §6.1]. |
| NF-10 Transparence | Couverture réelle par niveau publiée ; corriger l'écart entre le README (73/32/1) et la spec des niveaux (117 tests : 16/82/19). |
| NF-11 Responsabilité | Clauses : instrument de mise en conformité, non-certification, limitation de responsabilité ; attestation humaine par les partenaires. |

## 11. Indicateurs

- **Adoption** : installations, dépôts utilisant l'Action, instances actives.
- **Activation** : premier correctif vérifié (O1) ; première décision sur la page de revue (O1b).
- **Revue** : temps par *Test RGAA* revu, taille de la file, décisions modifiées après revue.
- **IA** : appels, tokens et coût par page, par *Famille de preuve*, par effort ; marge brute ; consommation et
  rechargement des crédits ; écart journal / relevé Mistral.
- **Qualité** : accord de *Calibration* par famille ; verdicts tronqués (cible 0).
- **Sécurité** : test de page piégée (cible : aucun effet).
- **Économie** : mix licences / services ; répartition Souveraine / Confort.

## 12. Jalons

| Jalon | Contenu | Sortie |
|---|---|---|
| **M0 — Socle** | F-ACC-0, F-ACC-4, F-LLM-4 à 6, F-LLM-9 (calibration), F-LLM-10, F-BILL-1 (définitive), F-PRV-4 ; DPA, durées de conservation, clauses (NF-2, NF-11) ; port Obscura non partagé (NF-5) | Large 4 admis ou refusé ; coût et latence réels connus ; failles fermées |
| **M1 — Pilote payant** (D4) | F-INST-1 à 6, F-ACC-7, F-BILL-2 à 5 (passerelle, crédits), F-VIS-8, 9, 6, F-PLG-1 à 5, retrait du verdict du DTO MCP (R3), F-SEC-1, 3, 4, F-GH-1, 2, 3, 7 | Premier pilote facturé sur crédits ; décisions de revue opposables |
| **M2 — Team publique** | Multi-tenant, achat de crédits en ligne, guide visuel navigateur (MCP public + OAuth), F-VIS-10, F-GH complet, signature, F-LLM-7, 8, 11, 12 | Carte M2/M3 (hors périmètre de la carte du pilote) |
| **M3 — Enterprise** | Horodatage tiers, SSO/RBAC, Jira, multi-sites, packs DE/Benelux | Phase 3 du PDF |

## 13. Risques

| Risque | Gravité | Parade |
|---|---|---|
| Promesse juridique (« conforme ») | Élevée | Vocabulaire contractuel ; R1-R3 ; NF-11 ; attestation humaine des partenaires |
| Injection de prompt depuis une page auditée | Élevée | R7, F-SEC-1, 3, 4 |
| Large 4 non admis à la calibration | Élevée | Calibration en M0 ; modèle de repli décidé dans [Admettre ou non Mistral Large 4 après calibration](https://github.com/jamon8888/Holo-RGAA/issues/406) |
| Marge IA négative (raisonnement `high` facturé en sortie) | Élevée | Mesure sur l'API Mistral ; effort par tier ; découpage des lots ; crédits prépayés |
| Durée d'audit trop longue à l'effort `high` | Moyenne | Mesure de latence (#414) ; effort et lots par critère (F-LLM-12) |
| Étiquetage humain non réalisé | Élevée | Bloque la calibration : décider qui étiquette (porteur ou cabinet partenaire) |
| Conditions d'usage des agents navigateur | Moyenne | Pas dans le pilote ; relecture humaine des CGU (OpenAI illisibles en recherche) avant M2 |
| Fuite de données vers les éditeurs d'agents | Moyenne | P-1 ; offre Souveraine |
| Concurrent qui ajoute un MCP | Moyenne | RGAA natif, 12 pays, piste attribuée, revue guidée |

## 14. Questions ouvertes

1. Coût et latence réels sur l'API Mistral, par effort ; sortie maximale ; facturation des échecs ; limites de débit
   ([#414](https://github.com/jamon8888/Holo-RGAA/issues/414)).
2. Accord de Large 4 avec l'étiquetage humain, et donc son admission ([#415](https://github.com/jamon8888/Holo-RGAA/issues/415),
   [#406](https://github.com/jamon8888/Holo-RGAA/issues/406)) ; qui étiquette.
3. Valeur d'un crédit, marge cible, grille du pilote ([#407](https://github.com/jamon8888/Holo-RGAA/issues/407)).
4. Premier client pilote, critères de succès, contrat ([#413](https://github.com/jamon8888/Holo-RGAA/issues/413)).
5. Contenu de l'offre Souveraine pour un acheteur public (brouillard de la carte).
6. Format du test de page piégée (brouillard de la carte).
7. Effet réel de `strict` sur `json_schema` ; limites d'images de Large 4.
8. Comportement du chemin `rig` face aux blocs `thinking` de l'API Mistral (testé sur NVIDIA seulement, où le format diffère).
9. AIPD nécessaire ou non ; certification DPF des éditeurs d'agents ; 10 points RGPD à faire valider par un juriste.
10. CGU d'OpenAI pour Codex for Chrome (non lues) ; disponibilité UE d'OpenAI confirmée par une source officielle (M2).

## 15. État de la carte du pilote

[Pilote payant M1 Holo-RGAA Europe](https://github.com/jamon8888/Holo-RGAA/issues/401) : **10 tickets fermés, 5 ouverts**.

| Ticket ouvert | Bloqué par | Besoin |
|---|---|---|
| [Calibrer Mistral Large 4 sur les familles de preuve étiquetées](https://github.com/jamon8888/Holo-RGAA/issues/415) | Aucune étiquette humaine | Étiquetage d'un lot ; `pip` / `python3-venv` sur la machine |
| [Mesurer coût et latence de Mistral Large 4 sur l'API Mistral, par niveau d'effort](https://github.com/jamon8888/Holo-RGAA/issues/414) | — | Clé Mistral |
| [Admettre ou non Mistral Large 4 après calibration](https://github.com/jamon8888/Holo-RGAA/issues/406) | Calibration | Grilling |
| [Valeur d'un crédit, marge cible et grille du pilote](https://github.com/jamon8888/Holo-RGAA/issues/407) | Mesure Mistral | Grilling |
| [Premier client pilote : profil, critères de succès, contrat](https://github.com/jamon8888/Holo-RGAA/issues/413) | Grille de crédits | Grilling |

Quand ces tickets seront fermés, ce PRD deviendra la **v4**, destination de la carte, avec la spec de passage à
l'exécution.

## 16. Journal des révisions

- **v1** (2026-11-20) : première version.
- **v2** (2026-11-20) : décisions D1–D4 ; objections de la contre-revue et de l'arbitrage ; corrections factuelles.
- **v3** (2026-11-20) : décisions de la carte du pilote (instance dédiée, page de revue, verdict retiré du MCP, plugins,
  Action GitHub) ; recherches Mistral, agents navigateur, RGPD ; mesure de coût ; support Mistral Large 4 livré dans le
  code ; défauts constatés (route `custom`, port Obscura partagé) ; état des 5 tickets ouverts. Exigences renumérotées
  pour l'instance (`F-INST`) et la page de revue (`F-VIS-8` à `10`) ; F-LLM-12 ajoutée.

## Sources

- `Strategie_Rust_accessibilite_Europe.pdf` (nov. 2026, document de travail)
- [Mistral Large 4 — page modèle](https://docs.mistral.ai/models/mistral-large-4) · [Changelog Mistral](https://docs.mistral.ai/resources/changelogs) · [Reasoning](https://docs.mistral.ai/inference/conversations/reasoning)
- Notes de recherche de la carte : branches `research/mistral-large-4-faits`, `research/agents-navigateur-topologie`, `research/rgpd-captures`
- Prototype de page de revue : branche `prototype/page-revue`
