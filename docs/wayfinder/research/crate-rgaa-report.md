# Research: nouveau crate `rgaa-report` (ticket #100, part of #97)

Question: inventorier le reporting existant dans `rgaa-rs` et trancher le design du nouveau crate `rgaa-report`, sans implémenter.
Sources primaires = code source uniquement (pas de docs secondaires). Vérifié sur `origin/master` @ `42319dc`.

## 1. Inventaire — ce qui existe aujourd'hui

### 1.1 `rgaa-core` : deux types d'audit cohabitent

| Type | Fichier | Contenu |
|---|---|---|
| `AuditResult` | `rgaa-rs/crates/rgaa-core/src/types.rs:71-84` | `audit_id, url, pages: Vec<PageResult>, total_criteria, passed, failed, na, overall_compliance, taux_global, coverage_percent, etat_conformite: String, duration_ms` |
| `PageResult` | `types.rs:62-68` | `url, title, criteria, compliance_rate, crawl_depth` |
| `CriterionResult` | `types.rs:42-51` | `criterion_id, title, classification, status, violations, confidence, justification, source` |
| `AuditBundle` | `rgaa-rs/crates/rgaa-core/src/audit_bundle.rs:55-64` | `schema_version ("1.0", cf. `CURRENT_SCHEMA_VERSION` ligne 10), audit_id, url, config, pages: Vec<PageAudit>, findings: Vec<Finding>, checkpoints: Vec<CheckpointResult>, summary: AuditSummary` |
| `Finding` | `src/findings.rs:6-31` | `id, rule, criterion_id?, url, target, evidence, status, severity?, description?, remediation?, html?, source, previously_incomplete` |
| `CheckpointResult` / `PageError` | `src/checkpoints.rs:12-18, 6-9` | `checkpoint_id, criterion_id, status, evidence, summary` |

Points structurants :

- `AuditBundle::validate()` (`audit_bundle.rs:81-151`) contrôle déjà : ids non vides, `schema_version == "1.0"` rejetée sinon (`UnsupportedSchemaVersion`), unicité globale des `finding.id` (top-level + pages), `rule/url/target` requis, `Checkpoint Pass` exige `evidence` complète. C'est le seul validateur existant — pas de JSON Schema.
- `From<AuditResult> for AuditBundle` (`audit_bundle.rs:194-259`) : dérive `PageAudit` (`page-{idx}`), explose chaque `violation` en `Finding`, agrège `summary { passed, failed, needs_review }`. Utilisé par CLI `analyze --format html` (`rgaa-cli/src/commands/analyze.rs:115-118`).
- Critères 106 : `RgaaCriteria::all()` (`rgaa-core/src/criteria.rs:129-143`, cache `OnceLock` ligne 13), classification `CLASSIFICATION` (lignes 17-124), `count()==106` testé (lignes 188-190). `RgaaCatalog` (`src/catalog.rs:85-176`, `OnceLock` + index `HashMap`) donne titres/tests/automatabilité : **39 Fully / 45 Partially / 22 Not** (test lignes 227-245).
- Taux : `CriterionStatus → ConformityStatus` (`types.rs:29-39`) : Pass→Conforme, Fail/Error→NonConforme, NA→NonApplicable, NeedsReview/NotTested→NonTeste.

### 1.2 `rgaa-orchestrator` : trois calculs de conformité divergents

Fichier unique : `rgaa-rs/crates/rgaa-orchestrator/src/pipeline.rs`.

- `calculate_compliance(&[CriterionResult])` (lignes 74-89) : `pass / (pass + fail+error)`, NA/NeedsReview/NotTested exclus. Sert `PageResult.compliance_rate` et `AuditResult.overall_compliance` (lignes 387, 763, 794).
- `calculate_compliance_summary` (lignes 91-138) : `taux_global = C/(C+NC)` via `ConformityStatus`, `coverage = validated_executed/validated_total` sur `Fully|PartiallyAutomatable` via `RgaaCatalog::by_id`, seuils `100→"totale" / ≥50→"partielle" / sinon "non conforme"`. Sert `run_crawl_and_audit` et `audit_one` (lignes 364, 764).
- `aggregate_site_compliance(&[PageResult])` (lignes 496-591, `pub`) : règle officielle RGAA — **NC site si Fail/Error sur UNE page quelconque**, `all_pass→C`, `all_na→exclu`, mixte Pass/NeedsReview/NotTested→exclu (NonTeste), critères `Manuel` sautés, mêmes seuils 100/50. Sert `run_crawl_and_audit` (ligne 364).
- `run_crawl_and_audit` (lignes 325-403) : échantillonnage RGAA 7 pages (`discover_rgaa_sample_pages`, lignes 407-491) ou crawl `SpiderTool`, `run_batch` (max 3 audits concurrents, `Semaphore` ligne 140), agrégation site-wide, `total = RgaaCriteria::count()`.

