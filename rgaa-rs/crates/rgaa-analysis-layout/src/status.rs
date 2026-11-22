//! 7.5 — status messages appearing after an action, and their AX exposure.

use crate::mesures::{bounded, Measurement, MessageStatut, Mesures};
use crate::snapshot::Snapshot;
use rgaa_analysis::Portee;

/// 7.5 measurement. A visible message must sit in an `aria-live` region, carry
/// `role=status|alert`, or receive focus; otherwise it is not restored to AT.
#[must_use]
pub fn analyze(snapshot: &Snapshot, max_actions: usize) -> Measurement {
    let allowed = bounded(0..snapshot.actions.len(), max_actions);
    let mut messages = Vec::new();
    let mut ignored = 0usize;
    let mut non_restitues = 0usize;

    for index in allowed {
        let action = &snapshot.actions[index];
        let Some(target) = snapshot.nodes.get(action.target) else {
            ignored += 1;
            continue;
        };
        if target.is_forbidden_action() {
            ignored += 1;
            continue;
        }
        for &message_index in &action.produced {
            let Some(node) = snapshot.nodes.get(message_index) else {
                continue;
            };
            if !node.is_visible() || !node.has_text() {
                continue;
            }
            let ax = snapshot.ax.iter().find(|ax| ax.node == message_index);
            let region_live = node
                .aria_live
                .clone()
                .or_else(|| ax.and_then(|ax| ax.live.clone()))
                .filter(|value| value != "off");
            let role = node.role.clone().or_else(|| {
                ax.map(|ax| ax.role.clone())
                    .filter(|role| role == "status" || role == "alert")
            });
            let focus = action.focus_after == Some(message_index)
                || ax.is_some_and(|ax| ax.has_state("focused"));
            let exposed = region_live.is_some()
                || matches!(role.as_deref(), Some("status") | Some("alert"))
                || focus;
            if !exposed {
                non_restitues += 1;
            }
            messages.push(MessageStatut {
                selecteur: node.selecteur.clone(),
                texte: node.text.clone(),
                region_live,
                role,
                focus,
            });
        }
    }

    let mesure = Measurement::new(
        Mesures::Status {
            messages,
            actions_ignorees: ignored,
        },
        Portee::Mesuree,
        &["7.5.1", "7.5.2"],
    );
    if non_restitues == 0 {
        mesure
    } else {
        mesure.suspect("message visible non restitué")
    }
}
