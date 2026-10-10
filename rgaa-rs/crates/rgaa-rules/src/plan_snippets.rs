//! Single-page gap-fix snippets for criteria the engine plan assigns to the
//! deterministic engine but that had no mechanism (see
//! `docs/research/criteres-traites-vs-non-testes.md`).
//!
//! Coverage is declared per criterion in `mechanisms.toml`; clean partial probes
//! cannot produce a criterion-level `Pass` (`GapFixRules::covers_whole_criterion`).
//! Triage probes return `review` on clean or ambiguous runs so the report
//! distinguishes an executed control from a criterion that was not examined.

pub(crate) const SNIPPETS: &[(&str, &str)] = &[
    // 8.1: document type declaration. A present doctype does not prove that the
    // source conforms to the HTML syntax rules, so a clean result needs review.
    (
        "8.1",
        r#"
        (() => {
            const bad = document.doctype ? 0 : 1;
            return JSON.stringify(bad
                ? { outcome: 'fail', details: 'page has no document type declaration', nodes: 1 }
                : { outcome: 'review', reason: 'source syntax validation is not available in the rendered DOM', details: `document type declaration present: ${document.doctype.name}`, nodes: 0 });
        })()
    "#,
    ),
    // 8.9: tags used only for presentation.
    (
        "8.9",
        r#"
        (() => {
            const legacy = [...document.querySelectorAll('font, center, basefont, big, strike, tt, blink, marquee')];
            const ambiguous = [...document.querySelectorAll('b, i, br')];
            if (legacy.length) return JSON.stringify({ outcome: 'fail', details: `${legacy.length} obsolete presentation element(s) found`, nodes: legacy.length });
            return JSON.stringify({ outcome: 'review', reason: 'visual intent cannot be inferred from markup alone', details: `${ambiguous.length} b/i/br element(s) need a presentation-use check; no obsolete presentation element found`, nodes: ambiguous.length });
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
            const directionNodes = document.querySelectorAll('[dir], bdo, bdi').length;
            if (bad) return JSON.stringify({ outcome: 'fail', details: `${bad} invalid dir attribute value(s)`, nodes: bad });
            return JSON.stringify({ outcome: 'review', reason: 'language and reading direction require contextual review', details: `RTL characters ${rtl ? 'detected' : 'not detected'}; ${directionNodes} explicit direction marker(s); computed direction and local text runs need review`, nodes: directionNodes });
        })()
    "#,
    ),
    // 9.4: quotation written with guillemets as a whole paragraph, outside blockquote/q.
    (
        "9.4",
        r#"
        (() => {
            let candidates = 0;
            document.querySelectorAll('p').forEach(p => {
                if (p.closest('blockquote, q')) return;
                const t = (p.textContent || '').trim();
                if (t.length > 40 && /^[«“"][\s\S]+[»”"]$/.test(t)) candidates++;
            });
            const semantic = document.querySelectorAll('blockquote, q').length;
            return JSON.stringify({ outcome: 'review', reason: 'quotation meaning cannot be established from punctuation alone', details: `${candidates} paragraph(s) look quoted by punctuation; ${semantic} semantic q/blockquote element(s) found`, nodes: candidates + semantic });
        })()
    "#,
    ),
    // 1.7: detailed image descriptions require a relevance judgment.
    (
        "1.7",
        r#"
        (() => {
            const images = [...document.querySelectorAll('img, input[type="image"], svg[role="img"]')];
            const described = images.filter(el => el.hasAttribute('longdesc') || el.hasAttribute('aria-describedby') || el.closest('figure')?.querySelector('figcaption'));
            return JSON.stringify({ outcome: 'review', reason: 'the relevance of a detailed image description is a semantic judgment', details: `${images.length} image(s) inspected; ${described.length} with a long description, description reference or figure caption candidate`, nodes: described.length });
        })()
    "#,
    ),
    (
        "7.2",
        r#"
        (() => {
            const scripts = [...document.querySelectorAll('script:not([type="application/ld+json"]), [onclick], [onchange], [oninput]')];
            const alternatives = document.querySelectorAll('noscript, [aria-describedby], [data-script-alternative]');
            return JSON.stringify({ outcome: 'review', reason: 'the relevance and functional equivalence of a script alternative require contextual review', details: `${scripts.length} script/handler candidate(s); ${alternatives.length} alternative or description candidate(s)`, nodes: scripts.length });
        })()
    "#,
    ),
    // 12.2 and 12.5: collect per-page candidates; consistency is compared at site scope.
    (
        "12.2",
        r#"
        (() => {
            const nav = [...document.querySelectorAll('header, nav, [role="navigation"]')];
            return JSON.stringify({ outcome: 'review', reason: 'menu and navigation placement must be compared across the whole page set and visually confirmed', details: `${nav.length} navigation/header region(s) observed on this page`, nodes: nav.length });
        })()
    "#,
    ),
    (
        "12.5",
        r#"
        (() => {
            const search = [...document.querySelectorAll('form[role="search"], form input[type="search"], input[type="search"], [role="search"]')];
            return JSON.stringify({ outcome: 'review', reason: 'search availability and access must be compared across the page set and keyboard-tested', details: `${search.length} internal search candidate(s) observed on this page`, nodes: search.length });
        })()
    "#,
    ),
    (
        "7.3",
        r#"
        (() => {
            const candidates = [...document.querySelectorAll('[onclick], [onmousedown], [role="button"], [role="link"], [role="menuitem"], [role="tab"], [role="checkbox"], [role="switch"], [role="slider"]')];
            const suspicious = candidates.filter(el => !el.matches('a[href], button, input, select, textarea') && (el.tabIndex < 0 || !el.hasAttribute('role')));
            return JSON.stringify({ outcome: 'review', reason: 'keyboard activation and assistive-technology behavior require interaction testing', details: `${candidates.length} scripted/custom interactive candidate(s), ${suspicious.length} without an obvious keyboard entry point`, nodes: suspicious.length });
        })()
    "#,
    ),
    // 10.4: 200% text resize. The rendered snapshot cannot reliably emulate user zoom.
    (
        "10.4",
        r#"
        (() => {
            const meta = document.querySelector('meta[name="viewport"]');
            const content = (meta && meta.content || '').toLowerCase();
            const directives = new Map(content.split(/[,;]/).map(part => {
                const [key, ...value] = part.split('=');
                return [key.trim(), value.join('=').trim()];
            }));
            const limit = directives.get('maximum-scale');
            const maximumScale = limit && /^(?:\d+(?:\.\d*)?|\.\d+)$/.test(limit) ? Number(limit) : NaN;
            const blocked = ['no', '0'].includes(directives.get('user-scalable')) || (Number.isFinite(maximumScale) && maximumScale < 2);
            if (blocked) return JSON.stringify({ outcome: 'fail', details: `viewport disables or restricts user scaling: ${content}`, nodes: 1 });
            const smallText = [...document.querySelectorAll('body *')].filter(el => {
                const s = getComputedStyle(el); return s.display !== 'none' && s.visibility !== 'hidden' && parseFloat(s.fontSize) <= 12;
            }).length;
            return JSON.stringify({ outcome: 'review', reason: 'actual 200% text resizing and resulting content loss were not emulated', details: `no restrictive viewport directive found; ${smallText} visible element(s) use text at or below 12px`, nodes: smallText });
        })()
    "#,
    ),
    // 10.11: reflow at 320 CSS px requires a viewport resize and exemption review.
    (
        "10.11",
        r#"
        (() => {
            const overflow = document.documentElement.scrollWidth > document.documentElement.clientWidth;
            const wide = [...document.querySelectorAll('table, video, canvas, pre, iframe')].filter(el => el.getBoundingClientRect().width > document.documentElement.clientWidth).length;
            return JSON.stringify({ outcome: 'review', reason: 'the page was not resized to 320 CSS px and two-dimensional content exemptions need review', details: `current viewport ${document.documentElement.clientWidth}px; horizontal overflow ${overflow ? 'present' : 'not observed'}; ${wide} potentially wide table/media/code/frame element(s)`, nodes: (overflow ? 1 : 0) + wide });
        })()
    "#,
    ),
    // 13.1: detect time-limit and redirect clues; server/session timing needs live testing.
    (
        "13.1",
        r#"
        (() => {
            const meta = [...document.querySelectorAll('meta[http-equiv="refresh" i]')];
            const text = (document.body && document.body.innerText || '').slice(0, 200000);
            const words = /\b(?:session|session expires|time(?:out|r limit)|countdown|redirect|expire|expiration|déconnexion|délai|compte à rebours|expiration)\b/i.test(text);
            const scripts = [...document.scripts].filter(s => /setTimeout|setInterval|location\.(?:href|assign|replace)|logout|expire/i.test(s.textContent || '')).length;
            return JSON.stringify({ outcome: 'review', reason: 'server-side/session time limits and available extensions cannot be determined from a post-load snapshot', details: `${meta.length} refresh directive(s), ${scripts} timer/redirect-like inline script(s), time-limit wording ${words ? 'detected' : 'not detected'}`, nodes: meta.length + scripts + (words ? 1 : 0) });
        })()
    "#,
    ),
    // 13.7: flash frequency requires temporal frame sampling and luminance analysis.
    (
        "13.7",
        r#"
        (() => {
            const media = [...document.querySelectorAll('video, canvas, svg animate, svg animateTransform')];
            const animatedImages = [...document.querySelectorAll('img')].filter(el => /\.gif(?:[?#]|$)/i.test(el.currentSrc || el.src));
            const css = [...document.querySelectorAll('*')].filter(el => {
                const s = getComputedStyle(el); return s.animationName !== 'none' || s.transitionDuration.split(',').some(v => parseFloat(v) > 0);
            }).length;
            return JSON.stringify({ outcome: 'review', reason: 'flash frequency and affected screen area require temporal frame sampling', details: `${animatedImages.length} GIF(s), ${media.length} video/canvas/SVG animation candidate(s), ${css} element(s) with CSS animation/transition`, nodes: animatedImages.length + media.length + css });
        })()
    "#,
    ),
    // 13.8: identify movement/auto-start candidates; duration and pause controls need interaction.
    (
        "13.8",
        r#"
        (() => {
            const moving = [...document.querySelectorAll('body *')].filter(el => {
                const s = getComputedStyle(el), rect = el.getBoundingClientRect();
                if (s.display === 'none' || s.visibility === 'hidden' || rect.width <= 0 || rect.height <= 0) return false;
                return el.matches('marquee, blink, video[autoplay], audio[autoplay], svg animate, svg animateTransform') || (el.tagName === 'IMG' && /\.gif(?:[?#]|$)/i.test(el.currentSrc || el.src)) || s.animationName !== 'none';
            });
            const controls = [...document.querySelectorAll('button, [role="button"], input[type="button"]')].filter(el => /pause|stop|arr[êe]ter|suspend/i.test(el.innerText || el.getAttribute('aria-label') || el.value || '')).length;
            return JSON.stringify({ outcome: 'review', reason: 'movement duration, auto-start behavior and whether pause controls work require timed interaction', details: `${moving.length} movement/animation candidate(s), ${controls} pause/stop control candidate(s)`, nodes: moving.length + controls });
        })()
    "#,
    ),
    // 13.9: rotating the viewport and checking equivalent content requires CDP emulation.
    (
        "13.9",
        r#"
        (() => {
            const lock = [...document.styleSheets].some(sheet => {
                try { return [...sheet.cssRules].some(rule => /orientation\s*:\s*portrait/i.test(rule.conditionText || rule.cssText || '')); } catch (_) { return false; }
            });
            const orientationScripts = [...document.scripts].filter(s => /screen\.orientation\.lock|orientationchange|matchMedia\s*\([^)]*orientation/i.test(s.textContent || '')).length;
            return JSON.stringify({ outcome: 'review', reason: 'portrait and landscape rendering/functionality were not compared in separate viewport states', details: `portrait-specific media rule ${lock ? 'detected' : 'not detected'}; ${orientationScripts} orientation-related inline script(s)`, nodes: (lock ? 1 : 0) + orientationScripts });
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
                const lg = Array.from(f.children).find(child => child.tagName === 'LEGEND');
                if (!lg || !(lg.textContent || '').trim()) { if (!f.getAttribute('aria-label') && !f.getAttribute('aria-labelledby')) bad++; }
            });
            document.querySelectorAll('[role=group], [role=radiogroup]').forEach(g => {
                if (!g.getAttribute('aria-label') && !g.getAttribute('aria-labelledby')) bad++;
            });
            return JSON.stringify({ pass: bad === 0, details: `${bad} field group(s) without legend`, nodes: bad });
        })()
    "#,
    ),
    // 13.2: inspect likely window-opening paths; triggering context is runtime behavior.
    (
        "13.2",
        r#"
        (() => {
            const targets = document.querySelectorAll('a[target], area[target]').length;
            const scripts = [...document.scripts].filter(s => /window\.open\s*\(/i.test(s.textContent || '')).length;
            return JSON.stringify({ outcome: 'review', reason: 'a static DOM scan cannot establish whether a new context opens without a user action', details: `${targets} explicit link/area target(s), ${scripts} inline script(s) mention window.open; trigger timing requires interaction testing`, nodes: targets + scripts });
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
    // 1.6: identify images that may need a long description. Complexity and
    // sufficiency are semantic; this probe deliberately returns review only.
    (
        "1.6",
        r#"
        (() => {
            const candidates = [...document.querySelectorAll('img, svg[role="img"], [role="img"]')]
                .filter(el => /graph|chart|diagram|map|plan|infograph|sch[eé]ma/i.test(
                    [el.getAttribute('alt'), el.getAttribute('aria-label'), el.getAttribute('title'), el.getAttribute('src')].join(' ')
                ));
            const linked = candidates.filter(el => el.hasAttribute('longdesc') || el.hasAttribute('aria-describedby') || el.closest('a[href]'));
            return JSON.stringify({
                pass: false,
                outcome: 'review',
                details: `${candidates.length} complex-image candidate(s); ${linked.length} have a description link/target`,
                nodes: candidates.length,
                reason: 'La nécessité et la pertinence de la description détaillée demandent une vérification humaine'
            });
        })()
    "#,
    ),
    // 3.3: sample solid foreground/background pairs from rendered SVG parts.
    // Gradients, images, masks and non-RGB colours remain review items.
    (
        "3.3",
        r#"
        (() => {
            const parse = value => {
                const hex = value.match(/^#([0-9a-f]{3}|[0-9a-f]{6})$/i);
                if (hex) {
                    const digits = hex[1].length === 3
                        ? hex[1].split('').map(c => c + c).join('')
                        : hex[1];
                    return [0, 2, 4].map(i => parseInt(digits.slice(i, i + 2), 16));
                }
                const m = value.match(/^rgba?\((\d+(?:\.\d+)?),\s*(\d+(?:\.\d+)?),\s*(\d+(?:\.\d+)?)(?:,\s*(\d+(?:\.\d+)?))?/i);
                if (!m || (m[4] !== undefined && Number(m[4]) < .1)) return null;
                return m.slice(1, 4).map(Number);
            };
            const lum = rgb => {
                const c = rgb.map(v => { v /= 255; return v <= .04045 ? v / 12.92 : ((v + .055) / 1.055) ** 2.4; });
                return .2126 * c[0] + .7152 * c[1] + .0722 * c[2];
            };
            const graphics = [...document.querySelectorAll('svg, canvas, img[usemap], [role="img"]')]
                .filter(el => { const r = el.getBoundingClientRect(); return r.width > 0 && r.height > 0; });
            let sampled = 0, low = 0, unknown = 0;
            for (const graphic of graphics) {
                if (graphic.tagName.toLowerCase() === 'svg') {
                    for (const part of graphic.querySelectorAll('path, rect, circle, ellipse, polygon, polyline, line')) {
                        const style = getComputedStyle(part);
                        const fg = parse(style.fill) || parse(style.stroke) || parse(part.getAttribute('fill') || '') || parse(part.getAttribute('stroke') || '');
                        const bg = parse(getComputedStyle(graphic).backgroundColor) || parse(getComputedStyle(graphic.parentElement || graphic).backgroundColor) || parse(graphic.getAttribute('data-rgaa-background') || '');
                        if (!fg || !bg || style.fill.includes('url(') || style.filter !== 'none') { unknown++; continue; }
                        const a = lum(fg), b = lum(bg), ratio = (Math.max(a, b) + .05) / (Math.min(a, b) + .05);
                        sampled++;
                        if (ratio < 3) low++;
                    }
                } else { unknown++; }
            }
            if (low) return JSON.stringify({ pass: false, outcome: 'review', details: `${low} sampled graphic pair(s) below 3:1; ${sampled} pair(s) measured`, nodes: low,
                reason: 'Mesure candidate à confirmer dans le rendu réel et selon le rôle informatif du graphique' });
            return JSON.stringify({ pass: false, outcome: 'review', details: `${sampled} solid graphic pair(s) sampled; ${unknown} pair(s) need visual inspection`, nodes: graphics.length,
                reason: graphics.length ? 'Les gradients, images, fonds hérités et la fonction informative exigent un contrôle visuel' : 'Aucun élément graphique mesurable; vérifier si la page comporte des éléments graphiques informatifs' });
        })()
    "#,
    ),
    // 8.7: capture explicit language-change markup and likely foreign text
    // as review evidence; a browser-side heuristic cannot establish language.
    (
        "8.7",
        r#"
        (() => {
            const root = (document.documentElement.lang || '').toLowerCase();
            const declared = [...document.querySelectorAll('[lang]')]
                .filter(el => (el.getAttribute('lang') || '').toLowerCase() !== root);
            const foreignWords = /\b(the|with|for|and|bonjour|merci|gracias|hello|please|welcome)\b/i;
            const walker = document.createTreeWalker(document.body || document, NodeFilter.SHOW_TEXT);
            let n, candidates = 0;
            while ((n = walker.nextNode())) {
                if (n.parentElement?.closest('[lang], script, style, code, pre')) continue;
                if (foreignWords.test(n.nodeValue || '')) candidates++;
            }
            return JSON.stringify({ pass: false, outcome: 'review', details: `${declared.length} explicit language-change element(s), ${candidates} untagged language candidate(s)`, nodes: declared.length + candidates,
                reason: 'La langue réelle des passages ne peut pas être déterminée de façon fiable par une sonde statique' });
        })()
    "#,
    ),
    // 11.3: compare repeated labels only when field purpose has a strong key.
    // Similar-purpose inference remains review when it cannot be established.
    (
        "11.3",
        r#"
        (() => {
            const norm = s => (s || '').replace(/\s+/g, ' ').trim().toLocaleLowerCase();
            const groups = new Map();
            let reviewed = 0;
            for (const field of document.querySelectorAll('input:not([type=hidden]), select, textarea')) {
                const purpose = norm(field.getAttribute('autocomplete')) || norm(field.getAttribute('name'));
                const label = norm([...(field.labels || [])].map(el => el.innerText || el.textContent || '').join(' ') || field.getAttribute('aria-label'));
                if (!purpose || !label) { reviewed++; continue; }
                const labels = groups.get(purpose) || new Set();
                labels.add(label);
                groups.set(purpose, labels);
            }
            const inconsistent = [...groups.values()].filter(labels => labels.size > 1).length;
            if (inconsistent) return JSON.stringify({ pass: false, outcome: 'fail', details: `${inconsistent} repeated field-purpose group(s) use inconsistent labels`, nodes: inconsistent });
            return JSON.stringify({ pass: false, outcome: 'review', details: `${groups.size} confidently keyed field-purpose group(s), ${reviewed} unclassified field(s)`, nodes: groups.size + reviewed,
                reason: 'Confirmer que les champs regroupés ont bien la même fonction et que les libellés restent cohérents dans l’ensemble du site' });
        })()
    "#,
    ),
    // 11.8: inventory native option groups; semantic grouping needs review.
    (
        "11.8",
        r#"
        (() => {
            const lists = [...document.querySelectorAll('select')];
            const grouped = lists.filter(select => select.querySelector('optgroup'));
            const candidates = lists.filter(select => select.querySelectorAll('option').length > 5 && !select.querySelector('optgroup'));
            return JSON.stringify({ pass: false, outcome: 'review', details: `${grouped.length} select list(s) use optgroup; ${candidates.length} larger ungrouped list(s) need review`, nodes: grouped.length + candidates.length,
                reason: 'La pertinence des regroupements dépend des relations de sens entre les options' });
        })()
    "#,
    ),
    // 13.3: inventory document downloads from URL extension, download attr or
    // declared MIME type. Document accessibility itself is not inferred.
    (
        "13.3",
        r#"
        (() => {
            const docs = [...document.querySelectorAll('a[href], area[href]')].filter(link => {
                const href = link.href || '';
                const type = link.getAttribute('type') || '';
                return link.hasAttribute('download') || /\.(pdf|docx?|xlsx?|pptx?|odt|ods|odp)(?:$|[?#])/i.test(href) || /application\/(pdf|msword|vnd\.|vnd\.oasis)/i.test(type);
            });
            return JSON.stringify({ pass: false, outcome: 'review', details: `${docs.length} downloadable document link(s) inventoried`, nodes: docs.length,
                reason: 'Le balisage HTML ne permet pas de vérifier l’accessibilité du document téléchargé' });
        })()
    "#,
    ),
    // 4.12: non-time-based interactive graphics without keyboard semantics.
    (
        "4.12",
        crate::keyboard_probes::with_keyboard_sweep!(
            "(e => e.matches('object, embed, canvas, svg, [role=application]') || !!e.closest('object, embed, canvas, svg, [role=application]'))",
            "non-temporal media",
        r#"
        (() => {
            const candidates = [...document.querySelectorAll('canvas, map area, svg [onclick], [onclick], [onpointerdown], [onmousedown]')];
            const unusable = candidates.filter(el => el.hasAttribute('onclick') || el.hasAttribute('onpointerdown') || el.hasAttribute('onmousedown'))
                .filter(el => el.tabIndex < 0 && !el.hasAttribute('onkeydown') && !el.hasAttribute('onkeyup'));
            if (unusable.length) return JSON.stringify({ pass:false, outcome:'fail', details:`${unusable.length} pointer-activated non-time-media control(s) have no keyboard focus or key handler`, nodes:unusable.length });
            return JSON.stringify({ pass:false, outcome:'review', details:`${candidates.length} non-time-media interaction candidate(s) found`, nodes:candidates.length,
                reason:'L’équivalence réelle au clavier et au pointeur doit être vérifiée par interaction contrôlée' });
        })()
    "#
        ),
    ),
    // 4.13: expose media roles, names and native controls for a follow-up AT review.
    (
        "4.13",
        r#"
        (() => {
            const media = [...document.querySelectorAll('audio, video, canvas, object, embed, iframe, svg[role="img"], [role="img"]')];
            const unnamed = media.filter(el => !(el.getAttribute('aria-label') || el.getAttribute('aria-labelledby') || el.getAttribute('title') || el.querySelector('track')?.label || '').trim());
            return JSON.stringify({ pass:false, outcome:'review', details:`${media.length} media element(s) inventoried; ${unnamed.length} have no explicit name/title`, nodes:media.length,
                reason:'La compatibilité avec les technologies d’assistance doit être validée avec les rôles, noms et états annoncés' });
        })()
    "#,
    ),
    // 10.9: record images, icons, charts and CSS-generated shape candidates.
    (
        "10.9",
        r#"
        (() => {
            const visual = [...document.querySelectorAll('svg, canvas, img, [role="img"], [class*="icon" i], [class*="arrow" i]')]
                .filter(el => { const r=el.getBoundingClientRect(); return r.width>0 && r.height>0; });
            return JSON.stringify({ pass:false, outcome:'review', details:`${visual.length} visual-shape/position candidate(s) require text-alternative comparison`, nodes:visual.length,
                reason:'La relation entre forme, taille, position et information textuelle est sémantique' });
        })()
    "#,
    ),
    // 10.12: temporarily apply RGAA text spacing, measure visible overflow, and
    // always remove the override before returning to the audit session.
    (
        "10.12",
        r#"
        (() => {
            const style = document.createElement('style'); style.dataset.rgaaProbe='10.12';
            style.textContent='* { line-height: 1.5 !important; letter-spacing: .12em !important; word-spacing: .16em !important; } p { margin-block: 2em !important; }';
            const before = document.documentElement.scrollWidth;
            document.head.appendChild(style);
            let overflow = [];
            try {
                overflow = [...document.querySelectorAll('body *')].filter(el => {
                    const r=el.getBoundingClientRect(), s=getComputedStyle(el);
                    const clippedX = ['hidden', 'clip'].includes(s.overflowX) && el.scrollWidth>el.clientWidth+2;
                    const clippedY = ['hidden', 'clip'].includes(s.overflowY) && el.scrollHeight>el.clientHeight+2;
                    return r.width>0 && r.height>0 && (clippedX || clippedY);
                });
                const after = document.documentElement.scrollWidth;
                if (overflow.length) return JSON.stringify({ pass:false, outcome:'review', details:`${overflow.length} possible clipped element(s) after text-spacing override; page width ${before}px → ${after}px`, nodes:overflow.length,
                    reason:'Confirmer visuellement que le contenu ou une fonction est réellement perdu après application de l’espacement RGAA' });
                return JSON.stringify({ pass:false, outcome:'review', details:`Text-spacing override applied and rolled back; document width ${before}px → ${after}px`, nodes:0,
                    reason:'L’absence de débordement DOM ne détecte pas toutes les pertes visuelles ou fonctionnelles' });
            } finally { style.remove(); }
        })()
    "#,
    ),
    // 10.13: compare hover/focus rules and identify hidden supplemental content.
    (
        "10.13",
        r#"
        (() => {
            const hover = new Set(), focus = new Set();
            for (const sheet of document.styleSheets) { let rules; try { rules=sheet.cssRules; } catch (_) { continue; }
                for (const rule of rules || []) if (rule.selectorText) {
                    if (/:hover/.test(rule.selectorText)) hover.add(rule.selectorText.replace(/:hover[^, ]*/g,'').trim());
                    if (/:focus|:focus-within/.test(rule.selectorText)) focus.add(rule.selectorText.replace(/:focus(?:-within|-visible)?/g,'').trim());
                }
            }
            const candidates = [...hover].filter(sel => { try { return !!document.querySelector(sel); } catch (_) { return false; } });
            const missing = candidates.filter(sel => ![...focus].some(f => f && (f===sel || f.includes(sel) || sel.includes(f))));
            return JSON.stringify({ pass:false, outcome:'review', details:`${candidates.length} hover disclosure selector(s), ${missing.length} without a comparable focus selector`, nodes:candidates.length,
                reason:'Vérifier au clavier l’apparition, le maintien, le déplacement du pointeur et la fermeture du contenu' });
        })()
    "#,
    ),
    // 11.11: inspect native validity and nearby correction guidance without
    // submitting or dispatching invalid events.
    (
        "11.11",
        r#"
        (() => {
            const fields=[...document.querySelectorAll('input:not([type=hidden]), select, textarea')].filter(el=>el.willValidate);
            const invalid=fields.filter(el=>!el.validity.valid);
            const unguided=invalid.filter(el=>{
                const ids=(el.getAttribute('aria-describedby')||'').split(/\s+/).filter(Boolean);
                const help=ids.map(id=>document.getElementById(id)?.textContent||'').join(' ')+' '+(el.title||'');
                return help.trim().length<8;
            });
            return JSON.stringify({ pass:false, outcome:'review', details:`${fields.length} validated field(s), ${invalid.length} currently invalid, ${unguided.length} without identified correction guidance`, nodes:invalid.length,
                reason:'La pertinence des suggestions dépend de la règle métier et du contexte de saisie' });
        })()
    "#,
    ),
    // 12.8: geometry and DOM order are evidence only; the Obscura Tab probe
    // supplies actual keyboard focus sequence in the audit pipeline.
    (
        "12.8",
        r#"
        (() => {
            const items=[...document.querySelectorAll('a[href],button,input:not([type=hidden]),select,textarea,[tabindex]')].filter(el=>el.tabIndex>=0&&!el.disabled);
            const unusual=items.filter((el,i)=>el.tabIndex>0 || (i>0 && el.compareDocumentPosition(items[i-1]) & Node.DOCUMENT_POSITION_FOLLOWING));
            return JSON.stringify({ pass:false, outcome:'review', details:`${items.length} sequentially focusable element(s), ${unusual.length} positive-tabindex/order candidate(s)`, nodes:items.length,
                reason:'Comparer l’ordre réel de Tab avec l’ordre visuel et le sens de lecture' });
        })()
    "#,
    ),
    // 12.9: keyboard-trap conclusions come from the bounded CDP Tab traversal.
    (
        "12.9",
        crate::keyboard_probes::with_keyboard_sweep!(
            "(() => true)",
            "focusable elements",
        r#"
        (() => JSON.stringify({ pass:false, outcome:'review', details:'Le parcours clavier CDP est exécuté séparément par Obscura', nodes:0,
            reason:'Une absence de piège sur un parcours borné ne prouve pas l’absence de piège dans tous les états' }))()
    "#
        ),
    ),
    // 12.10: enumerate author-declared single-key shortcuts; handlers need runtime review.
    (
        "12.10",
        r#"
        (() => {
            const items=[...document.querySelectorAll('[accesskey]')].map(el=>({key:el.getAttribute('accesskey'),tag:el.tagName.toLowerCase(),name:(el.innerText||el.getAttribute('aria-label')||'').trim().slice(0,80)}));
            return JSON.stringify({ pass:false, outcome:'review', details:`${items.length} accesskey declaration(s) inventoried`, nodes:items.length,
                reason:'Les raccourcis enregistrés en JavaScript ne sont pas tous introspectables sans déclencher leur action' });
        })()
    "#,
    ),
    // 12.11: hover/focus/activation disclosures overlap 10.13; this criterion
    // additionally needs keyboard reachability and dismissal evidence.
    (
        "12.11",
        r#"
        (() => {
            const triggers=[...document.querySelectorAll('[aria-haspopup], [aria-expanded], [title], [data-tooltip], [role="tooltip"]')];
            return JSON.stringify({ pass:false, outcome:'review', details:`${triggers.length} supplementary-content trigger/target candidate(s)`, nodes:triggers.length,
                reason:'Tester au clavier l’atteignabilité, le déplacement vers le contenu et sa fermeture' });
        })()
    "#,
    ),
    // 13.10: detect declarations of pointer/touch gestures without activating them.
    (
        "13.10",
        r#"
        (() => {
            const targets=[...document.querySelectorAll('[ontouchstart],[ontouchmove],[onpointerdown],[onpointermove],[ondblclick],[data-gesture]')];
            return JSON.stringify({ pass:false, outcome:'review', details:`${targets.length} gesture-handler candidate(s) found without activation`, nodes:targets.length,
                reason:'Vérifier qu’un geste simple permet la même fonction qu’un geste complexe' });
        })()
    "#,
    ),
    // 13.11: static handler names cannot establish cancellation, so identify
    // likely pointer-action controls for manual/runtime review.
    (
        "13.11",
        r#"
        (() => {
            const targets=[...document.querySelectorAll('[onclick],[onpointerdown],[onmousedown],button,a[href]')];
            return JSON.stringify({ pass:false, outcome:'review', details:`${targets.length} single-point pointer-action candidate(s)`, nodes:targets.length,
                reason:'Vérifier que l’action n’est validée qu’au relâchement et qu’elle peut être annulée ou inversée' });
        })()
    "#,
    ),
    // 13.12: inspect motion API references and device-motion affordances without
    // requesting sensor permission or moving the device.
    (
        "13.12",
        r#"
        (() => {
            const references=(document.documentElement.outerHTML.match(/DeviceMotionEvent|DeviceOrientationEvent|deviceorientation|devicemotion|onshake/gi)||[]).length;
            const motion=getComputedStyle(document.documentElement).animationName!=='none' || !!document.querySelector('video[autoplay], [data-motion]');
            return JSON.stringify({ pass:false, outcome:'review', details:`${references} device-motion API reference(s); motion candidate=${motion}`, nodes:references+(motion?1:0),
                reason:'Aucune permission capteur n’est demandée; vérifier une commande alternative indépendante du mouvement' });
        })()
    "#,
    ),
];
