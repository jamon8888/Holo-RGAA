# P0 et Mistral Large 4 — analyse précise du code

> Complète `offre-europe-saas-plugins.md`. Date : 2026-11-20.
> Légende : **[code]** lu dans le repo (chemin:ligne approximatif) · **[doc Mistral]** page officielle
> consultée le 2026-11-20 · **[à tester]** déduction non vérifiée en exécution · **[non trouvé]**.
> Je n'ai exécuté aucun test pour ce document : les défauts « [code] » sont établis par lecture, pas par reproduction.

## A. Mistral Large 4 — faits vérifiés

| Point | Valeur | Source |
|---|---|---|
| Identifiants API | `mistral-large-4`, `mistral-large-4-0`, `mistral-large-latest` | doc Mistral, page modèle |
| Statut | GA ; Public Preview le 2026-10-06, page datée du 2026-11-05 | changelog + page modèle |
| Contexte | « 1M » sur la fiche ; des sources tierces citent 524 288 tokens (guide API) et 512K (endpoint UE OpenRouter) | **conflit**, à mesurer |
| Sortie max | non listée | **[non trouvé]** |
| Entrées | texte + image (encodeur vision 1,6 B) ; nombre d'images, formats, taille max non listés | **[non trouvé]** |
| Fonctions | structured-outputs, function-calling, document-qna, batching, … | page modèle |
| Prix /1M tokens | entrée 0,7 $ · entrée en cache 0,07 $ · sortie 2,1 $ (bascule USD/EUR sur la page) ; le preview avait « -50 % pendant 2 semaines » ; la page ne dit pas si 0,7/2,1 est le tarif post-lancement | page modèle + changelog |
| `reasoning_effort` | `low` \| `medium` \| `high` ; **`none` refusé (HTTP 400)** ; **défaut `high` si omis** ; le modèle « raisonne toujours » | changelog 2026-10-29 + page reasoning |
| Format de réponse | avec raisonnement, `message.content` est une **liste de blocs** (`thinking`, `text`), plus une chaîne | page reasoning |
| Paramètres inconnus | l'API « applique strictement » la validité des paramètres : « Extra inputs are not permitted » (changelog 2024-01-11, général, pas propre à Large 4) | changelog |
| `response_format: json_schema` | forme brute (`name` / `schema` / `strict`) **non trouvée** ; l'exemple officiel passe par le SDK (`chat.parse` + Pydantic) ; `strict` jamais mentionné | **[non trouvé]** |
| Limites de débit | non listées sur ces pages | **[non trouvé]** |
| Retrait / dépréciation | aucune date pour Large 4 | page modèle |
| Licence des poids | Mistral Ecosystem Open Weight License (≠ Apache-2.0 : à lire avant tout auto-hébergement commercial) | page modèle |

Conséquence de prix : à 0,7 / 2,1 $, un appel de jugement visuel de 6 000 tokens en entrée et 1 500 en sortie
(**raisonnement inclus**, puisqu'il est facturé en sortie) coûte environ 0,0042 $ + 0,0032 $ = **≈ 0,0074 $**.
Ce chiffre est une illustration arithmétique avec des tailles *supposées* ; le repo ne contient aucune mesure
réelle de tokens par audit (voir B.2).

## B. Ce que le code fait aujourd'hui avec Mistral — défauts précis

Deux chemins LLM coexistent : le transport brut `rgaa-holo` (`transport.rs`) et l'agent `rig` (`rgaa-agent/src/agent.rs:345`,
client `openai::Client … .completions_api()`). Les deux lisent `LlmSettings` (`rgaa-core/src/provider.rs`).

**B.1 — `mistral` existe mais est inutilisable tel quel** [code]
`provider.rs:95-101` : `base_url: https://api.mistral.ai/v1`, `key_var: MISTRAL_API_KEY`, `default_model: None`,
`requires_key: true`. Sans `RGAA_LLM_MODEL`, `from_env` renvoie « RGAA_LLM_MODEL is not set (provider `mistral` has no
default model) ». Le provider par défaut reste `myia` (`provider.rs`, branche `unwrap_or_else(… "myia")`).

**B.2 — Le transport brut ne lit pas l'usage en tokens** [code]
`transport.rs:56-69` : `ChatEnvelope { choices }` — aucun champ `usage`. `complete()` renvoie un `HoloResponse`
(verdict, confidence, justification) sans compteur. Seul le chemin `rig` mesure (`rgaa-agent/src/metrics.rs`,
`CallMetrics`), et uniquement par un événement `tracing` : rien n'est stocké ni exposé. **Sans cela, on ne peut ni
facturer ni même connaître le coût d'un audit.**

