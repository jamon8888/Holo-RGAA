//! 9.2 — repeated visual zones compared with AX landmarks (spec §3).

use crate::mesures::{Measurement, Mesures, Repere, ZoneRepetee};
use crate::snapshot::{Node, Snapshot};
use rgaa_analysis::{AxTree, Portee, Rect};

fn overlaps(a: Rect, b: Rect) -> bool {
    a.x < b.x + b.width && b.x < a.x + a.width && a.y < b.y + b.height && b.y < a.y + a.height
}

fn is_landmark_role(role: Option<&str>) -> bool {
    matches!(
        role,
        Some("banner") | Some("navigation") | Some("main") | Some("contentinfo")
    )
}

/// Expected landmark from the vertical position of a repeated zone.
#[must_use]
pub fn expected_role(bbox: Rect, min_y: f64, max_y: f64) -> &'static str {
    let span = (max_y - min_y).max(1.0);
    if bbox.y - min_y <= span / 4.0 {
        "banner"
    } else if bbox.y - min_y >= span * 3.0 / 4.0 {
        "contentinfo"
    } else {
        "navigation"
    }
}

/// 9.2 measurement. Zones repeated across pages come from the collector; the
/// exposed landmarks come from both the DOM roles and the AX tree.
#[must_use]
pub fn analyze(snapshot: &Snapshot, ax_tree: Option<&AxTree>) -> Measurement {
    let boxes: Vec<Rect> = snapshot
        .nodes
        .iter()
        .filter_map(|node| node.bbox.filter(|_| node.is_visible() && node.has_text()))
        .collect();
    let min_y = boxes.iter().map(|b| b.y).fold(f64::INFINITY, f64::min);
    let max_y = boxes.iter().map(|b| b.y).fold(f64::NEG_INFINITY, f64::max);
    let (min_y, max_y) = if min_y.is_finite() {
        (min_y, max_y)
    } else {
        (0.0, 0.0)
    };

    let landmarks: Vec<(usize, &Node)> = snapshot
        .nodes
        .iter()
        .enumerate()
        .filter(|(_, node)| node.is_landmark())
        .collect();

    let mut zones_repetees = Vec::new();
    for (index, node) in snapshot.nodes.iter().enumerate() {
        let (Some(bbox), true) = (node.bbox, node.repeat_count >= 2) else {
            continue;
        };
        if !node.is_visible() || !node.has_text() {
            continue;
        }
        let ax_landmark = snapshot
            .ax
            .iter()
            .any(|ax| ax.node == index && is_landmark_role(Some(&ax.role)));
        let covered = landmarks.iter().any(|(_, landmark)| {
            landmark
                .bbox
                .is_some_and(|landmark_box| overlaps(bbox, landmark_box))
        });
        let repere = node.is_landmark()
            || ax_landmark
            || covered
            || ax_tree.is_some_and(|tree| {
                tree.nodes.iter().any(|ax_node| {
                    is_landmark_role(Some(&ax_node.role)) && ax_node.name.trim() == node.text.trim()
                })
            });
        zones_repetees.push(ZoneRepetee {
            selecteur: node.selecteur.clone(),
            occurrences: node.repeat_count,
            bbox: Some(bbox),
            repere,
            role_attendu: expected_role(bbox, min_y, max_y).to_owned(),
        });
    }

    let reperes = landmarks
        .iter()
        .map(|(_, node)| Repere {
            role: node.role.clone().unwrap_or_default(),
            selecteur: node.selecteur.clone(),
        })
        .collect();
    let sans_repere = zones_repetees
        .iter()
        .filter(|zone| !zone.repere)
        .map(|zone| zone.selecteur.clone())
        .collect::<Vec<_>>();

    let mesure = Measurement::new(
        Mesures::Landmarks {
            zones_repetees,
            reperes,
            sans_repere: sans_repere.clone(),
        },
        Portee::Mesuree,
        &["9.2.1"],
    );
    if sans_repere.is_empty() {
        mesure
    } else {
        mesure.suspect("zone visuelle répétée sans repère AX")
    }
}
