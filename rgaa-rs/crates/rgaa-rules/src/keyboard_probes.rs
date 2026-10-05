//! Keyboard-trap probes (spec `criteria-coverage-expansion` §7.1, issue #264).
//!
//! Obscura delivers `keydown` but implements no native Tab navigation (measured in
//! the keyboard prototype), so a trap cannot be found by pressing Tab and watching
//! `document.activeElement`. Instead, for each focusable element: give it focus,
//! send a **cancelable** `keydown` Tab, and read whether the page cancelled it. A
//! cancelled Tab is only a trap when focus then stays put *and* Escape does not
//! release it — a modal that cancels Tab to cycle focus, or that closes on Escape,
//! is legitimate.
//!
//! The probe is **partial** and can only `fail`: it never emits `pass`. Known limit,
//! deliberately undetected: a trap built by asynchronous refocus (`trap-refocus` in
//! the prototype) — so silence here is not evidence of conformance.

macro_rules! sweep {
    ($scope:literal, $what:literal) => {
        concat!(
            r#"
        (() => {
            const SEL = 'a[href], button, input:not([type=hidden]), select, textarea, summary, [tabindex], [contenteditable=""], [contenteditable=true]';
            const SCOPE = "#,
            $scope,
            r#";
            const name = e => e.tagName.toLowerCase() + (e.id ? '#' + e.id : '');
            const cancelled = (target, key) =>
                !target.dispatchEvent(new KeyboardEvent('keydown', { key, bubbles: true, cancelable: true }));
            const candidates = [...document.querySelectorAll(SEL)]
                .filter(e => !e.disabled && e.tabIndex >= 0 && SCOPE(e));
            const traps = [];
            for (const e of candidates) {
                e.focus();
                if (document.activeElement !== e) continue;
                if (!cancelled(e, 'Tab')) continue;
                if (document.activeElement !== e) continue;
                cancelled(e, 'Escape');
                if (document.activeElement === e && document.contains(e)) traps.push(name(e));
            }
            const details = traps.length
                ? `Tab is cancelled with no keyboard exit on "#,
            $what,
            r#": ${traps.slice(0, 5).join(', ')}`
                : `no cancelled-Tab trap among ${candidates.length} candidate(s)`;
            return JSON.stringify({ outcome: traps.length ? 'fail' : 'pass', pass: traps.length === 0, details, nodes: traps.length });
        })()
    "#
        )
    };
}

pub(crate) const SNIPPETS: &[(&str, &str)] = &[
    // 12.9: any focusable element.
    ("12.9", sweep!("(() => true)", "focusable elements")),
    // 4.12: the same sweep, restricted to non-temporal media and what they contain.
    (
        "4.12",
        sweep!(
            "(e => e.matches('object, embed, canvas, svg, [role=application]') || !!e.closest('object, embed, canvas, svg, [role=application]'))",
            "non-temporal media"
        ),
    ),
];

#[cfg(test)]
mod tests {
    use super::SNIPPETS;

    #[test]
    fn probes_send_cancelable_tab_and_escape_and_never_pass_unconditionally() {
        for (id, snippet) in SNIPPETS {
            assert!(snippet.contains("cancelable: true"), "{id}");
            assert!(
                snippet.contains("'Tab'") && snippet.contains("'Escape'"),
                "{id}"
            );
            assert!(
                snippet.contains("outcome: traps.length ? 'fail' : 'pass'"),
                "{id}"
            );
        }
    }

    #[test]
    fn media_probe_is_scoped_to_non_temporal_media() {
        let media = SNIPPETS.iter().find(|(id, _)| *id == "4.12").unwrap().1;
        let all = SNIPPETS.iter().find(|(id, _)| *id == "12.9").unwrap().1;
        assert!(media.contains("canvas") && media.contains("non-temporal media"));
        assert!(!all.contains("canvas"));
    }
}
