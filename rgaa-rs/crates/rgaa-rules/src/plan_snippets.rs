//! Single-page gap-fix snippets for criteria the engine plan assigns to the
//! deterministic engine but that had no mechanism (see
//! `docs/research/criteres-traites-vs-non-testes.md`).
//!
//! Every snippet here is **partial**: it may report a violation (which stands as
//! evidence) but a clean run never yields a criterion-level `Pass` — see
//! `GapFixRules::covers_whole_criterion`. Heuristics are written to prefer silence
//! over a false `Fail`.

pub(crate) const SNIPPETS: &[(&str, &str)] = &[
    // 8.1: no doctype in the document.
    (
        "8.1",
        r#"
        (() => {
            const bad = document.doctype ? 0 : 1;
            return JSON.stringify({ pass: bad === 0, details: bad ? 'page has no doctype' : 'doctype present', nodes: bad });
        })()
    "#,
    ),
    // 8.9: tags used only for presentation.
    (
        "8.9",
        r#"
        (() => {
            const n = document.querySelectorAll('font, center, basefont, big, strike, tt, blink, marquee').length;
            return JSON.stringify({ pass: n === 0, details: `${n} presentation-only element(s) (font, center, big, strike, tt, blink, marquee)`, nodes: n });
        })()
    "#,
    ),
    // 8.10: invalid dir value, or RTL script content with no dir anywhere.
    (
        "8.10",
        r#"
        (() => {
            let bad = 0;
            document.querySelectorAll('[dir]').forEach(el => {
                if (!['ltr', 'rtl', 'auto'].includes((el.getAttribute('dir') || '').toLowerCase())) bad++;
            });
            const rtl = /[֐-׿؀-ۿݐ-ݿ]/.test(document.body ? document.body.innerText : '');
            const anyDir = document.querySelector('[dir=rtl], [dir=auto], bdo, bdi') || document.documentElement.dir;
            if (rtl && !anyDir) bad++;
            return JSON.stringify({ pass: bad === 0, details: `${bad} invalid or missing reading-direction declaration(s)`, nodes: bad });
        })()
    "#,
    ),
    // 9.4: quotation written with guillemets as a whole paragraph, outside blockquote/q.
    (
        "9.4",
        r#"
        (() => {
            let bad = 0;
            document.querySelectorAll('p').forEach(p => {
                if (p.closest('blockquote, q')) return;
                const t = (p.textContent || '').trim();
                if (t.length > 40 && /^[«“"][\s\S]+[»”"]$/.test(t)) bad++;
            });
            return JSON.stringify({ pass: bad === 0, details: `${bad} quoted paragraph(s) not marked up with blockquote/q`, nodes: bad });
        })()
    "#,
    ),
    // 5.1: complex data table (spans / several header rows) without caption, summary or description.
    (
        "5.1",
        r#"
        (() => {
            let bad = 0;
            document.querySelectorAll('table').forEach(t => {
                const role = t.getAttribute('role');
                if (role === 'presentation' || role === 'none') return;
                const complex = t.querySelector('[rowspan], [colspan]') || t.querySelectorAll('thead tr').length > 1;
                if (!complex) return;
                const described = t.querySelector('caption') || t.hasAttribute('summary') || t.hasAttribute('aria-describedby') || t.hasAttribute('aria-label') || t.hasAttribute('aria-labelledby');
                const prev = t.previousElementSibling;
                if (!described && !(prev && /^(h[1-6]|p)$/i.test(prev.tagName) && /r[ée]sum[ée]|summary/i.test(prev.textContent))) bad++;
            });
            return JSON.stringify({ pass: bad === 0, details: `${bad} complex table(s) without summary/description`, nodes: bad });
        })()
    "#,
    ),
    // 5.8: layout table carrying data-table markup.
    (
        "5.8",
        r#"
        (() => {
            let bad = 0;
            document.querySelectorAll('table[role=presentation], table[role=none]').forEach(t => {
                if (t.querySelector('th, caption, thead, [headers], [scope]') || t.hasAttribute('summary')) bad++;
            });
            return JSON.stringify({ pass: bad === 0, details: `${bad} layout table(s) using data-table elements`, nodes: bad });
        })()
    "#,
    ),
    // 4.1 / 4.5 / 4.7 / 4.8 / 4.11: time-based media markup.
    (
        "4.1",
        r#"
        (() => {
            let bad = 0;
            document.querySelectorAll('video, audio').forEach(m => {
                const ctx = (m.closest('figure, section, div') || document.body).textContent || '';
                const hasTrack = m.querySelector('track[kind=captions], track[kind=subtitles], track[kind=descriptions]');
                if (!hasTrack && !/transcription|transcript|audiodescription|audio-description|sous-titr/i.test(ctx)) bad++;
            });
            return JSON.stringify({ pass: bad === 0, details: `${bad} media without transcript/captions/description nearby`, nodes: bad });
        })()
    "#,
    ),
    (
        "4.5",
        r#"
        (() => {
            let bad = 0;
            document.querySelectorAll('video').forEach(v => {
                const ctx = (v.closest('figure, section, div') || document.body).textContent || '';
                if (!v.querySelector('track[kind=descriptions]') && !/audiodescription|audio-description|audio description|version audiodécrite/i.test(ctx)) bad++;
            });
            return JSON.stringify({ pass: bad === 0, details: `${bad} video(s) with no audio description track or reference`, nodes: bad });
        })()
    "#,
    ),
    (
        "4.7",
        r#"
        (() => {
            let bad = 0;
            document.querySelectorAll('video, audio, object[type^="video"], object[type^="audio"], iframe[src*="youtube"], iframe[src*="vimeo"], iframe[src*="dailymotion"]').forEach(m => {
                const named = m.getAttribute('title') || m.getAttribute('aria-label') || m.getAttribute('aria-labelledby') || (m.closest('figure') && m.closest('figure').querySelector('figcaption'));
                if (!named) bad++;
            });
            return JSON.stringify({ pass: bad === 0, details: `${bad} media with no identifying title/caption`, nodes: bad });
        })()
    "#,
    ),
    (
        "4.8",
        r#"
        (() => {
            let bad = 0;
            document.querySelectorAll('object:not([type^="video"]):not([type^="audio"]), embed, canvas').forEach(m => {
                if (m.tagName === 'EMBED' && m.getAttribute('type') && /^(video|audio)/.test(m.getAttribute('type'))) return;
                const fallback = (m.textContent || '').trim() || m.getAttribute('aria-label') || m.getAttribute('aria-labelledby') || m.getAttribute('title') || m.getAttribute('role') === 'presentation';
                if (!fallback) bad++;
            });
            return JSON.stringify({ pass: bad === 0, details: `${bad} non-time-based media without alternative content`, nodes: bad });
        })()
    "#,
    ),
    (
        "4.11",
        r#"
        (() => {
            let bad = 0;
            document.querySelectorAll('video, audio').forEach(m => {
                if (m.hasAttribute('controls')) return;
                const scope = m.closest('figure, section, div') || document.body;
                if (!scope.querySelector('button, [role=button], input[type=button]')) bad++;
            });
            return JSON.stringify({ pass: bad === 0, details: `${bad} media without native controls or any control button`, nodes: bad });
        })()
    "#,
    ),
    // 7.4: context change triggered by a plain value/focus change.
    (
        "7.4",
        r#"
        (() => {
            let bad = 0;
            document.querySelectorAll('select[onchange], input[onchange], [onfocus]').forEach(el => {
                const code = (el.getAttribute('onchange') || '') + ' ' + (el.getAttribute('onfocus') || '');
                if (/submit\s*\(|location\s*[.=]|window\.open|\.href\s*=/.test(code)) bad++;
            });
            return JSON.stringify({ pass: bad === 0, details: `${bad} control(s) changing context on change/focus`, nodes: bad });
        })()
    "#,
    ),
    // 10.5: CSS colour declared without its counterpart (inline styles + same-origin sheets).
    (
        "10.5",
        r#"
        (() => {
            let bad = 0;
            const unpaired = s => {
                const fg = s.getPropertyValue('color');
                const bg = s.getPropertyValue('background-color') || s.getPropertyValue('background') || s.getPropertyValue('background-image');
                return (fg && !bg) || (!fg && s.getPropertyValue('background-color'));
            };
            document.querySelectorAll('[style]').forEach(el => { if (unpaired(el.style)) bad++; });
            for (const sheet of document.styleSheets) {
                let rules; try { rules = sheet.cssRules; } catch (e) { continue; }
                for (const r of rules || []) { if (r.style && r.selectorText && !r.selectorText.includes(':') && unpaired(r.style)) bad++; }
            }
            return JSON.stringify({ pass: bad === 0, details: `${bad} CSS declaration(s) setting text or background colour without the other`, nodes: bad });
        })()
    "#,
    ),
    // 10.7: focus indicator removed by CSS with no replacement.
    (
        "10.7",
        r#"
        (() => {
            let bad = 0;
            for (const sheet of document.styleSheets) {
                let rules; try { rules = sheet.cssRules; } catch (e) { continue; }
                for (const r of rules || []) {
                    if (!r.style || !r.selectorText || !/:focus(?!-visible\b.*outline)/.test(r.selectorText)) continue;
                    const o = (r.style.getPropertyValue('outline') || r.style.getPropertyValue('outline-style') || r.style.getPropertyValue('outline-width') || '').trim();
                    const removed = /^(none|0|0px)\b/.test(o);
                    const replaced = r.style.getPropertyValue('box-shadow') || r.style.getPropertyValue('border') || r.style.getPropertyValue('border-color') || r.style.getPropertyValue('background') || r.style.getPropertyValue('background-color') || r.style.getPropertyValue('text-decoration');
                    if (removed && !replaced) bad++;
                }
            }
            return JSON.stringify({ pass: bad === 0, details: `${bad} :focus rule(s) removing the outline with no replacement`, nodes: bad });
        })()
    "#,
    ),
    // 11.6: field group without a legend.
    (
        "11.6",
        r#"
        (() => {
            let bad = 0;
            document.querySelectorAll('fieldset').forEach(f => {
                const lg = f.querySelector(':scope > legend');
                if (!lg || !(lg.textContent || '').trim()) { if (!f.getAttribute('aria-label') && !f.getAttribute('aria-labelledby')) bad++; }
            });
            document.querySelectorAll('[role=group], [role=radiogroup]').forEach(g => {
                if (!g.getAttribute('aria-label') && !g.getAttribute('aria-labelledby')) bad++;
            });
            return JSON.stringify({ pass: bad === 0, details: `${bad} field group(s) without legend`, nodes: bad });
        })()
    "#,
    ),
    // 13.2: target=_blank link not announcing the new window.
    (
        "13.2",
        r#"
        (() => {
            let bad = 0;
            document.querySelectorAll('a[target=_blank], area[target=_blank]').forEach(a => {
                const label = [a.textContent, a.getAttribute('title'), a.getAttribute('aria-label'), ...[...a.querySelectorAll('img[alt], [aria-label]')].map(x => x.getAttribute('alt') || x.getAttribute('aria-label'))].join(' ');
                if (!/nouvelle fen[êe]tre|nouvel onglet|new (window|tab)|ouvre dans/i.test(label)) bad++;
            });
            return JSON.stringify({ pass: bad === 0, details: `${bad} link(s) opening a new window without warning`, nodes: bad });
        })()
    "#,
    ),
    // 13.5: emoticons / ASCII art outside an element that gives them a text alternative.
    (
        "13.5",
        r#"
        (() => {
            let bad = 0;
            const re = /(?:^|\s)(?:[:;=8][-^o]?[)(DPp\/\\|])(?=\s|$)|¯\\_\(ツ\)_\/¯|\^_\^|<3/;
            const walker = document.createTreeWalker(document.body || document, NodeFilter.SHOW_TEXT);
            let n;
            while ((n = walker.nextNode())) {
                const p = n.parentElement;
                if (!p || p.closest('script, style, code, pre, [aria-label], [title], abbr')) continue;
                if (re.test(n.nodeValue)) bad++;
            }
            return JSON.stringify({ pass: bad === 0, details: `${bad} emoticon/ASCII-art text node(s) without text alternative`, nodes: bad });
        })()
    "#,
    ),
];
