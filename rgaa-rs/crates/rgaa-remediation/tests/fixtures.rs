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
            include_str!("fixtures/angular/src/app.component.ts"),
        ),
    ];
    for (framework, source) in fixtures {
        let adapter = adapter_for(framework);
        assert_eq!(adapter.detect(source), Some(framework));
        let rule = if framework == Framework::Next {
            "button-name"
        } else {
            "image-alt"
        };
        let proposal = adapter
            .propose(&issue("fixture", framework, rule), source)
            .expect("high-confidence fixture proposal");
        assert_ne!(proposal.diff, source);
        assert!(proposal.diff.contains(if framework == Framework::Next {
            "aria-label"
        } else {
            "alt"
        }));
        assert_eq!(proposal.proposal_hash, proposal.compute_hash());
    }
}

#[test]
fn fixture_control_proposal_adds_an_accessible_name() {
    let source = include_str!("fixtures/react/src/App.tsx");
    let mut issue = issue("control", Framework::React, "label");
    issue.element_html = "<input id=\"email\">".into();
    let proposal = adapter_for(Framework::React)
        .propose(&issue, source)
        .expect("control proposal");
    assert_ne!(proposal.diff, source);
    assert!(proposal.diff.contains("aria-label=\"email\""));
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
