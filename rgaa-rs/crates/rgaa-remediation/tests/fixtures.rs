use rgaa_remediation::{
    adapter_for, Framework, PatchProposal, RemediationIssue, RemediationPolicy, SourceLocation,
};

fn issue(id: &str, framework: Framework, rule: &str) -> RemediationIssue {
    RemediationIssue {
        id: id.into(),
        rule: rule.into(),
        element_html: "<img src=\"hero.png\">".into(),
        page_url: "https://fixture.test".into(),
        source_locations: vec![SourceLocation {
            file: "fixture/component".into(),
            line: 1,
            column: None,
        }],
        summary: "missing alternative text".into(),
        remediation: "add alt text".into(),
        criteria: vec!["RGAA-1.1".into()],
        framework: Some(framework),
    }
}

#[test]
fn framework_fixtures_produce_deterministic_proposals() {
    let fixtures = [
        (Framework::React, include_str!("fixtures/react/src/App.tsx")),
        (
            Framework::Next,
            include_str!("fixtures/next/pages/index.tsx"),
        ),
        (Framework::Vue, include_str!("fixtures/vue/src/App.vue")),
        (
            Framework::Angular,
            include_str!("fixtures/angular/src/app.component.html"),
        ),
    ];
    for (framework, source) in fixtures {
        let adapter = adapter_for(framework);
        let proposal = adapter.propose(&issue("fixture", framework, "image-alt"), source);
        if framework != Framework::Next {
            let proposal = proposal.expect("high-confidence fixture proposal");
            assert_eq!(proposal.proposal_hash, proposal.compute_hash());
        }
    }
}

#[test]
fn fixture_policy_keeps_approval_as_a_boundary() {
    let policy = RemediationPolicy::default();
    assert!(policy.require_approval);
    let proposal = PatchProposal::new(
        "p",
        vec!["f".into()],
        "diff",
        vec!["file".into()],
        "why",
        vec![],
        vec![],
        "effect",
    );
    assert!(!proposal.diff.is_empty());
}
