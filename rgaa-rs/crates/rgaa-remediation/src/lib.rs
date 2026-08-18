mod adapters;
mod lifecycle;
mod policy;
mod proposals;

pub use adapters::*;
pub use lifecycle::*;
pub use policy::*;
pub use proposals::*;

#[cfg(test)]
mod contract_tests {
    use super::*;

    fn issue(id: &str) -> RemediationIssue {
        RemediationIssue {
            id: id.into(),
            rule: "image-alt".into(),
            element_html: "import React from \"react\"; <img src=\"hero.png\">".into(),
            page_url: "https://example.test".into(),
            source_locations: vec![SourceLocation {
                file: "src/App.tsx".into(),
                line: 10,
                column: Some(4),
            }],
            summary: "Image has no alternative text".into(),
            remediation: "Add an alt attribute".into(),
            criteria: vec!["RGAA-1.1".into()],
            framework: Some(Framework::React),
        }
    }

    #[test]
    fn lifecycle_accepts_valid_and_rejects_invalid_transitions() {
        let mut lifecycle = FindingLifecycle::new("f-1");
        lifecycle
            .transition(FindingState::Triaged, "auditor", "reviewed")
            .expect("valid");
        assert!(lifecycle
            .transition(FindingState::Resolved, "auditor", "skip")
            .is_err());
        assert_eq!(lifecycle.history().len(), 1);
    }

    #[test]
    fn batch_preserves_independent_success_and_error_correlations() {
        let policy = RemediationPolicy::default();
        let results = remediate(
            &[
                issue("ok"),
                RemediationIssue {
                    id: "bad".into(),
                    rule: String::new(),
                    ..issue("bad")
                },
            ],
            &policy,
            &ReactAdapter,
        )
        .expect("valid batch");
        assert_eq!(results.len(), 2);
        assert!(
            matches!(&results[0], RemediationOutcome::Ok(guidance) if guidance.issue_id == "ok")
        );
        assert!(matches!(&results[1], RemediationOutcome::Error(error) if error.issue_id == "bad"));
    }

    #[test]
    fn batch_bounds_are_one_through_twenty_five() {
        let policy = RemediationPolicy::default();
        assert!(remediate(&[], &policy, &ReactAdapter).is_err());
        let issues = (0..26).map(|i| issue(&i.to_string())).collect::<Vec<_>>();
        assert!(remediate(&issues, &policy, &ReactAdapter).is_err());
    }

    #[test]
    fn proposal_hash_is_stable() {
        let proposal = PatchProposal::new(
            "p-1",
            vec!["f-1".into()],
            "diff",
            vec!["src/App.tsx".into()],
            "rationale",
            vec!["risk".into()],
            vec!["cargo test".into()],
            "fixes alt",
        );
        assert_eq!(proposal.proposal_hash, proposal.compute_hash());
        assert_eq!(proposal.proposal_hash, proposal.clone().compute_hash());
    }

    #[test]
    fn policy_denies_remote_remediation() {
        let policy = RemediationPolicy {
            allow_remote_ai: false,
            use_remote_ai: true,
            ..Default::default()
        };
        let result = remediate(&[issue("remote")], &policy, &ReactAdapter);
        assert!(
            matches!(result, Ok(outcomes) if matches!(&outcomes[0], RemediationOutcome::Error(error) if error.code == RemediationErrorCode::PolicyDenied))
        );
    }

    #[test]
    fn adapters_detect_frameworks_and_propose_fixture_fixes() {
        let fixtures = [
            (
                "import React from \"react\"; <img src=\"hero.png\">",
                Framework::React,
                "alt",
            ),
            (
                "'use client'; <button></button>",
                Framework::Next,
                "aria-label",
            ),
            (
                "<template><img src=\"hero.png\"></template>",
                Framework::Vue,
                "alt",
            ),
            (
                "@Component({template: '<img src=\"hero.png\">'})",
                Framework::Angular,
                "alt",
            ),
        ];
        for (source, framework, expected) in fixtures {
            let adapter = adapter_for(framework);
            assert_eq!(adapter.detect(source), Some(framework));
            let mut fixture_issue = issue("fixture");
            fixture_issue.framework = Some(framework);
            fixture_issue.rule = if framework == Framework::Next {
                "button-name".into()
            } else {
                "image-alt".into()
            };
            let proposal = adapter.propose(&fixture_issue, source).expect("proposal");
            assert!(proposal.diff.contains(expected));
            assert_ne!(proposal.diff, source);
        }
        assert_eq!(
            ReactAdapter.detect("<template><img src=\"x\"></template>"),
            None
        );
        assert_eq!(VueAdapter.detect("arbitrary source"), None);
        assert!(matches!(
            ReactAdapter.propose(&issue("ambiguous"), "<img src={value} />"),
            Err(RemediationError::NeedsReview { .. })
        ));
    }

