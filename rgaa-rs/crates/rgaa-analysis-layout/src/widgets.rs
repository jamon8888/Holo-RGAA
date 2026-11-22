//! 7.1 — custom widgets: AX role and state before and after a simulated click.

use crate::mesures::{bounded, Measurement, Mesures, WidgetObserve};
use crate::snapshot::{AxInfo, Snapshot};
use rgaa_analysis::Portee;

const AX_STATE_KEYS: &[&str] = &["expanded", "pressed", "checked"];
/// A render change below 2 % of the widget box is ignored (spec §3, 7.1).
const PIXEL_CHANGE_THRESHOLD: f64 = 0.02;

fn ax_or_default(snapshot: &Snapshot, index: usize) -> AxInfo {
    snapshot
        .ax
        .iter()
        .find(|ax| ax.node == index)
        .cloned()
        .unwrap_or_default()
}

/// 7.1 measurement. Only the first `max_actions` actions are simulated, and
/// actions on forbidden elements are never executed (spec §6.3).
#[must_use]
pub fn analyze(snapshot: &Snapshot, max_actions: usize) -> Measurement {
    let allowed = bounded(0..snapshot.actions.len(), max_actions);
    let mut observes = Vec::new();
    let mut ignored = 0usize;
    let mut raisons = Vec::new();

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
        if !target.is_custom_widget() {
            continue;
        }
        let before = action
            .ax_before
            .clone()
            .unwrap_or_else(|| ax_or_default(snapshot, action.target));
        let after = action
            .ax_after
            .clone()
            .unwrap_or_else(|| ax_or_default(snapshot, action.target));
        let state_changed = AX_STATE_KEYS
            .iter()
            .any(|key| before.has_state(key) != after.has_state(key));
        let render_changed = action.pixels_changed > PIXEL_CHANGE_THRESHOLD;
        let raison = if before.is_generic() || after.is_generic() {
            "rôle AX générique"
        } else if render_changed && !state_changed {
            "état AX inchangé alors que le rendu change"
        } else {
            ""
        };
        if !raison.is_empty() {
            raisons.push(raison.to_owned());
        }
        observes.push(WidgetObserve {
            selecteur: target.selecteur.clone(),
            role_avant: before.role.clone(),
            role_apres: after.role.clone(),
            etats_avant: before.states.clone(),
            etats_apres: after.states.clone(),
            pixels_changed: action.pixels_changed,
            raison: raison.to_owned(),
        });
    }

    let mesure = Measurement::new(
        Mesures::Widget {
            observes,
            actions_ignorees: ignored,
        },
        Portee::Mesuree,
        &["7.1.1", "7.1.2", "7.1.3"],
    );
    if raisons.is_empty() {
        mesure
    } else {
        mesure.suspect(raisons.join(" ; "))
    }
}
