# Offre Holo-RGAA — SaaS minimal, plugins-first, Mistral, GitHub

> Statut : proposition (2026-11-20). Source : `Strategie_Rust_accessibilite_Europe.pdf` (nov. 2026,
> document de travail ; chiffres de marché non revérifiés ici) confronté au code de `rgaa-rs/`.
> **Détail précis des P0 et de Mistral Large 4 : `p0-et-mistral-large-4.md`** (il corrige et remplace §3 et la vérification « Mistral 4 » ci-dessous).
> Les prix sont des hypothèses à valider par étude de prix, comme le dit le PDF lui-même (§11.1).

## 1. Verdict en cinq lignes

1. Le PDF décrit une plateforme **locale / open-core** ; notre approche est un **SaaS minimal** qui
   refacture l'IA. Les deux se concilient : *moteur, CLI, MCP et plugins ouverts et locaux ; SaaS = plan de
   contrôle fin (clés, mesure de conso, historique, rapports signés)*. Un SaaS « obligatoire » casserait
   l'argument souveraineté (PDF §2.3, §4.1).
2. Le code couvre déjà l'essentiel des briques P0 du PDF (§3) ; les vrais trous sont **facturation,
   multi-tenant, action GitHub publiable, signature des rapports, SSO**.
3. Notre différenciateur par rapport au PDF : la **remédiation visuelle pas à pas** (Claude for Chrome /
   computer use) et les **12 pays** (le PDF vise FR puis DE/Benelux).
4. Garde-fous du PDF à garder tels quels : pas d'overlay, pas de « conformité garantie »,
   l'agent propose / l'humain décide, services plafonnés.
5. Mistral est cohérent avec le discours souveraineté (fournisseur UE), à condition de proposer aussi un
   mode « client apporte sa clé / modèle local ».

## 2. Ce que le code fournit déjà (vérifié)

| Brique du PDF (Tableau 2) | État dans le repo | Preuve |
|---|---|---|
| Moteur de règles + mapping en données | Présent : 106 critères, registre axe, mapping | `rgaa-core`, `rgaa-rules`, `rgaa-data`, `docs/specs/axe-rules-per-test-proposal.md` |
| CLI + sorties CI | Présent : rapports JSON/MD/SARIF/JUnit/HTML | `rgaa-cli/src/commands/report.rs` |
| API locale / daemon | Présent (REST, batch, webhooks signés) ; **pas d'auth multi-tenant ni de mesure d'usage** | `rgaa-api`, `docs/batch-webhooks.md`, `rgaa-storage` (Postgres) |
| Serveur MCP | Présent : stdio + HTTP ; outils `audit_url`, `analyze`, `lint_static`, `remediate`, `igt`, `source_map`, `list_criteria`, `get_audit_result` | `rgaa-mcp`, `rgaa-mcp-http` |
| Intégrations agents | Présent : plugins Claude Code (`rgaa-consultant`, `acces-accessibility`) et Codex (`rgaa-codex`) ; commandes audit-site / audit-project / generate-report ; skills audit, remediate, verify, guided-test, report, triage | `rgaa-rs/plugins/` |
| Linter statique (« avant le merge ») | Présent | `rgaa-linter` |
| Remédiation vérifiée | Présent : patchs par framework, re-vérification sur scan frais | `rgaa-remediation` |
| Navigateur / preuves | Présent : Obscura épinglé, captures, hash, AXTree | `rgaa-obscura`, `rgaa-browser-tools` |
| IA multi-fournisseur | Présent : `mistral` est un preset (`MISTRAL_API_KEY`) mais **sans modèle par défaut** | `rgaa-core/src/provider.rs:95` |
| Documents officiels 12 pays | Présent — **avance sur le PDF** | `rgaa-report`, README |
| Workflow GitHub | Existe pour **notre** repo (`rgaa-audit.yml`, modes changed/smoke/full/diff/policy) ; **aucune Action réutilisable publiée** (pas de `action.yml` suivi) | `.github/workflows/` |
| Signature ed25519 des rapports | **Seulement dans des specs/plans**, pas dans le code | `docs/superpowers/specs/2026-08-24-rgaa-distribution-design.md` |
| Jira, SSO/SAML, RBAC, extension navigateur, hôte WASM | **Absents** | grep sur le repo |

## 3. Écarts à combler, classés

**P0 — sans cela l'offre ne se vend pas**
- **Multi-tenant + clés + mesure de conso** : comptes, clés API par organisation, journal d'usage par appel LLM
  (tokens in/out, modèle, audit_id) dans `rgaa-storage`. C'est la base de la refacturation.
- **GitHub Action réutilisable** (`uses: jamon8888/rgaa-action@v1`) : voir §6.
- **Mistral de premier rang** : modèle par défaut explicite et épinglé, routage par niveau (petit modèle pour
  le tri, grand pour le jugement visuel), repli sur `NeedsReview` déjà prévu. *Identifiant vérifié : `mistral-large-4` (voir `p0-et-mistral-large-4.md`).*
- **Rapports signés** (ed25519) : implémenter ce que la spec décrit ; c'est l'argument « piste d'audit opposable ».

**P1**
- Mode « local-first » documenté : tout fonctionne sans le SaaS (clé du client ou modèle local `ollama`/`vllm`).
- Extension Chrome / flux Claude for Chrome (voir §5) comme entonnoir d'adoption.
- Jira + export VPAT/EN 301 549 ; mapping WCAG 2.2 progressif.

