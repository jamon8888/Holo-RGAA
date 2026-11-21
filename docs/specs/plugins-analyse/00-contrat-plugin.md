# P-00 — Contrat de plugin d'analyse

**Type** : socle · **Bloque** : P-01 à P-09 · **Estimation** : 1 session agent

## 1. Objectif

Définir **une seule interface** que tous les plugins d'analyse implémentent, avec :

- un registre qui les découvre ;
- un modèle de *Preuve* commun ;
- l'exposition identique dans le CLI, le MCP et l'API ;
- la gestion des modèles locaux.

## 2. Décision d'architecture : Rust d'abord, sidecar sinon

Règle de choix, appliquée par chaque spec P-01 à P-08 :

1. Une **bibliothèque Rust** mûre existe → plugin **natif** (crate du workspace).
2. Sinon, ou si la bibliothèque Rust est nettement moins bonne (langues, précision, licence de liaison) → plugin **sidecar**, c'est-à-dire un exécutable externe piloté par le protocole du §7 bis.
3. Une même capacité peut avoir les deux formes. Exemple : OCR natif `ocrs`, et sidecar `tesseract` pour le français. L'utilisateur choisit dans `rgaa.toml`, et la preuve indique la variante utilisée (`version_modele`).

| Option | Retenue ? | Raison |
|--------|-----------|--------|
| **Natif : crates du workspace + features Cargo + registre statique** | **Oui (premier choix)** | Zéro ABI à maintenir. Les dépendances lourdes (`whisper-rs`, `ort`) ne sont compilées que si la feature est active. |
| **Sidecar : processus externe, JSON lignes sur stdin/stdout** | **Oui (repli)** | Il accepte n'importe quel langage et n'importe quel binaire : `ffmpeg`, `tesseract`, `whisper.cpp` server, script Python. Son isolement est réel : un plantage ou une fuite mémoire ne tue pas l'audit, et un timeout est appliqué au processus. Il ne lie aucune bibliothèque sous GPL dans nos binaires. |
| Bibliothèques dynamiques (`dylib`) | Non | Pas d'ABI Rust stable, distribution fragile. |
| Composants WASM (`wasmtime`) | Non pour M1 | Les plugins ont besoin de l'ONNX natif et de `ffmpeg`. À rouvrir si des tiers doivent écrire des plugins. |

## 3. Nouveaux crates

```
rgaa-rs/crates/
  rgaa-analysis/            # contrat + registre + Preuve (sans dépendance lourde)
  rgaa-analysis-contrast/   # P-01   feature: plugin-contrast   (défaut)
  rgaa-analysis-colorvision # P-02   feature: plugin-colorvision (défaut)
  rgaa-analysis-images/     # P-03   feature: plugin-images
  rgaa-analysis-media/      # P-04   feature: plugin-media
  rgaa-analysis-motion/     # P-05   feature: plugin-motion
  rgaa-analysis-lang/       # P-06   feature: plugin-lang        (défaut)
  rgaa-analysis-layout/     # P-07   feature: plugin-layout      (défaut)
  rgaa-analysis-text/       # P-08   feature: plugin-text
```

Les features sont déclarées dans `rgaa-plugins`, puis relayées par `rgaa-cli`, `rgaa-mcp` et `rgaa-orchestrator`. `default` contient les plugins **sans modèle à télécharger** (contrast, colorvision, lang, layout). Une feature `plugins-all` les active tous.

## 4. Interface Rust (`rgaa-analysis`)

```rust
/// Ce qu'un plugin reçoit : tout ce qui a déjà été capturé de la page, rien de plus.
pub struct PageCapture<'a> {
    pub url: &'a str,
    pub dom_snapshot: &'a DomSnapshot,          // CDP DOMSnapshot.captureSnapshot (styles calculés inclus)
    pub ax_tree: Option<&'a AxTree>,            // CDP Accessibility.getFullAXTree
    pub screenshot: Option<&'a image::RgbaImage>, // pleine page, device pixel ratio connu
    pub dpr: f32,
    pub resources: &'a dyn ResourceFetcher,     // télécharge médias/images de la page (même origine/cookies)
    pub lang_declared: Option<&'a str>,
}

pub trait AnalysisPlugin: Send + Sync {
    /// Identifiant stable, kebab-case : "contrast", "colorvision", "images"…
    fn id(&self) -> &'static str;
    fn version(&self) -> &'static str;                 // env!("CARGO_PKG_VERSION")
    /// Tests RGAA servis, ex. ["3.2.1", "3.2.2", …] — clés du catalogue `rgaa-core`.
    fn tests(&self) -> &'static [&'static str];
    /// Modèles locaux requis (vide si aucun) — voir §6.
    fn models(&self) -> &'static [ModelSpec] { &[] }
    async fn analyze(&self, page: &PageCapture<'_>, opts: &PluginOptions)
        -> Result<Vec<Preuve>, RgaaError>;
}
```