⚠️ Divergence constatée (à corriger dans `rgaa-report`, pas ici) : le HTML CLI recalcule son propre taux/couverture (`report/html.rs:227-241`, voir §1.3) avec une formule **incompatible** (`passed/(passed+failed+needs_review)` et `tested/total_findings`), et le TUI utilise des seuils **80/50** (`tui/export.rs:39-52`) contre **100/50** partout ailleurs.

### 1.3 `rgaa-cli/src/report.rs` : 5 formats, zéro dépendance template

- `render(&AuditBundle, ReportFormat)` (`rgaa-cli/src/report.rs:10-19`) : `Json` = `serde_json::to_string_pretty(bundle)` ; `Markdown`/`Sarif`/`Junit` = builders manuels ; `Html` = `html::generate_html_report(bundle)`.
- `ReportFormat` (`src/format.rs:5-17`, `ALL` ligne 21, `FromStr` lignes 35-48) : `json (défaut), markdown|md, sarif, junit|xml, html`. `pdf` explicitement rejeté (test ligne 68).
- `render_markdown` (lignes 21-67) : `writeln!` table Summary + liste findings (`all_findings` = top-level + pages, lignes 162-168).
- `render_sarif` (lignes 69-110) : SARIF 2.1.0 manuel (`$schema` schemastore, `runs[0].tool.driver.name="rgaa"`, `error/warning/note` mappés de `Fail/NeedsReview/Error/autres`).
- `render_junit` (lignes 112-160) : XML concaténé + `escape_xml` (lignes 181-188), `failure` pour Fail/NeedsReview, `error` pour Error.
- `report/html.rs:generate_html_report` (lignes 5-13) : header/summary/stats/findings/footer en `writeln!` inline, `escape_html` (lignes 253-257, pas d'échappement `'`), filtre findings Fail|NeedsReview uniquement (lignes 243-250), horodatage `chrono::Utc::now()` (ligne 223).
- Commande `audit report` (`src/commands/report.rs:44-50`) : `--input bundle.json` → parse → `bundle.validate()` → `render` → `--output` ou stdout. `--audit-id` (storage) = `unimplemented` (lignes 56-57). `analyze` (`src/commands/analyze.rs:66-76`) rend `table|json|html` depuis `AuditResult` (converti en bundle pour HTML).
- Deps CLI (`rgaa-cli/Cargo.toml`) : `serde, serde_json, serde_yaml, chrono, clap, tokio` — **aucun** moteur de template, aucun validateur schema.

### 1.4 `rgaa-tui` export : JSON/HTML/PDF via `wkhtmltopdf`

`rgaa-rs/crates/rgaa-tui/src/tui/export.rs` (155 lignes), appelé par `commands.rs:audit()` (lignes 23-26) :

- `export_json` (lignes 30-35) : sérialise **`AuditResult`** (pas `AuditBundle` — incohérent avec CLI).
- `export_html` (lignes 37-114) : `format!` inline, **première page seulement** (`pages.first()`, ligne 55), thème sombre hardcodé, seuils 80/50 divergents (voir §1.2).
- `export_pdf` (lignes 116-141) : écrit un HTML temporaire, shell-out `wkhtmltopdf --enable-local-file-access` (binaire via `RGAA_WKHTMLTOPDF` ou PATH), supprime l'intermédiaire. Échec = `Err(String)`.
- Deps TUI (`rgaa-tui/Cargo.toml`) : `ratatui, rusqlite, keyring, reqwest…` — aucune lib PDF.

## 2. Design proposé pour `rgaa-report` — décisions tranchées

### 2.1 Responsabilités (et non-responsabilités)

| Dans `rgaa-report` | Hors scope (reste où c'est) |
|---|---|
| Moteur de calcul : `taux_global, coverage, etat_conformite, compliance_page` — **source unique**, paramétré par référentiel (§2.3). Déménage `calculate_compliance*` + `aggregate_site_compliance` depuis `pipeline.rs`. | Collecte (axe/gap-fix/agent/crawl) : reste orchestrator. |
| Validateur : `AuditBundle::validate()` **déplacé tel quel** + validateur JSON Schema généré (§2.4). | Persistance : reste `rgaa-storage`. |
| Renders : `json, markdown, sarif, junit, html` **déménagés tels quels** depuis `rgaa-cli/src/report{,.rs,/html.rs}` + adaptateur `AuditResult→Bundle` ; export fichier. | Génération PDF binaire : **pas de moteur PDF dans le crate** — `rgaa-report` produit HTML imprimable + appel optionnel au navigateur (§2.6). |
| Types partagés : `ReportFormat` (depuis `format.rs`), `ReportInput` (`Bundle` ou `AuditResult`), erreurs `ReportError` (thiserror). | CLI/TUI : deviennent coquilles fines qui appellent `rgaa-report` (§2.6). |

API publique cible (minimale) :

```rust
pub enum ReportFormat { Json, Markdown, Sarif, Junit, Html }
pub enum ReportInput<'a> { Bundle(&'a AuditBundle), Result(&'a AuditResult) } // Result auto-converti via From
pub struct Referentiel { /* seuils + règles d'agrégation, cf. §2.3 */ }
pub fn compute_metrics(pages: &[PageResult], referentiel: &Referentiel) -> SiteMetrics;
pub fn validate(bundle: &AuditBundle) -> Result<(), RgaaError>; // ré-export core
pub fn validate_schema(json: &serde_json::Value) -> Result<(), SchemaError>;
pub fn render(input: ReportInput, format: ReportFormat, referentiel: &Referentiel) -> Result<String, ReportError>;
```

`Orchestrator` garde ses signatures (`run/run_batch/run_crawl_and_audit → AuditResult`) ; le calcul interne délègue à `rgaa-report::compute_metrics(RGAA_41)`.

### 2.2 Dépendances : zéro nouvelle dépendance

`rgaa-report = { rgaa-core, serde, serde_json, thiserror, chrono }` — toutes déjà dans le workspace (`Cargo.toml` racine + `rgaa-cli/Cargo.toml`). Pas d'Askama/Tera/jsonschema en phase 1 (justifié §2.4-2.5). `edition = "2021"`, `rust-version = "1.80"` comme les autres crates.

### 2.3 Moteur de calcul paramétré par référentiel

Constat : la logique métier est tripliquée et divergente (§1.2 + HTML CLI + TUI). Trancher :

- **Option A (retenue) : `Referentiel` struct** `{ id: "rgaa-4.1.2", seuils: (100.0, 50.0), skip_manuel: true, nc_si_un_fail: true, coverage_source: Automatable }`. `RGAA_41: Referentiel` en constante. `compute_metrics` implémente `aggregate_site_compliance` tel quel, `calculate_compliance_summary` devient le cas `pages.len()==1`. Les seuils TUI 80/50 deviennent `Referentiel { id: "display-tui", … }` ou sont supprimés (recommandé : supprimer, unifier sur 100/50 officiels).
- Option B (écartée) : trait `Referentiel` + generics. Rejetée : un seul référentiel réel (RGAA 4.1.2), pas de second implémenteur — abstraction spéculative (YAGNI).
- Option C (écartée) : garder le calcul dans l'orchestrator et dupliquer côté report. Rejetée : c'est le bug actuel.

### 2.4 Validateur JSON Schema

- **Option A (retenue) : `schemars` (déjà dans le workspace via `rmcp`, `Cargo.toml` racine ligne 34) pour dériver le schéma depuis `AuditBundle`, + tests `assert!(schema.validate(bundle))`.** Pas de validation runtime lourde : `validate()` existant reste la source de vérité ; le schéma sert l'interop (CLI `report --input`, CI).
- Option B (écartée) : crate `jsonschema` en dépendance runtime. Rejetée : nouvelle dépendance, coût de maintenance, besoin non démontré (un seul producteur/consommateur aujourd'hui).
- Option C (écartée) : schéma `.json` maintenu à la main. Rejetée : dérive garantie avec `schemars`.

### 2.5 Moteur de templates : ni Askama ni Tera en phase 1

| Option | + | − | Verdict |
|---|---|---|---|
| **A. `write!/format!` actuels, déplacés tels quels** | 0 nouvelle dép, code déjà testé (`report.rs` tests lignes 191-244, `html.rs` tests 259-320), diff minimal | templates dans le code, `escape_html` à compléter (`'` manquant côté HTML) | **Retenue (phase 1)** |
| B. Askama (compile-time, typé) | erreurs au build, échappement auto | nouvelle dép, templates à extraire, gain nul à 5 formats stables | Phase 2 si HTML/Markdown grossissent |
| C. Tera (runtime, dynamique) | thèmes utilisateur | nouvelle dép, erreurs au runtime, perf −, overkill | Écartée |
| D. `maud`/`horrorshow` | DSL typé | nouvelle dép, réécriture complète | Écartée |

Règle de bascule : passer à Askama **quand** un 6ᵉ format ou un thème client l'exige, pas avant.

### 2.6 Branchement orchestrator + CLI/TUI, sortie PDF/UA

- `rgaa-orchestrator/Cargo.toml` += `rgaa-report` (path). `pipeline.rs` : `calculate_*`/`aggregate_site_compliance` deviennent thin wrappers vers `rgaa-report::compute_metrics` (garder `aggregate_site_compliance` `pub` en deprecated une version pour les appelants externes).
- `rgaa-cli` : `report.rs + report/html.rs + format.rs` → déplacés dans `rgaa-report` (même code, `CliError` → `ReportError`). `commands/report.rs:load_from_file` appelle `rgaa_report::validate_schema` avant `validate()`. `commands/analyze.rs:render_output` délègue à `rgaa_report::render`. Effet : `analyze --format html` et `audit report` partagent enfin le même moteur.
- `rgaa-tui` : `tui/export.rs` devient `export_json/html → rgaa_report::render`, multi-pages gratuit (corrige le bug `pages.first()`).
- **Remplacement `wkhtmltopdf`** (aujourd'hui `export.rs:116-141`) :
  - **Retenue : `Page.printToPDF` CDP via le navigateur déjà piloté (`ObscuraBridge`, `rgaa-obscura/src/lib.rs`) ou `chromium --headless --print-to-pdf`.** Motifs : `wkhtmltopdf` = Qt WebKit figé, CSS moderne cassé, **pas de PDF taggé/UA** ; Chromium = moteur réel + `--tagged-pdf` / en-têtes/pieds + `@media print`. `rgaa-report` fournit `print.css` + HTML sémantique (h1/table/thead) ; le PDF reste une étape de sortie, pas un format du crate.
  - Écartées : `genpdf/lopdf` (pur Rust mais mise en page manuelle, pas de HTML→PDF, pas d'UA), `weasyprint` (binaire Python externe, même classe de problème que wkhtmltopdf), `typst` (nouveau langage, réécriture).
  - PDF/UA : aucun outil HTML→PDF open-source ne garantit seul l'UA ; la voie réaliste = HTML accessible + `printToPDF` Chromium taggé + vérification (ex. veraPDF/PAC) hors crate.

## 3. Plan de découpe suggéré (pour le ticket d'implémentation, pas cette recherche)

1. Créer `crates/rgaa-report` (deps §2.2) + déplacer `format.rs`, `report.rs`, `report/html.rs` (renommer erreurs).
2. Déplacer `calculate_compliance*` + `aggregate_site_compliance` en `compute_metrics(Referentiel)` ; orchestrator délègue ; unifier seuils (supprimer 80/50 TUI).
3. `#[derive(JsonSchema)]` via `schemars` sur `AuditBundle` et cie ; test golden du schéma.
4. CLI/TUI branchés sur `rgaa-report` ; TUI multi-pages ; `wkhtmltopdf` → fonction `export_pdf_via_browser` (ou supprimée au profit de HTML print).
5. Corriger au passage : `escape_html` (`'`), `write_html_stats` ligne 146 (`passed+failed+needs_review` affiché comme "Not Applicable" — suspecte), `From<AuditResult>` qui `clone()` les findings page (ligne 216).

## Sources (toutes vérifiées par lecture directe)

- `rgaa-rs/crates/rgaa-core/src/types.rs` (AuditResult, PageResult, statuts)
- `rgaa-rs/crates/rgaa-core/src/audit_bundle.rs` (validate, From, schema 1.0)
- `rgaa-rs/crates/rgaa-core/src/criteria.rs` (106, OnceLock), `src/catalog.rs` (39/45/22), `src/findings.rs`, `src/checkpoints.rs`
- `rgaa-rs/crates/rgaa-orchestrator/src/pipeline.rs` (3 calculs, run_batch/crawl, seuils 100/50)
- `rgaa-rs/crates/rgaa-cli/src/report.rs`, `src/report/html.rs`, `src/format.rs`, `src/commands/report.rs`, `src/commands/analyze.rs`, `rgaa-cli/Cargo.toml`
- `rgaa-rs/crates/rgaa-tui/src/tui/export.rs` (wkhtmltopdf, 80/50, first-page), `src/commands.rs`, `rgaa-tui/Cargo.toml`
- `rgaa-rs/Cargo.toml` (workspace deps : schemars présent, aucun moteur template)