    #[test]
    fn approval_is_required_until_explicitly_granted() {
        let policy = RemediationPolicy::default();
        let mut outcomes = remediate(&[issue("approval")], &policy, &ReactAdapter).expect("batch");
        let RemediationOutcome::Ok(guidance) = &mut outcomes[0] else {
            panic!("expected proposal")
        };
        assert!(guidance.proposal.requires_approval());
        assert!(guidance.proposal.ensure_approved().is_err());
        let token = guidance.proposal.approval_token();
        guidance
            .proposal
            .approve("reviewer", &token)
            .expect("approve");
        assert!(guidance.proposal.ensure_approved().is_ok());
    }

    #[test]
    fn proposals_reject_mismatched_framework_and_unsafe_sources() {
        let mut mismatched = issue("mismatch");
        mismatched.framework = Some(Framework::Vue);
        assert!(matches!(
            ReactAdapter.propose(&mismatched, "import React from \"react\"; <img src=\"x\">"),
            Err(RemediationError::UnsupportedFramework { .. })
        ));

        let mut control = issue("control");
        control.framework = Some(Framework::Angular);
        control.rule = "label".into();
        control.element_html = "@Component({template: '<input [value]=\"name\">'})".into();
        assert!(
            matches!(AngularAdapter.propose(&control, &control.element_html), Err(RemediationError::NeedsReview { reason, .. }) if reason.contains("dynamic"))
        );
        assert!(matches!(
            ReactAdapter.propose(&issue("empty"), ""),
            Err(RemediationError::NeedsReview { .. })
        ));
        let mut labeled = issue("labeled");
        labeled.rule = "label".into();
        labeled.element_html =
            "import React from \"react\"; <label for=\"email\">Email</label><input id=\"email\">"
                .into();
        assert!(matches!(
            ReactAdapter.propose(&labeled, &labeled.element_html),
            Err(RemediationError::NeedsReview { reason, .. }) if reason.contains("ambiguous")
        ));
    }

    #[test]
    fn dynamic_image_bindings_need_review_in_each_framework_syntax() {
        let cases = [
            (
                Framework::React,
                "import React from \"react\"; <img src = { image } />",
            ),
            (
                Framework::Vue,
                "<template><img :src = \"image\"></template>",
            ),
            (
                Framework::Vue,
                "<template><img v-bind:src = \"image\"></template>",
            ),
            (
                Framework::Angular,
                "@Component({template: '<img [src] = \"image\">'})",
            ),
            (
                Framework::Angular,
                "@Component({template: '<img bind-src = \"image\">'})",
            ),
        ];
        for (framework, source) in cases {
            let mut image = issue("dynamic-image");
            image.framework = Some(framework);
            assert!(matches!(
                adapter_for(framework).propose(&image, source),
                Err(RemediationError::NeedsReview { reason, .. }) if reason.contains("dynamic")
            ));
        }

        let dynamic_controls = [
            (
                Framework::React,
                "import React from \"react\"; <input value = { value } />",
                "label",
            ),
            (
                Framework::Vue,
                "<template><input v-model = \"value\"></template>",
                "label",
            ),
            (
                Framework::Angular,
                "@Component({template: '<input [value] = \"value\">'})",
                "label",
            ),
            (
                Framework::React,
                "import React from \"react\"; <button>{ label }</button>",
                "button-name",
            ),
            (
                Framework::Vue,
                "<template><button>{{ label }}</button></template>",
                "button-name",
            ),
            (
                Framework::Angular,
                "@Component({template: '<button *ngIf = \"show\"></button>'})",
                "button-name",
            ),
        ];
        for (framework, source, rule) in dynamic_controls {
            let mut dynamic = issue("dynamic-control");
            dynamic.framework = Some(framework);
            dynamic.rule = rule.into();
            assert!(matches!(
                adapter_for(framework).propose(&dynamic, source),
                Err(RemediationError::NeedsReview { .. })
            ));
        }
    }

