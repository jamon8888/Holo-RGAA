use crate::{PatchProposal, RemediationError, RemediationIssue, SourceLocation};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Framework {
    React,
    Next,
    Vue,
    Angular,
}

pub trait FrameworkAdapter: Send + Sync {
    fn framework(&self) -> Framework;
    fn detect(&self, source: &str) -> Option<Framework>;
    fn locate(&self, source: &str, issue: &RemediationIssue) -> Vec<SourceLocation>;
    fn propose(
        &self,
        issue: &RemediationIssue,
        source: &str,
    ) -> Result<PatchProposal, RemediationError>;
}

pub struct ReactAdapter;
pub struct NextAdapter;
pub struct VueAdapter;
pub struct AngularAdapter;

pub fn adapter_for(framework: Framework) -> &'static dyn FrameworkAdapter {
    match framework {
        Framework::React => &ReactAdapter,
        Framework::Next => &NextAdapter,
        Framework::Vue => &VueAdapter,
        Framework::Angular => &AngularAdapter,
    }
}

fn propose_for(
    framework: Framework,
    issue: &RemediationIssue,
    source: &str,
) -> Result<PatchProposal, RemediationError> {
    let detected = detect_framework(source);
    if issue.framework != Some(framework) && issue.framework.is_some() {
        return Err(RemediationError::UnsupportedFramework {
            issue_id: issue.id.clone(),
        });
    }
    if detected != Some(framework) {
        return Err(RemediationError::NeedsReview {
            issue_id: issue.id.clone(),
            reason: "source does not safely identify the requested framework".into(),
        });
    }
    let trimmed = source.trim();
    if trimmed.is_empty() {
        return Err(RemediationError::NeedsReview {
            issue_id: issue.id.clone(),
            reason: "source is empty".into(),
        });
    }

    let diff = if issue.rule.contains("image") && trimmed.contains("<img") {
        let tag = opening_tag(trimmed, "img").ok_or_else(|| RemediationError::NeedsReview {
            issue_id: issue.id.clone(),
            reason: "image source is incomplete".into(),
        })?;
        let compact_tag = compact(tag);
        if compact_tag.contains("[src")
            || compact_tag.contains(":src")
            || compact_tag.contains("v-bind:src")
            || compact_tag.contains("src={")
            || compact_tag.contains("{{")
            || compact_tag.contains("bind:src")
            || compact_tag.contains("bind-src")
        {
            return Err(RemediationError::NeedsReview {
                issue_id: issue.id.clone(),
                reason: "image source uses a dynamic binding".into(),
            });
        }
        if tag.contains(" alt=")
            || tag.contains(" aria-label=")
            || tag.contains(" aria-labelledby=")
        {
            return Err(RemediationError::NeedsReview {
                issue_id: issue.id.clone(),
                reason: "image already has an accessible name".into(),
            });
        }
        insert_attribute(source, "img", " alt=\"\"", &issue.id)?
    } else if issue.rule.contains("label")
        || issue.rule.contains("input")
        || issue.rule.contains("control")
    {
        let tag_name = ["input", "select", "textarea"]
            .iter()
            .find(|name| trimmed.contains(&format!("<{name}")))
            .copied()
            .ok_or_else(|| RemediationError::NeedsReview {
                issue_id: issue.id.clone(),
                reason: "control element is ambiguous".into(),
            })?;
        let tag = opening_tag(trimmed, tag_name).ok_or_else(|| RemediationError::NeedsReview {
            issue_id: issue.id.clone(),
            reason: "control source is incomplete".into(),
        })?;
        let compact_tag = compact(tag);
        if compact_tag.contains('[')
            || compact_tag.contains(':')
            || compact_tag.contains("v-")
            || compact_tag.contains("bind-")
            || compact_tag.contains("{{")
            || (compact_tag.contains('=') && compact_tag.contains('{'))
        {
            return Err(RemediationError::NeedsReview {
                issue_id: issue.id.clone(),
                reason: "control uses a dynamic binding and its label is ambiguous".into(),
            });
        }
        if compact_tag.contains("aria-label=")
            || tag.contains("aria-labelledby=")
            || (trimmed.contains("<label") && tag.contains("id="))
        {
            return Err(RemediationError::NeedsReview {
                issue_id: issue.id.clone(),
                reason: "control label association is ambiguous".into(),
            });
        }
        let name = attribute_value(tag, "id")
            .or_else(|| attribute_value(tag, "name"))
            .ok_or_else(|| RemediationError::NeedsReview {
                issue_id: issue.id.clone(),
                reason: "control has no stable name for an accessible label".into(),
            })?;
        insert_attribute(
            source,
            tag_name,
            &format!(" aria-label=\"{name}\""),
            &issue.id,
        )?
    } else if issue.rule.contains("button") && trimmed.contains("<button") {
        let tag = opening_tag(trimmed, "button").ok_or_else(|| RemediationError::NeedsReview {
            issue_id: issue.id.clone(),
            reason: "button source is incomplete".into(),
        })?;
        let body = button_body(trimmed).ok_or_else(|| RemediationError::NeedsReview {
            issue_id: issue.id.clone(),
            reason: "button closing tag is missing".into(),
        })?;
        let compact_tag = compact(tag);
        let compact_body = compact(body);
        if compact_tag.contains("aria-label=")
            || tag.contains("aria-labelledby=")
            || tag.contains("title=")
            || compact_body.contains('{')
            || compact_body.contains("{{")
            || compact_body.contains("v-")
            || compact_tag.contains('[')
            || compact_tag.contains(':')
            || compact_tag.contains("bind-")
            || compact_tag.contains("*ng")
        {
            return Err(RemediationError::NeedsReview {
                issue_id: issue.id.clone(),
                reason: "button name depends on dynamic or existing content".into(),
            });
        }
        if !body.trim().is_empty() {
            return Err(RemediationError::NeedsReview {
                issue_id: issue.id.clone(),
                reason: "button already has rendered content".into(),
            });
        }
        insert_attribute(source, "button", " aria-label=\"Submit\"", &issue.id)?
    } else {
        return Err(RemediationError::NeedsReview {
            issue_id: issue.id.clone(),
            reason: "pattern is not high confidence".into(),
        });
    };

    if diff == source || diff.trim().is_empty() {
        return Err(RemediationError::NeedsReview {
            issue_id: issue.id.clone(),
            reason: "remediation would not change the source".into(),
        });
    }
    let file = issue
        .source_locations
        .first()
        .ok_or_else(|| RemediationError::MissingSourceLocation {
            issue_id: issue.id.clone(),
        })?
        .file
        .clone();
    Ok(PatchProposal::new(
        format!("{}-proposal", issue.id),
        vec![issue.id.clone()],
        diff,
        vec![file],
        format!(
            "apply the high-confidence {} remediation for {:?}",
            issue.rule, framework
        ),
        vec!["verify rendered accessibility semantics".into()],
        vec!["run the focused accessibility test".into()],
        "removes the reported accessibility violation",
    ))
}

