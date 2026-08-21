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
