//! 9.1 — visual headings compared with exposed headings (spec §3).

use crate::mesures::{Measurement, Mesures, TitreExpose, TitreVisuel};
use crate::snapshot::{Node, Snapshot};
use rgaa_analysis::{AxTree, Portee, Rect};
use std::collections::HashSet;

fn overlaps(a: Rect, b: Rect) -> bool {
    a.x < b.x + b.width && b.x < a.x + a.width && a.y < b.y + b.height && b.y < a.y + a.height
}

/// Median font size of the body text — short text at a normal weight.
#[must_use]
pub fn median_body_size(nodes: &[Node]) -> f64 {
    let mut sizes: Vec<f64> = nodes
        .iter()
        .filter(|node| node.is_visible() && node.has_text() && node.weight() < 600)
        .filter_map(Node::size_px)
        .collect();
    if sizes.is_empty() {
        sizes = nodes
            .iter()
            .filter(|node| node.is_visible() && node.has_text())
            .filter_map(Node::size_px)
            .collect();
    }
    sizes.sort_by(f64::total_cmp);
    sizes.get(sizes.len() / 2).copied().unwrap_or(16.0)
}

/// A short text block, alone on its line, noticeably larger or bolder than the
/// body (spec §3, 9.1).
#[must_use]
pub fn is_visual_heading(node: &Node, median: f64) -> Option<f64> {
    let size = node.size_px()?;
    if !node.is_visible() || !node.has_text() || node.text.chars().count() >= 120 {
        return None;
    }
    if size < median * 1.25 && !(node.weight() >= 600 && size > median) {
        return None;
    }
    Some(size)
}

/// 9.1 measurement. `ax_tree` reproduces the CDP `Accessibility.getFullAXTree`
/// names, which corroborate the DOM's exposed headings.
#[must_use]
pub fn analyze(snapshot: &Snapshot, ax_tree: Option<&AxTree>, _lang: &str) -> Measurement {
    let median = median_body_size(&snapshot.nodes);
    let ax_names: HashSet<&str> = ax_tree
        .map(|tree| {
            tree.nodes
                .iter()
                .filter(|node| node.role == "heading")
                .map(|node| node.name.trim())
                .collect()
        })
        .unwrap_or_default();

    let mut titres_visuels = Vec::new();
    for (index, node) in snapshot.nodes.iter().enumerate() {
        let Some(size) = is_visual_heading(node, median) else {
            continue;
        };
        if let Some(box_) = node.bbox {
            let shares_line = snapshot
                .nodes
                .iter()
                .enumerate()
                .any(|(other_index, other)| {
                    other_index != index
                        && other.is_visible()
                        && other.has_text()
                        && other
                            .bbox
                            .is_some_and(|other_box| overlaps(box_, other_box))
                });
            if shares_line {
                continue;
            }
        }
        let role_heading = snapshot
            .ax
            .iter()
            .any(|ax| ax.node == index && ax.role == "heading");
        let expose = node.is_heading() || role_heading || ax_names.contains(node.text.trim());
        titres_visuels.push(TitreVisuel {
            texte: node.text.clone(),
            taille_px: size,
            graisse: node.weight(),
            expose,
        });
    }

    let mut titres_exposes = Vec::new();
    for (index, node) in snapshot.nodes.iter().enumerate() {
        let ax = snapshot.ax.iter().find(|ax| ax.node == index);
        let role_heading = node.role.as_deref() == Some("heading")
            || ax.is_some_and(|ax| ax.role == "heading")
            || ax_names.contains(node.text.trim());
        let Some(niveau) = node.heading_level().or_else(|| ax.and_then(|ax| ax.level)) else {
            continue;
        };
        if !node.is_visible() && !role_heading {
            continue;
        }
        titres_exposes.push(TitreExpose {
            niveau: Some(niveau),
            texte: if node.has_text() {
                node.text.clone()
            } else {
                ax.map_or_else(String::new, |ax| ax.name.clone())
            },
            taille_px: node.size_px(),
        });
    }

    let mut inversions = 0usize;
    let mut by_level = titres_exposes.clone();
    by_level.sort_by_key(|heading| heading.niveau);
    for pair in by_level.windows(2) {
        let (higher, lower) = (&pair[0], &pair[1]);
        if let (Some(a), Some(b)) = (higher.niveau, lower.niveau) {
            if a < b && higher.taille_px.unwrap_or(0.0) < lower.taille_px.unwrap_or(0.0) {
                inversions += 1;
            }
        }
    }

    let mut sauts_niveau = Vec::new();
    let mut previous: Option<u8> = None;
    for heading in &titres_exposes {
        if let (Some(before), Some(after)) = (previous, heading.niveau) {
            if after > before + 1 {
                sauts_niveau.push(format!("h{before}→h{after}"));
            }
        }
        previous = heading.niveau;
    }

    let mut raisons = Vec::new();
    if titres_visuels.iter().any(|heading| !heading.expose) {
        raisons.push("titre visuel non exposé");
    }
    if inversions > 0 {
        raisons.push("inversion de titres");
    }
    if !sauts_niveau.is_empty() {
        raisons.push("saut de niveau");
    }

    let mesure = Measurement::new(
        Mesures::Headings {
            titres_visuels,
            titres_exposes,
            mediane_corps_px: median,
            inversions,
            sauts_niveau,
            avis_modele: None,
        },
        Portee::Mesuree,
        &["9.1.1", "9.1.3"],
    );
    if raisons.is_empty() {
        mesure
    } else {
        mesure.suspect(raisons.join(" ; "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_bold_block_larger_than_the_body_is_a_visual_heading() {
        let node = Node {
            text: "Nos offres".to_owned(),
            font_size: Some("28px".to_owned()),
            font_weight: Some("700".to_owned()),
            ..Node::default()
        };
        assert_eq!(is_visual_heading(&node, 16.0), Some(28.0));
    }

    #[test]
    fn body_sized_text_is_not_a_visual_heading() {
        let node = Node {
            text: "Un paragraphe".to_owned(),
            font_size: Some("16px".to_owned()),
            font_weight: Some("400".to_owned()),
            ..Node::default()
        };
        assert_eq!(is_visual_heading(&node, 16.0), None);
    }
}