fn opening_tag<'a>(source: &'a str, name: &str) -> Option<&'a str> {
    source.split(&format!("<{name}")).nth(1)?.split('>').next()
}

fn insert_attribute(
    source: &str,
    name: &str,
    attribute: &str,
    issue_id: &str,
) -> Result<String, RemediationError> {
    let marker = format!("<{name}");
    let start = source
        .find(&marker)
        .ok_or_else(|| RemediationError::NeedsReview {
            issue_id: issue_id.into(),
            reason: "opening tag is missing".into(),
        })?;
    let end = opening_tag_end(source, start).ok_or_else(|| RemediationError::NeedsReview {
        issue_id: issue_id.into(),
        reason: "opening tag is incomplete".into(),
    })?;
    let at = attribute_insertion_point(source, start, end);
    Ok(format!("{}{}{}", &source[..at], attribute, &source[at..]))
}

/// Byte index of the `>` that closes the opening tag starting at `start`.
///
/// Scans for it rather than taking the first `>`, because a `>` inside a quoted
/// attribute value does not end the tag: `<img alt="a > b" src="x" />` would
/// otherwise be cut after `a `, and the attribute inserted into the middle of a
/// value.
///
/// Scanning bytes is safe here and lands on a character boundary: the markers
/// are ASCII, and a UTF-8 continuation byte is never an ASCII byte.
fn opening_tag_end(source: &str, start: usize) -> Option<usize> {
    let mut quote: Option<u8> = None;
    for (offset, byte) in source.as_bytes().iter().enumerate().skip(start) {
        match quote {
            Some(open) if *byte == open => quote = None,
            Some(_) => {}
            None => match *byte {
                b'"' | b'\'' => quote = Some(*byte),
                b'>' => return Some(offset),
                _ => {}
            },
        }
    }
    None
}

