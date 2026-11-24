//! 5.6 visual table headers and 5.3 presentation-table linearisation.

use crate::mesures::{CelluleEnTete, Measurement, Mesures};
use crate::order;
use crate::snapshot::Snapshot;
use rgaa_analysis::color::{delta_e_2000, parse_color};
use rgaa_analysis::Portee;

/// 5.6 — a `td` of the first row or column styled like a header, without `th`
/// or `scope`. The colour distance uses the canonical CIEDE2000 of P-00.
#[must_use]
pub fn header_measurements(snapshot: &Snapshot) -> Vec<Measurement> {
    let mut measurements = Vec::new();
    for table in &snapshot.tables {
        if table.presentation {
            continue;
        }
        let body = table.body_background.as_deref().and_then(parse_color);
        let mut candidats = Vec::new();
        for (row, cells) in table.rows.iter().enumerate() {
            for (column, cell) in cells.iter().enumerate() {
                if row != 0 && column != 0 {
                    continue;
                }
                if cell.tag.eq_ignore_ascii_case("th") || cell.scope.is_some() {
                    continue;
                }
                let Some(node) = snapshot.nodes.get(cell.node) else {
                    continue;
                };
                let graisse = node.weight();
                let delta_e = match (body, node.background_color.as_deref().and_then(parse_color)) {
                    (Some(body), Some(cell_color)) => delta_e_2000(body, cell_color),
                    _ => 0.0,
                };
                let lourd = graisse >= 600;
                let colore = delta_e > 10.0;
                if !lourd && !colore {
                    continue;
                }
                let raison = match (lourd, colore) {
                    (true, true) => "graisse et fond d'en-tête",
                    (true, false) => "graisse d'en-tête",
                    (false, true) => "fond d'en-tête (ΔE > 10)",
                    (false, false) => "",
                };
                candidats.push(CelluleEnTete {
                    selecteur: node.selecteur.clone(),
                    ligne: row,
                    colonne: column,
                    graisse,
                    delta_e,
                    raison: raison.to_owned(),
                });
            }
        }
        if candidats.is_empty() {
            continue;
        }
        let mesure = Measurement::new(
            Mesures::TableHeaders {
                selecteur: table.selecteur.clone(),
                candidats,
            },
            Portee::Mesuree,
            &["5.6.1", "5.6.2"],
        );
        measurements.push(mesure.suspect("style d'en-tête sans th ni scope"));
    }
    measurements
}

/// 5.3 — a `role=presentation` table must remain understandable when the DOM
/// is linearised: compare cell order in the DOM with their visual order.
#[must_use]
pub fn linearisation_measurements(snapshot: &Snapshot, rtl: bool) -> Vec<Measurement> {
    let visual = order::visual_order(&snapshot.nodes, rtl);
    let mut measurements = Vec::new();
    for table in &snapshot.tables {
        if !table.presentation {
            continue;
        }
        let dom = table
            .rows
            .iter()
            .flatten()
            .map(|cell| cell.node)
            .filter(|index| {
                snapshot
                    .nodes
                    .get(*index)
                    .is_some_and(|node| node.bbox.is_some() && node.has_text() && node.is_visible())
            })
            .collect::<Vec<_>>();
        let visuel = visual
            .iter()
            .copied()
            .filter(|index| dom.contains(index))
            .collect::<Vec<_>>();
        let (tau, inversions) = order::kendall_tau(&dom, &visuel);
        let selecteur_of = |index: &usize| {
            snapshot
                .nodes
                .get(*index)
                .map_or_else(String::new, |node| node.selecteur.clone())
        };
        let mesure = Measurement::new(
            Mesures::Linearisation {
                selecteur: table.selecteur.clone(),
                tau,
                inversions,
                ordre_dom: dom.iter().map(selecteur_of).collect(),
                ordre_visuel: visuel.iter().map(selecteur_of).collect(),
            },
            Portee::Mesuree,
            &["5.3.1"],
        );
        measurements.push(if inversions > 0 {
            mesure.suspect("ordre de linéarisation inversé")
        } else {
            mesure
        });
    }
    measurements
}