- Le trait utilise des `async fn` natives (Rust ≥ 1.75). Le workspace demande `rust-version = "1.80"`, donc c'est possible. Le registre manipule les plugins à travers `Box<dyn AnalysisPlugin>` grâce à `async-trait` **ou** à un retour `Pin<Box<dyn Future>>`. Le choix se fait à l'implémentation ; il ne change pas le contrat.
- `RgaaError` gagne une variante `Analysis { plugin: String, message: String }`. Pas de `unwrap()` ni d'`expect()` hors tests (CLAUDE.md).
- **Registre explicite**, et non à features dans `rgaa-analysis` : un crate de plugin dépend de `rgaa-analysis` pour son contrat, et Cargo refuse la dépendance inverse, même optionnelle. Décision du 2026-11-21, livrée dans la PR #467.
  - `rgaa-analysis` expose `Registry::builder().with(Box<dyn AnalysisPlugin>)…build()`. Le registre est trié par id et refuse les id en double.
  - Un crate **agrégateur `rgaa-plugins`** dépend, en optionnel et par feature, de chaque crate de plugin. Il expose `default_registry()` : d'abord les plugins natifs activés, puis les sidecars découverts.
  - `rgaa-cli` et `rgaa-mcp` passent par `rgaa-plugins`, qui leur relaie les features `default` et `plugins-all`.
- **Calculs de couleur partagés** : la luminance relative WCAG, le ratio de contraste et ΔE2000 vivent dans `rgaa_analysis::color`, en une seule implémentation pour tous les plugins.

## 5. Modèle de *Preuve*

```rust
#[derive(Serialize, Deserialize, Clone, Debug, schemars::JsonSchema)]
pub struct Preuve {
    pub plugin: String,             // "contrast"
    pub version_plugin: String,     // "0.1.0"
    pub version_modele: Option<String>, // "ocrs-text-recognition@sha256:…"
    pub tests: Vec<String>,         // ["3.2.1"]
    pub sujet: Sujet,               // quoi : élément, média, page
    pub portee: Portee,             // Mesuree | Estimee
    pub mesures: serde_json::Value, // schéma propre au plugin, documenté dans sa spec et exporté en JSON Schema
    pub indice: Option<Indice>,     // aide au tri, JAMAIS un statut (R-P1)
    pub artefacts: Vec<Artefact>,   // vignettes, images simulées, transcriptions
    pub empreinte_entree: String,   // sha256 de l'entrée analysée (R-P4)
}

pub enum Sujet { Element { selecteur: String, xpath: String, bbox: Rect }, Media { url: String }, Page }
pub enum Portee { Mesuree, Estimee }
/// Ordonne la file de revue ; ne figure jamais dans le rapport comme verdict (R3).
pub enum Indice { Suspect { raison: String }, Neutre, Rassurant { raison: String } }
pub struct Artefact { pub nom: String, pub media_type: String, pub chemin_bundle: String, pub sha256: String }
```

- Les artefacts sont écrits dans le **bundle d'audit** existant (`rgaa-storage`), sous `analysis/<plugin>/<sha256>.<ext>`. La rétention suit le PRD §10 : bundles 3 ans, captures 90 jours. Les artefacts image comptent comme des **captures**.
- Le moteur de règles (`rgaa-orchestrator`) lit les preuves et décide du statut :
  - `Portee::Mesuree` avec un seuil franchi peut donner `fail` **uniquement** sur un test classé Automatique ;
  - tout le reste donne `review`, avec la preuve jointe.

## 6. Modèles locaux

```rust
pub struct ModelSpec { pub nom: &'static str, pub url: &'static str, pub sha256: &'static str,
                       pub taille_octets: u64, pub licence: &'static str }
```

- Commande `rgaa models list|pull <plugin>|verify`. Le cache se trouve dans `$RGAA_MODELS_DIR`, par défaut `~/.cache/rgaa/models`.
- Un plugin dont le modèle manque renvoie `RgaaError::Analysis { message: "modèle absent : lancez `rgaa models pull images`" }`. Il ne télécharge **jamais** pendant un audit (R-P3).
- Une empreinte qui ne correspond pas entraîne un refus.
- Les licences sont listées dans `rgaa models list` et dans le rapport.

## 7. Exposition