/// Where to insert a new attribute in the opening tag spanning `start..=end`.
///
/// Before a self-closing `/` and the whitespace that precedes it, so
/// `<img … />` becomes `<img … alt="" />`. Inserting before the `>` instead —
/// which is what this did originally — yields `<img … / alt="">`: the slash is
/// orphaned, the tag is no longer self-closing, and the result does not
/// compile. JSX and Vue self-close by default, so that hit the two most common
/// RGAA defects in exactly the sources this crate exists to patch.
///
/// A `/` inside a quoted value is not a self-closing marker. The last
/// non-whitespace byte before `>` is the only candidate, and in `src="/x.png">`
/// that is the closing quote, so the common case is safe. An *unquoted* value
/// ending in a slash is not distinguishable here, but JSX and Vue always quote
/// or brace their attribute values, so it cannot arise in the sources reached
/// through these adapters.
fn attribute_insertion_point(source: &str, start: usize, end: usize) -> usize {
    let inner = &source[start..end];
    match inner.rfind(|c: char| !c.is_whitespace()) {
        Some(offset) if inner.as_bytes()[offset] == b'/' => {
            start + inner[..offset].trim_end().len()
        }
        _ => end,
    }
}

fn attribute_value<'a>(tag: &'a str, name: &str) -> Option<&'a str> {
    tag.split_whitespace().find_map(|part| {
        part.strip_prefix(&format!("{name}=\""))
            .and_then(|value| value.strip_suffix('"'))
    })
}

fn compact(value: &str) -> String {
    value
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect()
}

fn button_body(source: &str) -> Option<&str> {
    let start = source.find("<button")?;
    let opening_end = source[start..].find('>')? + start + 1;
    let close = source[opening_end..].find("</button>")? + opening_end;
    Some(&source[opening_end..close])
}

pub fn detect_framework(source: &str) -> Option<Framework> {
    if source.contains("\"use client\"")
        || source.contains("'use client'")
        || source.contains("from \"next/")
        || source.contains("from 'next/")
    {
        return Some(Framework::Next);
    }
    if source.contains("from \"react\"")
        || source.contains("from 'react'")
        || source.contains("className=")
        || source.contains("import React")
    {
        return Some(Framework::React);
    }
    if source.contains("<template")
        || source.contains("v-model")
        || source.contains("<script setup")
    {
        return Some(Framework::Vue);
    }
    if source.contains("@Component")
        || source.contains("[ngModel]")
        || source.contains("[(ngModel)]")
        || source.contains("*ngIf")
    {
        return Some(Framework::Angular);
    }
    None
}

macro_rules! adapter_impl {
    ($type:ty, $framework:expr) => {
        impl FrameworkAdapter for $type {
            fn framework(&self) -> Framework {
                $framework
            }
            fn detect(&self, source: &str) -> Option<Framework> {
                (detect_framework(source) == Some($framework)).then_some($framework)
            }
            fn locate(&self, _source: &str, issue: &RemediationIssue) -> Vec<SourceLocation> {
                issue.source_locations.clone()
            }
            fn propose(
                &self,
                issue: &RemediationIssue,
                source: &str,
            ) -> Result<PatchProposal, RemediationError> {
                propose_for($framework, issue, source)
            }
        }
    };
}