**B.3 — Contenu en liste de blocs : échec probable à 100 %** [code + doc, **à tester**]
`ChatChoiceMessage.content: Option<String>` (`transport.rs:66-69`). Avec Large 4, `content` est une liste ;
la désérialisation de l'enveloppe échoue, `extract_json` retombe sur `extract_from_content(text)` appliqué au corps
HTTP brut. Les guillemets y sont échappés (`\"verdict\"`) : le parse direct, le bloc ``` et la regex
(`"verdict"…"confidence"…"justification"`) échouent tous. Résultat attendu : `JSON_PARSE_ERROR`, 5 tentatives (voir la
constante `MAX_RETRIES`), puis `CIRCUIT_BREAKER_OPEN`. Le chemin `rig` : comportement face au bloc `thinking`
**inconnu** [à tester]. *Premier travail : un test reproduisant ce corps de réponse.*

**B.4 — `reasoning_effort` absent ; le défaut `high` coûte cher** [code + doc]
`CompletionParams` (`rgaa-core/src/completion.rs`) = `temperature`, `max_tokens` (défaut 4096), `enable_thinking`,
`response_format`. Aucun `reasoning_effort`. Le modèle raisonne donc à `high` pour **chaque** critère, et ces tokens sont
facturés en sortie et comptés dans `max_tokens`. Le commentaire du fichier décrit déjà le risque : verdict JSON tronqué
→ disjoncteur.

**B.5 — Paramètres « thinking » rejetés par Mistral** [code + doc, **à tester**]
`completion.rs` `thinking_body()` envoie `chat_template_kwargs`, `think` et `enable_thinking` dès que
`RGAA_LLM_ENABLE_THINKING` vaut `true`/`false`. Mistral applique « Extra inputs are not permitted » (changelog 2024) :
400 attendu. En `auto` sur un fournisseur distant rien n'est envoyé (défaut sûr, `completion.rs` test
`defaults_are_the_single_source_of_truth`).

**B.6 — `json_schema` strict : forme et support non établis** [code + non trouvé]
`ResponseFormat::JsonSchema` envoie `{"type":"json_schema","json_schema":{"name","strict":true,"schema"}}`.
Je n'ai pas trouvé la forme brute attendue par Mistral ni le support de `strict`. Ne pas activer sans test réel ;
`json_object` + validation locale par `verdict_schema()` est l'option sûre.

**B.7 — Image** [code]
`transport.rs:~188` envoie `{"type":"image_url","image_url":{"url":"data:image/png;base64,…"}}` : valide la base64,
mais aucune limite de taille ni de dimension n'est vérifiée (limites Mistral **[non trouvé]**).

**B.8 — Aucune tarification par modèle** [code]
Rien dans le repo ne mappe `modèle → prix`. Les tiers `RGAA_LLM_MODEL_TACTICAL` / `_REASONING` existent
(`provider.rs`), bien adaptés à : petit modèle pour le tri, `mistral-large-4` pour le jugement visuel. Le nom du petit
modèle à utiliser n'a pas été vérifié ici.

### Chantier P0-M (Mistral Large 4) — critères d'acceptation

1. Test de non-régression : une réponse Large 4 avec `content` en liste (`thinking` + `text`) est parsée ; le bloc
   `thinking` n'est **jamais** interprété comme verdict.
2. `CompletionParams.reasoning_effort: Option<ReasoningEffort>` (`low|medium|high`), variable `RGAA_LLM_REASONING_EFFORT`,
   rejet explicite de `none` au chargement de la config pour `mistral`, valeur recommandée par tiers
   (tactical `low`, reasoning `medium`/`high` à calibrer sur le corpus). Apparaît dans `LlmProvenance`.
3. `max_tokens` relevé pour Large 4 (raisonnement inclus) et mesuré : taux de troncature = 0 sur le corpus.
4. `default_model` du preset `mistral` = `mistral-large-4` (identifiant exact, pas `-latest`, pour la reproductibilité
   des verdicts) ; `RGAA_LLM_ENABLE_THINKING` refusé pour `mistral` avec un message clair.
5. `usage` lu dans les deux chemins (B.2) et propagé jusqu'à l'audit (voir C.1).
6. Test réel unique, derrière un drapeau d'environnement (la règle `RGAA_E2E_REQUIRED` de `CLAUDE.md` s'applique) :
   un critère avec image PNG, `json_object` puis `json_schema`, pour trancher B.6 et B.7 avec des faits.
7. Calibration : relancer le protocole existant (`docs/specs/calibration-protocol.md`, `tools/calibration/candidates.toml`
   cite déjà Mistral) avec Large 4 avant toute promesse de qualité.

## C. P0 — spécifications précises

### C.1 Multi-tenant, clés et mesure de consommation

État [code] :
- `api_keys` (`001_initial_schema.sql`) : `id, key_hash, name, scopes, expires_at, created_at, last_used_at`. **Pas
  d'organisation.** Hash SHA-256 non salé (`repository.rs:419`) : acceptable pour une clé aléatoire à forte entropie
  (UUID v4 = 122 bits, `create_api_key`), à ne pas réutiliser pour des secrets choisis par un humain.
- `audits`, `findings`, `audit_logs`, `audit_bundle_versions`, `audit_batches` : **aucune colonne propriétaire**. Tout
  le monde voit tout, une fois authentifié.
- `auth_middleware` (`rgaa-api/src/routes.rs:211-230`) valide la clé et le scope `audit:write` puis **jette la ligne
  de clé** : les handlers ne savent pas qui appelle.
- `list_audit_bundles` / `get_audit_bundle` / `delete_audit_bundle` n'ont aucun filtre d'appartenance.
- **Trou de sécurité existant** : `/audit` et `/audit/:id` (`lib.rs`, `legacy_routes`) sont montés **sans**
  `auth_middleware` — n'importe qui peut lancer un audit (donc des appels LLM facturés) et lire un résultat.
  CORS `Any` sur toutes les routes. À corriger **avant** toute mise en ligne, indépendamment de l'offre.
- MCP HTTP : un seul jeton partagé `RGAA_MCP_TOKEN` (`rgaa-mcp-http/src/lib.rs:70-76`), liste d'origines CORS ;
  pas de notion d'organisation. Une extension navigateur envoie une `Origin` `chrome-extension://…` : elle doit être
  ajoutée à l'allowlist, sinon refus (`Denied::Origin`).