    #[test]
    fn policy_can_make_approval_optional_without_marking_required() {
        let policy = RemediationPolicy {
            require_approval: false,
            ..Default::default()
        };
        let outcomes = remediate(&[issue("optional")], &policy, &ReactAdapter).expect("batch");
        let RemediationOutcome::Ok(guidance) = &outcomes[0] else {
            panic!("expected proposal")
        };
        assert!(!guidance.proposal.requires_approval());
        guidance.proposal.ensure_approved().expect("not required");
    }

    #[test]
    fn detection_is_ordered_and_non_overlapping() {
        let next_react =
            "\"use client\"; import React from \"react\"; export default function Page() {}";
        assert_eq!(detect_framework(next_react), Some(Framework::Next));
        assert_eq!(NextAdapter.detect(next_react), Some(Framework::Next));
        assert_eq!(ReactAdapter.detect(next_react), None);
        assert_eq!(detect_framework("random source"), None);
    }

    #[test]
    fn omitted_framework_deserializes_and_requires_safe_detection() {
        let json = r#"{"id":"f","rule":"image-alt","element_html":"<img src=\"x\">","page_url":"https://x.test","source_locations":[{"file":"x.tsx","line":1}],"summary":"missing alt","remediation":"add alt","criteria":[]}"#;
        let parsed: RemediationIssue = serde_json::from_str(json).expect("compatible issue");
        assert_eq!(parsed.framework, None);
        let policy = RemediationPolicy::default();
        let outcomes = remediate(&[parsed], &policy, &ReactAdapter).expect("batch");
        assert!(
            matches!(&outcomes[0], RemediationOutcome::Error(error) if error.code == RemediationErrorCode::NeedsReview)
        );
        let mut safe_issue = issue("safe-omitted");
        safe_issue.framework = None;
        let safe = remediate(&[safe_issue], &policy, &ReactAdapter).expect("batch");
        assert!(matches!(&safe[0], RemediationOutcome::Ok(_)));
    }

    #[test]
    fn approval_token_is_bound_to_proposal_identity() {
        let mut proposal = PatchProposal::new(
            "p",
            vec!["f".into()],
            "diff",
            vec!["file".into()],
            "why",
            vec![],
            vec![],
            "effect",
        );
        assert!(proposal.approve("reviewer", "wrong-token").is_err());
        let token = proposal.approval_token();
        proposal.approve("reviewer", &token).expect("valid token");
        proposal.proposal_id = "changed".into();
        assert!(proposal.ensure_approved().is_err());
        let mut changed = PatchProposal::new(
            "p",
            vec!["f".into()],
            "diff",
            vec!["file".into()],
            "why",
            vec![],
            vec![],
            "effect",
        );
        let changed_token = changed.approval_token();
        changed
            .approve("reviewer", &changed_token)
            .expect("valid token");
        changed.diff = "different diff".into();
        assert!(changed.ensure_approved().is_err());
    }

    #[test]
    fn deserialized_not_required_state_cannot_bypass_approval() {
        let proposal = PatchProposal::new(
            "serialized",
            vec!["finding".into()],
            "diff",
            vec!["file".into()],
            "why",
            vec![],
            vec![],
            "effect",
        );
        let mut payload = serde_json::to_value(proposal).expect("serialize proposal");
        payload["approval"] = serde_json::json!("NotRequired");
        let restored: PatchProposal =
            serde_json::from_value(payload).expect("deserialize proposal");
        assert!(restored.requires_approval());
        assert!(restored.ensure_approved().is_err());
    }

    #[test]
    fn malformed_markup_returns_review_without_panicking() {
        let cases = [
            ("image-alt", "import React from \"react\"; <img src=\"x\""),
            ("label", "import React from \"react\"; <input id=\"x\""),
            ("button-name", "import React from \"react\"; <button>"),
        ];
        for (rule, source) in cases {
            let mut issue = issue("malformed");
            issue.rule = rule.into();
            issue.element_html = source.into();
            assert!(matches!(
                ReactAdapter.propose(&issue, source),
                Err(RemediationError::NeedsReview { .. })
            ));
        }
    }
}
