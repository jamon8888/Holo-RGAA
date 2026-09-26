#[test]
fn rgaa_agent_is_sync() {
    fn assert_sync<T: Sync>() {}
    assert_sync::<rgaa_agent::agent::RgaaAgent>();
    assert_sync::<rgaa_core::Criterion>();
    assert_sync::<rgaa_holo::PageContext>();
}