- Aucune migration, aucun crate de licence : `rgaa-license`, Stripe et facturation n'existent que dans
  `docs/superpowers/specs/2026-08-24-rgaa-distribution-design.md`.

À construire :
1. **Migration `003_tenancy.sql`** : `organizations(id, name, plan, created_at)`, `api_keys.org_id NOT NULL`,
   `audits.org_id`, `findings.org_id`, `audit_batches.org_id` + index ; jamais de requête sans `org_id`.
2. **Extension de requête** : le middleware injecte `AuthContext { org_id, key_id, scopes }` ; chaque handler et
   chaque méthode `Storage` prend `org_id`. Test d'isolation : l'organisation A reçoit 404 sur un audit de B.
3. **Table `llm_usage`** : `(id, org_id, audit_id, criterion_id, provider, model, reasoning_effort, input_tokens,
   cached_tokens, output_tokens, latency_ms, status, created_at)`. Une ligne par appel de modèle, écrite aussi sur les
   échecs (un appel échoué peut avoir été facturé par le fournisseur — à confirmer côté Mistral).
4. **Table `price_book`** (versionnée) : `(provider, model, input_per_mtok, cached_per_mtok, output_per_mtok,
   currency, valid_from)` — amorcée avec 0,7 / 0,07 / 2,1 pour `mistral-large-4` et **relue chaque mois** (le
   changelog montre des promotions et un changement de comportement en 3 semaines).
5. **Quotas** : budget par audit et par organisation, vérifié **avant** l'appel ; dépassement → verdict
   `NeedsReview` avec cause `budget` (jamais de verdict inventé).
6. **Auth MCP** : remplacer le jeton unique par la même table `api_keys` (réutiliser `validate_api_key`), avec
   `org_id` dans le contexte des outils MCP.
7. Retirer les routes `/audit*` non authentifiées ou les placer derrière le même middleware.

Hors périmètre ici : paiement (Stripe), SSO. La mesure d'usage doit exister **avant** la facturation.

### C.2 GitHub Action réutilisable

État [code] :
- `.github/workflows/rgaa-audit.yml` : `workflow_dispatch` seulement, modes `changed|smoke|full|diff|policy`,
  construit depuis les sources, télécharge Obscura 0.2.4 avec SHA-256 vérifié. C'est un workflow **interne**.
- `release.yml` publie `rgaa`, `rgaa-cli`, `rgaa-api`, `rgaa-mcp`, `rgaa-mcp-http` (linux/macOS/Windows) avec
  somme de contrôle ; aucune signature, aucune SBOM, aucune attestation (rien trouvé dans le fichier).