| Surface | Forme |
|---------|-------|
| CLI | `rgaa analyze <url> --plugin contrast[,images…]` ; `rgaa plugins list` (id, version, tests, modèles, présents ou non) |
| MCP (`rgaa-mcp`) | outil `list_analysis_plugins` ; outil `run_analysis_plugin { url, plugin, options }` qui renvoie `Vec<Preuve>` **sans statut** (R3) ; `audit_url` accepte `config.plugins: [..]` |
| API (`rgaa-api`) | `POST /audit` accepte `plugins` ; `GET /audit/:id/preuves?plugin=` |
| Orchestrateur | après la collecte axe + déterministe, il exécute les plugins actifs dont les `tests()` intersectent les tests à évaluer, avec 2 plugins en parallèle par page (configurable) |

## 7 bis. Protocole sidecar

**Déclaration.** Un sidecar se déclare par un manifeste `plugins/<id>/sidecar.toml`, cherché dans `$RGAA_PLUGINS_DIR` puis dans `~/.config/rgaa/plugins` :

```toml
id = "ocr-tesseract"
version = "0.1.0"
tests = ["1.3.1", "1.8.1"]
commande = ["tesseract-rgaa-sidecar"]        # ou ["python3", "-I", "sidecar.py"]
mode = "persistant"                           # "persistant" (un processus par audit) ou "par-appel"
timeout_ms = 30000
binaires_requis = [{ nom = "tesseract", version_min = "5.0", verif = ["tesseract", "--version"] }]
licence = "Apache-2.0"
```

**Échanges.** Ils passent sur stdin/stdout, avec un objet JSON par ligne (JSON-RPC 2.0, sans transport réseau) :

| Méthode | Sens | Corps |
|---------|------|-------|
| `initialiser` | rgaa → sidecar | `{ "protocole": 1, "options": {…} }` → `{ "id", "version", "tests", "version_modele" }` |
| `analyser` | rgaa → sidecar | `{ "url", "sujets": [...], "fichiers": { "capture": "/chemin/tmp/capture.png", … } }` → `{ "preuves": [Preuve…] }` |
| `journal` | sidecar → rgaa | notification `{ "niveau", "message" }`, relayée vers `tracing` |
| `arreter` | rgaa → sidecar | aucun corps |

**Règles.**

- **Fichiers.** Les fichiers lourds (captures, médias) passent par des **chemins** dans un répertoire temporaire de l'audit, jamais en base64 dans le JSON. Le sidecar n'a pas besoin du réseau : `ResourceFetcher` télécharge les ressources avant l'appel (R-P3).
- **Validation.** Chaque `Preuve` reçue est validée contre le JSON Schema de P-00 §5. Une preuve invalide est rejetée et journalisée. Un champ `status` ou `verdict` vaut rejet (R-P1).
- **Échec.** Un timeout, une sortie non nulle ou un JSON invalide met les tests concernés en `review` « preuve manquante (sidecar <id> : <raison>) ». L'audit continue.
- **Adaptateur.** Le type `SidecarPlugin` du crate `rgaa-analysis` implémente `AnalysisPlugin`. Le registre fusionne les plugins natifs et les sidecars découverts, et l'orchestrateur ne voit pas la différence.
- **Sécurité.** Seuls les manifestes situés dans les deux répertoires ci-dessus sont chargés. `rgaa plugins list` affiche la commande exacte et l'empreinte SHA-256 de l'exécutable. Une empreinte qui change entre deux audits produit un avertissement dans le rapport.
- **Kit.** Le dépôt livre un sidecar d'exemple, `examples/sidecar-echo/` (Python, 40 lignes), et un test d'intégration qui l'exécute.

## 8. Critères d'acceptation

1. `cargo build -p rgaa-cli --no-default-features` compile **sans** aucun plugin. `--features plugins-all` compile tout.
2. `rgaa plugins list` affiche les plugins compilés avec leurs tests. Un test du catalogue servi par deux plugins est signalé.
3. Un plugin factice `echo` (sous `cfg(test)`) traverse le CLI, le MCP et l'API, et sa preuve apparaît dans le bundle avec son artefact et ses empreintes.
4. Le DTO MCP de `run_analysis_plugin` ne contient aucun champ `status` ou `verdict` (test de schéma via `dump-tool-schemas`).
5. Un modèle absent donne une erreur explicite, et l'audit continue avec les tests concernés en `review` « preuve manquante ».
6. Le JSON Schema de `Preuve` et de chaque `mesures` est exporté dans `types/` (même mécanisme que les schémas MCP existants).
7. Le sidecar `examples/sidecar-echo` est découvert, initialisé et appelé. Sa preuve est identique à celle du plugin natif `echo`. Un sidecar qui dort au-delà de `timeout_ms` est tué et produit « preuve manquante ».
8. Un sidecar qui renvoie un champ `verdict` voit sa preuve rejetée (test dédié).

## 9. Hors périmètre

Plugins tiers chargés à l'exécution ; marketplace de plugins ; exécution des plugins sur l'instance OVHcloud (D4 : tout tourne côté client).