**P2** — SSO/SAML/RBAC, orchestrateur multi-sites consolidé, marketplace de plugins (WASM seulement si une
demande réelle apparaît ; le repo n'a pas de sandbox WASM).

## 4. Offre et prix (hypothèses)

Principe du PDF conservé : « ce qui aide chaque développeur est libre ; ce qui engage l'organisation est payant ».
Ajout : **l'IA est refacturée à l'usage**, séparée de l'abonnement, pour que la marge ne dépende pas du cours des tokens.

| Offre | Contenu | Facturation |
|---|---|---|
| **Community** (gratuit, open source) | Moteur, CLI, MCP, linter, plugins Claude Code / Codex, Action GitHub en mode déterministe (73 critères sans IA) | — |
| **Team** | + critères assistés par IA (32), remédiation guidée, rapports HTML/PDF, historique, support | Abonnement par dépôt ou par dev **+ crédits IA** |
| **Enterprise** | + multi-sites, SSO/RBAC, rapports signés longue durée, déclarations 12 pays, SLA, hébergement UE, Jira | Contrat annuel par portefeuille **+ conso IA au coût majoré** |
| **Partenaires** (cabinets) | Licence Team offerte, tarif revendeur sur la conso IA, certification | Revenue share ; l'outil ne vend jamais d'audit humain (PDF §7.3) |

**Refacturation IA** : prix = coût Mistral × (1 + marge), exprimé en « crédits » plutôt qu'en tokens, avec plafonds
par audit, alertes de budget et option « apportez votre clé » à 0 % de marge. Afficher le coût estimé **avant** un
audit complet. Le coût réel par audit doit être mesuré avant de fixer les prix : le repo ne contient pas ce chiffre.

## 5. Plugins et parcours guidé (le cœur de notre différence)

Tout passe par le même MCP/CLI/API ; les agents ne sont que des clients.

| Plugin | Rôle | Client |
|---|---|---|
| `audit` | Lancer / reprendre un audit, lire les résultats | Claude Code Desktop, Codex |
| `remediate` | Proposer un patch, le rejouer, re-scanner | idem (existe : `rgaa-consultant`) |
| `ai-assist` | Jugement des 32 critères, escalade `NeedsReview` | idem |
| `reporting` | Déclaration, rapport PDF, pages de grilles avec captures | idem |
| `visual-guide` | Remédiation pas à pas dans le vrai navigateur | Claude for Chrome / Codex for Chrome + computer use |

Règle déjà établie dans `docs/specs/human-criteria-automation.md` et à respecter : **le computer use enrichit la
preuve mais ne fait monter aucun test d'un niveau.** Le flux visuel guide l'humain (surbrillance de l'élément,
patch proposé, re-test, capture avant/après) ; la décision reste humaine et horodatée. Les captures « belles
pages de grilles » sont un rendu du bundle d'audit existant (hash de capture déjà produit), pas une nouvelle
source de vérité. Prévoir un MCP **HTTP authentifié** (déjà `rgaa-mcp-http`) pour que Chrome/Codex y accèdent
sans exposer le poste du client.

## 6. Version GitHub (CI des clients)

1. **Action `rgaa-action`** : télécharge le binaire signé, épingle la version (le workflow interne épingle déjà
   Obscura avec vérification SHA-256 : même discipline), lance `changed` sur les PR et `full` en planifié.
2. **Sorties** : SARIF vers *Code scanning* (permission `security-events: write`, déjà utilisée en interne),
   commentaire de PR avec le delta de taux global, check de régression (modes `diff` / `policy` existants).
3. **Remédiation** : l'agent ouvre une **PR de correctifs** (jamais de push direct), re-vérifiée par la CI ;
   un humain fusionne. C'est la version CI de « l'agent propose, l'humain décide ».
4. **Données** : mode déterministe sans appel sortant ; mode IA via clé client ou via notre passerelle mesurée.
   Documenter précisément ce qui sort du runner (le PDF fait de l'absence d'exfiltration un argument d'achat).
5. **Livraison** : action publiée sur le Marketplace, exemple de workflow dans le README, démo < 10 min
   (le critère du PDF §4.2).

## 7. Positionnement face au marché

| Acteur | Force | Notre angle |
|---|---|---|
| Deque (axe, MCP inclus) | Maturité, écosystème | RGAA natif + 12 pays + exécution locale vérifiable ; importer les configs axe plutôt que le combattre |
| Level Access, Siteimprove | Conformité cloud, reporting | Prix publics, agent-first, preuve signée |
| AudioEye, overlays | Prix bas | Correction au code source, jamais d'overlay |
| Asqatasun, WAVE, Equal Access | Gratuits | Couverture des 106 critères (eux : DOM seul) |
| Cabinets (Access42, Temesis…) | Audit humain, confiance | Partenaires, pas concurrents |

## 8. Séquence (alignée sur les phases du PDF)

- **M0-3** : multi-tenant + mesure d'usage, Mistral épinglé, `rgaa-action` v1, rapports signés, démos
  (Claude Code, Codex, GitHub).
- **M3-9** : plugin `visual-guide`, extension Chrome, grille Team publiée, 5-10 pilotes FR, 3-5 cabinets.
- **M9-18** : SSO/RBAC, multi-sites, packs DE/Benelux (les gabarits existent déjà), accords-cadres.

## 9. Risques propres à notre variante

- **Marge IA** : mesurer le coût réel par audit avant de publier les prix.
- **Dépendance Mistral** : garder l'abstraction multi-fournisseur (déjà là) et le repli local.
- **Dépendance aux clients Chrome/computer use** : le parcours visuel est un confort, pas un prérequis ; le CLI
  et le MCP doivent rester suffisants (même parade que le PDF, tableau 7).
- **Promesse juridique** : « instrument de mise en conformité », jamais « conformité garantie ».
- **Vérifications ouvertes** : identifiant et capacités de « Mistral 4 » ; conditions d'usage de Claude for
  Chrome / Codex for Chrome pour un service commercial ; régime RGPD du SaaS (sous-traitance, hébergement UE).
