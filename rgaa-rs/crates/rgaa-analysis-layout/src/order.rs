//! Visual vs DOM reading order (10.3 and 5.3, spec §3).
//!
//! Nodes are grouped into lines with an 8 px tolerance, then sorted by `x`
//! (reversed for `dir=rtl`). Kendall's tau compares that visual sequence with
//! the DOM sequence; a positioned element jumping more than three ranks is
//! reported separately even when tau stays high.

use crate::snapshot::Node;

/// Vertical tolerance for considering two boxes on the same line (spec §3).
pub const LINE_TOLERANCE_PX: f64 = 8.0;
/// A positioned element skipping more than this many ranks is a suspect.
pub const MAX_RANK_JUMP: usize = 3;

fn box_of(node: &Node) -> Option<rgaa_analysis::Rect> {
    node.bbox.filter(|_| node.is_visible() && node.has_text())
}

/// Indices of the nodes that take part in a reading order, in DOM order.
#[must_use]
pub fn ordered_indices(nodes: &[Node]) -> Vec<usize> {
    nodes
        .iter()
        .enumerate()
        .filter_map(|(index, node)| box_of(node).map(|_| index))
        .collect()
}

/// Same set of indices, sorted by visual line (`y`) then horizontal position
/// (`x`), i.e. the order a sighted reader follows.
#[must_use]
pub fn visual_order(nodes: &[Node], rtl: bool) -> Vec<usize> {
    let mut sorted = ordered_indices(nodes);
    sorted.sort_by(|&left, &right| {
        let a = box_of(&nodes[left]).expect("filtered node has a box");
        let b = box_of(&nodes[right]).expect("filtered node has a box");
        a.y.partial_cmp(&b.y)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.x.partial_cmp(&b.x).unwrap_or(std::cmp::Ordering::Equal))
    });

    let mut lines: Vec<Vec<usize>> = Vec::new();
    let mut line_y: Option<f64> = None;
    for index in sorted {
        let y = box_of(&nodes[index]).expect("box").y;
        match line_y {
            Some(current) if (y - current).abs() <= LINE_TOLERANCE_PX => {
                lines
                    .last_mut()
                    .expect("a line exists when line_y is set")
                    .push(index);
            }
            _ => {
                lines.push(vec![index]);
                line_y = Some(y);
            }
        }
    }

    let mut visual = Vec::with_capacity(lines.iter().map(Vec::len).sum());
    for mut line in lines {
        line.sort_by(|&left, &right| {
            let a = box_of(&nodes[left]).expect("box");
            let b = box_of(&nodes[right]).expect("box");
            a.x.partial_cmp(&b.x).unwrap_or(std::cmp::Ordering::Equal)
        });
        if rtl {
            line.reverse();
        }
        visual.extend(line);
    }
    visual
}

/// Kendall's tau between two sequences of the same indices, plus the number
/// of discordant pairs (the "inversions" of the spec). Ties in the visual
/// sequence are handled with the tau-b denominator.
#[must_use]
pub fn kendall_tau(dom: &[usize], visual: &[usize]) -> (f64, usize) {
    let n = dom.len();
    if n < 2 {
        return (1.0, 0);
    }
    let mut concordant = 0usize;
    let mut discordant = 0usize;
    let mut visual_ties = 0usize;
    for i in 0..n {
        for j in (i + 1)..n {
            match visual[i].cmp(&visual[j]) {
                std::cmp::Ordering::Less => concordant += 1,
                std::cmp::Ordering::Greater => discordant += 1,
                std::cmp::Ordering::Equal => visual_ties += 1,
            }
        }
    }
    let pairs = n * (n - 1) / 2;
    let denominator = ((pairs - visual_ties) as f64).sqrt() * (pairs as f64).sqrt();
    let tau = if denominator == 0.0 {
        1.0
    } else {
        (concordant as f64 - discordant as f64) / denominator
    };
    (tau, discordant)
}

/// Whether a node's placement can move it across reading lines.
#[must_use]
pub fn is_positioned(node: &Node) -> Option<&str> {
    if node.order.is_some() {
        Some("order")
    } else if node.grid_area.is_some() {
        Some("grid-area")
    } else if matches!(node.position.as_deref(), Some("absolute") | Some("fixed")) {
        Some("position")
    } else {
        None
    }
}

/// 10.3 measurement: DOM order versus visual order (Kendall tau) and large
/// jumps caused by positioning.
#[must_use]
pub fn reading_measurement(
    snapshot: &crate::snapshot::Snapshot,
    rtl: bool,
) -> crate::mesures::Measurement {
    use crate::mesures::{Measurement, Mesures, SautPositionne};
    use rgaa_analysis::Portee;

    let dom = ordered_indices(&snapshot.nodes);
    let visual = visual_order(&snapshot.nodes, rtl);
    let (tau, inversions) = kendall_tau(&dom, &visual);

    let sauts_positionnes = dom
        .iter()
        .enumerate()
        .filter_map(|(rang_dom, &index)| {
            let node = &snapshot.nodes[index];
            let motif = is_positioned(node)?;
            let rang_visuel = visual.iter().position(|&candidate| candidate == index)?;
            (rang_dom.abs_diff(rang_visuel) > MAX_RANK_JUMP).then(|| SautPositionne {
                selecteur: node.selecteur.clone(),
                rang_dom,
                rang_visuel,
                motif: motif.to_owned(),
            })
        })
        .collect::<Vec<_>>();

    let selecteur_of = |index: &usize| {
        snapshot
            .nodes
            .get(*index)
            .map_or_else(String::new, |node| node.selecteur.clone())
    };
    let mesure = Measurement::new(
        Mesures::ReadingOrder {
            tau,
            inversions,
            ordre_dom: dom.iter().map(selecteur_of).collect(),
            ordre_visuel: visual.iter().map(selecteur_of).collect(),
            sauts_positionnes: sauts_positionnes.clone(),
            direction: if rtl { "rtl" } else { "ltr" }.to_owned(),
            avis_modele: None,
        },
        Portee::Mesuree,
        &["10.3.1"],
    );
    if tau < 0.8 || !sauts_positionnes.is_empty() {
        mesure.suspect("ordre de lecture visuel et DOM divergents")
    } else {
        mesure
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn nodes(boxes: &[(f64, f64)]) -> Vec<Node> {
        boxes
            .iter()
            .enumerate()
            .map(|(index, (x, y))| Node {
                selecteur: format!("n{index}"),
                text: format!("texte {index}"),
                bbox: Some(rgaa_analysis::Rect {
                    x: *x,
                    y: *y,
                    width: 10.0,
                    height: 10.0,
                }),
                ..Node::default()
            })
            .collect()
    }

    #[test]
    fn lines_are_grouped_with_an_eight_pixel_tolerance() {
        let nodes = nodes(&[(100.0, 0.0), (0.0, 4.0), (50.0, 40.0)]);
        assert_eq!(visual_order(&nodes, false), vec![1, 0, 2]);
    }

    #[test]
    fn rtl_reverses_the_horizontal_reading_order() {
        let nodes = nodes(&[(0.0, 0.0), (100.0, 0.0)]);
        assert_eq!(visual_order(&nodes, true), vec![1, 0]);
    }

    #[test]
    fn kendall_tau_is_minus_one_for_a_full_reversal() {
        let (tau, inversions) = kendall_tau(&[0, 1, 2], &[2, 1, 0]);
        assert!((tau + 1.0).abs() < 1e-9, "{tau}");
        assert_eq!(inversions, 3);
    }
}
