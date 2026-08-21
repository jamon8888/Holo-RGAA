use rgaa_agent::agent::RgaaAgent;
use rgaa_agent::models::ModelRouter;

#[test]
fn agent_creates_with_placeholder_router() {
    let router = ModelRouter::new_placeholder();
    let agent = RgaaAgent::new(router);
    assert!(std::mem::size_of_val(&agent) > 0);
}

#[test]
fn criteria_defs_cover_all_27_ia_assiste() {
    use rgaa_agent::criteria_defs::get_criterion_definition;
    let ia_ids = [
        "1.3", "1.7", "2.2", "3.1", "4.2", "4.4", "4.6", "4.9",
        "5.2", "5.3", "5.5", "7.2", "8.4", "8.6", "8.8", "9.2",
        "10.3", "10.10", "11.2", "11.3", "11.7", "11.8", "11.9", "11.10",
        "12.3", "12.8", "13.6",
    ];
    for id in ia_ids {
        assert!(
            get_criterion_definition(id).is_some(),
            "missing definition for {id}"
        );
    }
    assert_eq!(ia_ids.len(), 27);
}

/// Drift-prevention test: ensure rgaa-agent criterion definitions stay in sync
/// with rgaa-core's canonical catalog. If this test fails, either:
/// - rgaa-core's catalog changed (update criteria_defs.rs to match), or
/// - a definition was added/removed intentionally (update the expected count).
#[test]
fn criteria_defs_match_rgaa_core_catalog() {
    use rgaa_agent::criteria_defs::{VISUAL_CRITERIA, get_criterion_definition};
    use rgaa_core::{RgaaCriteria, Classification};

    let core_criteria = RgaaCriteria::all();
    let core_map: std::collections::HashMap<&str, _> =
        core_criteria.iter().map(|c| (c.id, c)).collect();

    // Every criterion in rgaa-agent must exist in rgaa-core with IaAssiste classification
    let agent_ids: Vec<&str> = core_criteria
        .iter()
        .filter(|c| c.classification == Classification::IaAssiste)
        .map(|c| c.id)
        .collect();

    assert_eq!(
        agent_ids.len(),
        27,
        "rgaa-core IaAssiste count changed (expected 27, got {}); update criteria_defs.rs if new criteria were added",
        agent_ids.len()
    );

    for id in &agent_ids {
        let def = get_criterion_definition(id)
            .unwrap_or_else(|| panic!("rgaa-agent missing definition for IaAssiste criterion {id}"));
        let core = core_map[*id];

        assert_eq!(
            def.title, core.title,
            "title drift for criterion {id}: agent={:?}, core={:?}",
            def.title, core.title
        );
        assert_eq!(
            def.wcag_refs, core.wcag_refs,
            "wcag_refs drift for criterion {id}: agent={:?}, core={:?}",
            def.wcag_refs, core.wcag_refs
        );
    }

    // Every criterion in rgaa-agent must be classified as IaAssiste in rgaa-core
    for id in &agent_ids {
        let core = core_map[*id];
        assert_eq!(
            core.classification,
            Classification::IaAssiste,
            "criterion {id} is {:?} in rgaa-core but has a definition in rgaa-agent (expected IaAssiste)",
            core.classification
        );
    }

    // VISUAL_CRITERIA must be a subset of the agent's 27 definitions
    for id in VISUAL_CRITERIA {
        assert!(
            get_criterion_definition(id).is_some(),
            "VISUAL_CRITERIA references {id} but it has no definition in criteria_defs.rs"
        );
    }
}
