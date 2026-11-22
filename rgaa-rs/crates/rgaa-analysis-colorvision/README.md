# rgaa-analysis-colorvision (P-02)

Plugin d'analyse natif pour le critère RGAA 3.1 (et appui à 3.3 / 10.9). Il
produit des **preuves** — mesures et vignettes — jamais de statut (R-P1).

## Contrat d'entrée

`AnalysisPlugin::analyze` reçoit un `PageCapture` (P-00). Le `dom_snapshot` est
un JSON ; le plugin accepte :

- `elements[]` : forme normalisée, décrite ci-dessous ;
- `documents[].nodes[]` : nœuds CDP `DOMSnapshot.captureSnapshot` (les styles
  calculés doivent être aplatis dans `styles`).

Chaque élément peut porter `tag`, `selecteur`/`selector`, `xpath`, `bbox`,
`styles`, `parent_styles`, `text`, `parent_text`, `icon`, `error_message`,
`series`. Les champs absents sont ignorés : un instantané partiel donne zéro
candidat, jamais une erreur ni une panique. Seul un JSON invalide est une
`RgaaError::Analysis`.

## Formule WCAG unique

`rgaa_analysis::color::contrast_ratio` est la seule formule de contraste WCAG
2.1 du dépôt (PR #468, module canonique partagé avec P-01). Ce crate n'a plus
aucune copie locale des formules de couleur : seules les opérations propres au
daltonisme restent ici. La migration de `rgaa-obscura/src/contrast.rs` (PR #456,
branche `116zoo/wf-contrast-371`) est signalée dans la PR P-02, à faire quand
#456 sera fusionnée.

## Enregistrement

Ce crate livre `ColorVisionPlugin` / `COLORVISION_PLUGIN` et la feature
`plugin-colorvision`. Il ne modifie pas `rgaa-analysis::registry()` : le
registre global est porté par le crate agrégateur `rgaa-plugins` (P-06), qui
ajoutera ce plugin à son `default_registry()` après sa PR.

## Export des matrices

```bash
(cd rgaa-rs && cargo run -p rgaa-analysis-colorvision --bin dump-colorvision-matrices) \
  > types/colorvision-matrices.json
(cd rgaa-rs && cargo run -p rgaa-analysis-colorvision --bin dump-colorvision-schemas)
```

Le fichier JSON est la source unique des coefficients utilisés par la page de
revue P-09 (filtres CSS). Il est vérifié par un test contre les constantes Rust.