adapter_impl!(ReactAdapter, Framework::React);
adapter_impl!(NextAdapter, Framework::Next);
adapter_impl!(VueAdapter, Framework::Vue);
adapter_impl!(AngularAdapter, Framework::Angular);

#[cfg(test)]
mod insert_attribute_tests {
    use super::insert_attribute;

    fn insert(source: &str, name: &str, attribute: &str) -> String {
        insert_attribute(source, name, attribute, "issue-1").expect("insertion succeeds")
    }

    /// The case that shipped broken: inserting before the opening tag's first
    /// `>` puts the attribute after a self-closing `/`, so the tag stops
    /// compiling. JSX and Vue self-close by default, and the two call sites
    /// this affects are the most common RGAA defects there.
    #[test]
    fn keeps_a_self_closing_jsx_tag_self_closing() {
        assert_eq!(
            insert(
                r#"<img className="logo" src="/logo.png" />"#,
                "img",
                r#" alt="""#
            ),
            r#"<img className="logo" src="/logo.png" alt="" />"#
        );
    }

    #[test]
    fn keeps_a_self_closing_input_self_closing() {
        assert_eq!(
            insert(
                r#"<input className="email-field" id="email" type="email" name="email" />"#,
                "input",
                r#" aria-label="email""#
            ),
            r#"<input className="email-field" id="email" type="email" name="email" aria-label="email" />"#
        );
    }

    #[test]
    fn handles_a_self_closing_tag_with_no_space_before_the_slash() {
        assert_eq!(insert("<img/>", "img", r#" alt="""#), r#"<img alt=""/>"#);
    }

    #[test]
    fn handles_whitespace_between_the_slash_and_the_bracket() {
        assert_eq!(
            insert(r#"<img src="a.png" / >"#, "img", r#" alt="""#),
            r#"<img src="a.png" alt="" / >"#
        );
    }

    /// Plain HTML does not self-close, and must keep behaving as before.
    #[test]
    fn appends_before_the_bracket_when_the_tag_does_not_self_close() {
        assert_eq!(
            insert(r#"<img class="logo" src="/logo.png">"#, "img", r#" alt="""#),
            r#"<img class="logo" src="/logo.png" alt="">"#
        );
    }

    #[test]
    fn leaves_a_non_void_element_untouched_apart_from_the_attribute() {
        assert_eq!(
            insert(
                r#"<button className="submit-btn" type="submit"></button>"#,
                "button",
                r#" aria-label="Submit""#
            ),
            r#"<button className="submit-btn" type="submit" aria-label="Submit"></button>"#
        );
    }

    /// A `/` inside a quoted value is not a self-closing marker. `src="/x.png"`
    /// is the overwhelmingly common shape, so getting this wrong would corrupt
    /// the attribute value rather than the tag.
    #[test]
    fn does_not_mistake_a_slash_inside_a_quoted_value_for_self_closing() {
        assert_eq!(
            insert(r#"<img src="/assets/logo.png">"#, "img", r#" alt="""#),
            r#"<img src="/assets/logo.png" alt="">"#
        );
    }

    /// A `>` inside a quoted value does not end the opening tag.
    #[test]
    fn does_not_end_the_tag_at_a_bracket_inside_a_quoted_value() {
        assert_eq!(
            insert(
                r#"<img alt="a > b" src="x.png" />"#,
                "img",
                r#" role="img""#
            ),
            r#"<img alt="a > b" src="x.png" role="img" />"#
        );
    }

    #[test]
    fn needs_review_when_the_opening_tag_is_absent() {
        let err = insert_attribute("<span></span>", "img", r#" alt="""#, "issue-1")
            .expect_err("no img tag");
        assert!(
            format!("{err:?}").contains("opening tag is missing"),
            "{err:?}"
        );
    }

    #[test]
    fn needs_review_when_the_opening_tag_never_closes() {
        let err = insert_attribute(r#"<img class="logo""#, "img", r#" alt="""#, "issue-1")
            .expect_err("unterminated tag");
        assert!(
            format!("{err:?}").contains("opening tag is incomplete"),
            "{err:?}"
        );
    }
}