- `rgaa-linter` n'a **pas de binaire** : il n'est consommé que par `rgaa-mcp` (outil `lint_static`). La spec
  `2026-08-24-rgaa-linter-design.md` prévoit `rgaa-lint --format github` et `action.yml` : rien n'est livré.
- `rgaa audit report --format sarif|junit` existe (`rgaa-cli/src/commands/report.rs`) ; `audit policy` aussi.
- Aucun `action.yml` suivi dans git.

À construire :
1. **Binaire `rgaa-lint`** (ou sous-commande `rgaa lint`) autour de `rgaa-linter`, sorties `github` (annotations),
   `sarif`, `json` ; code de sortie CI. Aucun réseau, aucun LLM : c'est l'offre gratuite.
2. **`action.yml`** (composite) : télécharge la release **par version épinglée** + vérifie la somme de contrôle (même
   discipline que pour Obscura), exécute `rgaa-lint` sur les fichiers modifiés de la PR, téléverse le SARIF
   (`github/codeql-action/upload-sarif`), commente le delta.
3. **Mode audit d'URL** (preview déployée) : exige Obscura ; réutiliser la logique d'installation vérifiée de
   `rgaa-audit.yml`. L'IA reste optionnelle et désactivée par défaut.
4. **Permissions minimales** : `contents: read`, `security-events: write`, `pull-requests: write` seulement si
   commentaire. Ne jamais utiliser `pull_request_target` avec du code de la PR ; les PR de forks n'ont pas accès aux
   secrets : prévoir un mode dégradé (déterministe) plutôt qu'un échec.
5. **Remédiation** : étape séparée qui crée une **branche + PR** ; nécessite un jeton à portée `contents: write` et
   `pull-requests: write` fourni par le client, et la re-vérification de `rgaa-remediation` avant ouverture.
6. **Signature des artefacts de release** (cosign ou minisign) + SBOM CycloneDX : prérequis pour que l'action
   vérifie ce qu'elle télécharge ; sans cela, l'argument « chaîne d'approvisionnement » du PDF (§4.1) n'est pas tenu.

### C.3 Rapports signés

État [code] : ed25519 n'apparaît que dans la spec de distribution (exemple de JSON `"signature": "ed25519:…"`) ;
aucune dépendance ni code de signature dans `rgaa-rs/crates`. Les bundles ont déjà un hash
(`audit_bundle_versions.bundle_hash`) et les webhooks sont signés (`rgaa-api/src/webhook.rs`) : le mécanisme de
hash existe, pas la signature de document.

À construire : signature détachée ed25519 sur le **JSON canonique** du bundle (la forme canonique doit être spécifiée,
sinon la vérification dépend de l'ordre des clés) ; clé par organisation, rotation, clé publique publiée ;
sous-commande `rgaa audit verify` ; l'horodatage doit venir d'un tiers si l'on veut une preuve « opposable dans le
temps » (une signature seule ne prouve pas la date — point que le PDF ne traite pas).

## D. Ordre de réalisation recommandé

1. **B.3 + B.4 + B.5** (Mistral Large 4 utilisable) — petit, bloque tout le reste côté IA.
2. **C.1 point 7** (fermer `/audit*`) — correctif de sécurité, indépendant.
3. **B.2 + C.1 points 1-4** (tenancy + usage) — base de la refacturation.
4. **C.2 points 1-2** (`rgaa-lint` + action déterministe) — entrée gratuite, sans dépendance IA.
5. **B.6/B.7** (test réel Mistral), **C.1 points 5-6**, **C.2 points 3-5**, **C.3**.

## E. Non vérifié

Limites de débit et sortie max de Large 4 ; limites d'images ; forme brute de `json_schema` ; comportement du chemin
`rig` face aux blocs `thinking` ; conflit 1M / 512K sur le contexte ; si les appels échoués sont facturés ;
le coût réel d'un audit ; conditions d'usage de Claude for Chrome / Codex for Chrome en service commercial.

## Sources

- [Mistral Large 4 — page modèle](https://docs.mistral.ai/models/mistral-large-4)
- [Changelog Mistral](https://docs.mistral.ai/resources/changelogs)
- [Reasoning — Mistral](https://docs.mistral.ai/inference/conversations/reasoning)
- [Structured Outputs custom — Mistral](https://docs.mistral.ai/inference/conversations/structured-output/custom)
- [Mistral-Large-4-1T-A52B (poids)](https://huggingface.co/mistralai/Mistral-Large-4-1T-A52B)
