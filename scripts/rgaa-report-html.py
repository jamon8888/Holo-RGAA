#!/usr/bin/env python3
"""Render an RGAA audit JSON (rgaa-cli audit analyze --format json) as a standalone HTML report.

Usage:
    scripts/rgaa-report-html.py <audit.json> [-o out.html]
    rgaa-cli audit analyze --url example.com --format json | scripts/rgaa-report-html.py - -o out.html
"""
from __future__ import annotations

import argparse
import html
import json
import pathlib
import re
import sys
from collections import Counter
from datetime import datetime, timezone

STATUS_LABELS = {
    "pass": "Conforme",
    "fail": "Non conforme",
    "needs_review": "À vérifier",
    "not_applicable": "Non applicable",
    "not_tested": "Non testé",
}
STATUS_ORDER = ["fail", "needs_review", "not_tested", "pass", "not_applicable"]
EXPECTED_CRITERIA_PER_PAGE = 106
EXPECTED_TESTS_PER_PAGE = 258
CLASSIFICATION_LABELS = {
    "Deterministe": "Déterministe",
    "IaAssiste": "IA-assisté",
    "Manuel": "Manuel",
}
# RGAA 4.1.2 thematic families, in catalog order.
TOPICS = {
    "1": "Images",
    "2": "Cadres",
    "3": "Couleurs",
    "4": "Multimédia",
    "5": "Tableaux",
    "6": "Liens",
    "7": "Scripts",
    "8": "Éléments obligatoires",
    "9": "Structuration de l'information",
    "10": "Présentation de l'information",
    "11": "Formulaires",
    "12": "Navigation",
    "13": "Consultation",
}

LINK_RE = re.compile(r"\[([^\]]+)\]\(#[^)]*\)")

# Official RGAA 4.1.2 catalog shipped with rgaa-core; used to fill in the criterion
# wording for results the engine returns with an empty `title` (axe-core / gap-fix rows).
DEFAULT_CATALOG = (
    "rgaa-rs/crates/rgaa-core/data/rgaa-4.1.2/criteres.json"
)


def load_catalog(path: str | None) -> tuple[dict[str, str], dict[str, str]]:
    """Return ({criterion_id: title}, {topic_number: topic_name}) from criteres.json."""
    candidate = pathlib.Path(path) if path else pathlib.Path(__file__).resolve().parent.parent / DEFAULT_CATALOG
    if not candidate.is_file():
        return {}, {}
    doc = json.loads(candidate.read_text(encoding="utf-8"))
    titles: dict[str, str] = {}
    topics: dict[str, str] = {}
    for topic in doc.get("topics", []):
        num = str(topic.get("number"))
        topics[num] = topic.get("topic", "")
        for entry in topic.get("criteria", []):
            c = entry.get("criterium", entry)
            titles[f"{num}.{c.get('number')}"] = c.get("title", "")
    return titles, topics


def clean_title(raw: str | None) -> str:
    """Strip the RGAA glossary markdown links, keeping the label."""
    return LINK_RE.sub(r"\1", (raw or "").strip())


def crit_sort_key(cid: str) -> tuple[int, int]:
    head, _, tail = cid.partition(".")
    try:
        return int(head), int(tail)
    except ValueError:
        return 99, 99


def unwrap_justification(text: str | None) -> str:
    """Some agent answers leak a raw JSON verdict envelope; surface the inner justification."""
    if not text:
        return ""
    stripped = text.strip()
    if stripped.startswith("{"):
        try:
            inner = json.loads(stripped)
        except json.JSONDecodeError:
            pass
        else:
            if isinstance(inner, dict) and inner.get("justification"):
                return str(inner["justification"]).strip()
    return stripped


def page_counts(criteria: list[dict]) -> Counter:
    return Counter(status_token(c.get("status", "not_tested")) for c in criteria)


def status_token(value: object) -> str:
    raw = str(value or "not_tested").lower().replace("-", "_")
    return {
        "notapplicable": "not_applicable",
        "needsreview": "needs_review",
        "nottested": "not_tested",
        "modelestimate": "model_estimate",
    }.get(raw, raw)


def applicable_rate(counts: Counter) -> float | None:
    """RGAA conformity = pass / (pass + fail) over criteria actually decided."""
    decided = counts["pass"] + counts["fail"]
    return 100.0 * counts["pass"] / decided if decided else None


def model_source(source: object) -> bool:
    value = str(source or "").lower()
    return (value in {"agent", "holo", "holo3", "myia"} or value.startswith("agent-")
            or "model" in value or "estimate" in value)


def verified_status(criterion: dict) -> str | None:
    if criterion.get("verified_status") is not None:
        explicit = status_token(criterion["verified_status"])
        return explicit if explicit in {"pass", "fail", "not_applicable"} else None
    if model_source(criterion.get("source")):
        return None
    status = status_token(criterion.get("status"))
    return status if status in {"pass", "fail", "not_applicable"} else None


def verified_rate(criteria: list[dict]) -> float | None:
    counts = Counter(status for item in criteria if (status := verified_status(item)))
    return applicable_rate(counts)


def coverage_metrics(data: dict, pages: list[dict]) -> dict[str, float | int]:
    catalog_ids = set()
    test_keys: dict[str, set[str]] = {}
    catalog_path = pathlib.Path(__file__).resolve().parent.parent / "rgaa-rs/crates/rgaa-core/data/rgaa-4.1.2/automatable_criteres.json"
    if catalog_path.is_file():
        catalog = json.loads(catalog_path.read_text(encoding="utf-8"))
        for entry in catalog.get("criteria", []):
            cid = str(entry.get("criterion_id", ""))
            catalog_ids.add(cid)
            test_keys[cid] = {str(key) for key in entry.get("test_keys", [])}

    automatic_count = 0
    evidence_slots: set[tuple[int, str, str]] = set()
    site_statuses: dict[str, list[str | None]] = {}
    for page_index, page in enumerate(pages):
        first_by_id: dict[str, dict] = {}
        for criterion in page.get("criteria", []):
            cid = str(criterion.get("criterion_id", ""))
            if cid in catalog_ids:
                first_by_id.setdefault(cid, criterion)
                outcomes = criterion.get("tests") or []
                for outcome in outcomes:
                    key = str(outcome.get("test_key", ""))
                    evidence = outcome.get("evidence")
                    if (key in test_keys.get(cid, set()) and not model_source(outcome.get("source"))
                            and isinstance(evidence, str) and evidence.strip()):
                        evidence_slots.add((page_index, cid, key))
        for cid in catalog_ids:
            criterion = first_by_id.get(cid)
            site_statuses.setdefault(cid, []).append(verified_status(criterion) if criterion else None)
        automatic_count += sum(
            1 for criterion in first_by_id.values()
            if status_token(criterion.get("automated_verdict")) in {"pass", "fail", "not_applicable"}
        )

    verified_pass = verified_fail = 0
    for statuses in site_statuses.values():
        if "fail" in statuses:
            verified_fail += 1
        elif len(statuses) == len(pages) and all(s in {"pass", "not_applicable"} for s in statuses):
            if "pass" in statuses:
                verified_pass += 1
    verified_percent = 100.0 * verified_pass / (verified_pass + verified_fail) if verified_pass + verified_fail else 0.0

    expected_automatic = len(pages) * EXPECTED_CRITERIA_PER_PAGE
    expected_tests = len(pages) * EXPECTED_TESTS_PER_PAGE
    automatic_percent = 100.0 * automatic_count / expected_automatic if expected_automatic else 0.0
    evidence_count = len(evidence_slots)
    evidence_percent = 100.0 * evidence_count / expected_tests if expected_tests else 0.0
    return {
        "expected_automatic": expected_automatic,
        "automatic_count": automatic_count,
        "automatic_percent": automatic_percent,
        "expected_tests": expected_tests,
        "evidence_count": evidence_count,
        "evidence_percent": evidence_percent,
        "verified_percent": verified_percent,
    }


CSS = """
:root{--bg:#f4f6f8;--surface:#fff;--ink:#1c2733;--muted:#5c6b7a;--line:#dde3ea;
--pass:#1d8348;--fail:#c0392b;--review:#b9770e;--nt:#6c7a89;--na:#8e9aa6;--accent:#1a5276;
--pass-bg:#e8f6ee;--fail-bg:#fdecea;--review-bg:#fdf3e2;--nt-bg:#eef1f4;--na-bg:#f2f4f6;}
*{margin:0;padding:0;box-sizing:border-box}
body{font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,Helvetica,Arial,sans-serif;
line-height:1.6;color:var(--ink);background:var(--bg);-webkit-font-smoothing:antialiased}
.wrap{max-width:1180px;margin:0 auto;padding:1.5rem}
a{color:var(--accent)}
header{background:linear-gradient(135deg,#17405e,#2980b9);color:#fff;padding:2rem;border-radius:10px;margin-bottom:1.5rem}
header h1{font-size:1.55rem;line-height:1.3;margin-bottom:.4rem}
header .sub{opacity:.92;font-size:.95rem}
header dl{display:flex;flex-wrap:wrap;gap:.4rem 1.75rem;margin-top:1rem;font-size:.87rem;opacity:.95}
header dl div{display:flex;gap:.4rem}
header dt{opacity:.8}
header dd{font-variant-numeric:tabular-nums}
.cards{display:grid;grid-template-columns:repeat(auto-fit,minmax(170px,1fr));gap:1rem;margin-bottom:1.5rem}
.card{background:var(--surface);padding:1.15rem 1.25rem;border-radius:10px;border:1px solid var(--line)}
.card h3{font-size:.74rem;text-transform:uppercase;letter-spacing:.05em;color:var(--muted);font-weight:600;margin-bottom:.3rem}
.card .v{font-size:2rem;font-weight:700;line-height:1.1;font-variant-numeric:tabular-nums}
.card .n{font-size:.78rem;color:var(--muted);margin-top:.15rem}
.v.pass{color:var(--pass)}.v.fail{color:var(--fail)}.v.review{color:var(--review)}
.v.nt{color:var(--nt)}.v.na{color:var(--na)}.v.accent{color:var(--accent)}
section.block{background:var(--surface);border:1px solid var(--line);border-radius:10px;padding:1.5rem;margin-bottom:1.5rem}
section.block > h2{font-size:1.15rem;margin-bottom:.35rem}
section.block > p.lead{color:var(--muted);font-size:.9rem;margin-bottom:1rem}
.note{background:var(--review-bg);border-left:4px solid var(--review);padding:.85rem 1rem;border-radius:0 6px 6px 0;font-size:.88rem;margin-bottom:1rem}
.bar{display:flex;height:14px;border-radius:7px;overflow:hidden;background:var(--nt-bg);margin:.6rem 0 .5rem}
.bar span{display:block}
.bar .s-pass{background:var(--pass)}.bar .s-fail{background:var(--fail)}
.bar .s-needs_review{background:var(--review)}.bar .s-not_tested{background:var(--nt)}
.bar .s-not_applicable{background:var(--na)}
.legend{display:flex;flex-wrap:wrap;gap:.35rem .9rem;font-size:.8rem;color:var(--muted)}
.legend i{display:inline-block;width:9px;height:9px;border-radius:2px;margin-right:.3rem;vertical-align:baseline}
table{width:100%;border-collapse:collapse;font-size:.88rem}
th,td{text-align:left;padding:.55rem .6rem;border-bottom:1px solid var(--line);vertical-align:top}
th{font-size:.74rem;text-transform:uppercase;letter-spacing:.04em;color:var(--muted);font-weight:600}
td.num{font-variant-numeric:tabular-nums;white-space:nowrap}
.badge{display:inline-block;padding:.12rem .55rem;border-radius:20px;font-size:.76rem;font-weight:600;white-space:nowrap}
.b-pass{background:var(--pass-bg);color:var(--pass)}.b-fail{background:var(--fail-bg);color:var(--fail)}
.b-needs_review{background:var(--review-bg);color:var(--review)}
.b-not_tested{background:var(--nt-bg);color:var(--nt)}
.b-not_applicable{background:var(--na-bg);color:var(--na)}
.tag{display:inline-block;padding:.08rem .45rem;border-radius:4px;background:var(--nt-bg);
color:var(--muted);font-size:.72rem;font-family:ui-monospace,SFMono-Regular,Menlo,monospace}
.controls{display:flex;flex-wrap:wrap;gap:.5rem;align-items:center;margin-bottom:1rem}
.controls input[type=search]{flex:1 1 220px;min-width:180px;padding:.45rem .65rem;border:1px solid var(--line);
border-radius:6px;font:inherit;font-size:.88rem;background:var(--surface)}
.chip{padding:.35rem .8rem;border:1px solid var(--line);background:var(--surface);border-radius:20px;
font:inherit;font-size:.82rem;cursor:pointer;color:var(--muted)}
.chip[aria-pressed=true]{background:var(--accent);border-color:var(--accent);color:#fff;font-weight:600}
details.crit{border:1px solid var(--line);border-radius:8px;margin-bottom:.5rem;background:var(--surface)}
details.crit[hidden],h4[data-topichead][hidden]{display:none}
details.crit > summary{cursor:pointer;padding:.7rem .9rem;display:grid;
grid-template-columns:4.2rem 7.5rem 1fr auto;gap:.75rem;align-items:baseline;list-style:none}
details.crit > summary::-webkit-details-marker{display:none}
details.crit > summary:hover{background:#fafbfc}
details.crit > summary:focus-visible{outline:2px solid var(--accent);outline-offset:-2px}
summary .cid{font-weight:700;font-variant-numeric:tabular-nums}
summary .q{font-size:.88rem;color:var(--ink)}
summary .q.empty{color:var(--muted);font-style:italic}
summary .conf{font-size:.76rem;color:var(--muted);font-variant-numeric:tabular-nums}
.crit-body{padding:0 .9rem 1rem;border-top:1px solid var(--line);margin-top:-1px}
.crit-body h4{font-size:.74rem;text-transform:uppercase;letter-spacing:.04em;
color:var(--muted);margin:.9rem 0 .25rem}
.crit-body p{font-size:.88rem}
.viol{background:var(--fail-bg);border-left:3px solid var(--fail);padding:.6rem .8rem;
border-radius:0 6px 6px 0;margin-top:.4rem;font-size:.85rem}
.viol code{font-family:ui-monospace,SFMono-Regular,Menlo,monospace;font-weight:600}
.empty-msg{padding:1rem;color:var(--muted);font-size:.9rem;text-align:center}
footer{color:var(--muted);font-size:.8rem;text-align:center;padding:1rem 0 2rem}
@media print{body{background:#fff}.controls{display:none}
details.crit{break-inside:avoid}details.crit[hidden]{display:none}}
@media(max-width:700px){
details.crit > summary{grid-template-columns:3.6rem 1fr;gap:.3rem .6rem}
summary .badge-cell{grid-column:2}
header{padding:1.5rem}.wrap{padding:1rem}}
"""

JS = """
(function(){
  var q=document.getElementById('q'), chips=[].slice.call(document.querySelectorAll('.chip'));
  var items=[].slice.call(document.querySelectorAll('details.crit'));
  function apply(){
    var term=(q.value||'').toLowerCase().trim();
    var on=chips.filter(function(c){return c.getAttribute('aria-pressed')==='true';})
                .map(function(c){return c.dataset.status;});
    items.forEach(function(el){
      var okStatus = !on.length || on.indexOf(el.dataset.status)>-1;
      var okTerm = !term || el.dataset.search.indexOf(term)>-1;
      el.hidden = !(okStatus && okTerm);
    });
    document.querySelectorAll('h4[data-topichead]').forEach(function(h){
      var any=false, n=h.nextElementSibling;
      while(n && !(n.tagName==='H4' && n.hasAttribute('data-topichead'))){
        if(n.classList.contains('crit') && !n.hidden){any=true;break;}
        n=n.nextElementSibling;
      }
      h.hidden=!any;
    });
    document.querySelectorAll('[data-pagegroup]').forEach(function(g){
      var any=[].slice.call(g.querySelectorAll('details.crit')).some(function(e){return !e.hidden;});
      var msg=g.querySelector('.empty-msg');
      if(msg) msg.hidden=any;
    });
  }
  q.addEventListener('input',apply);
  chips.forEach(function(c){c.addEventListener('click',function(){
    c.setAttribute('aria-pressed', c.getAttribute('aria-pressed')==='true'?'false':'true');
    apply();
  });});
  apply();
})();
"""


def e(x) -> str:
    return html.escape(str(x), quote=True)


def render(data: dict, titles: dict[str, str] | None = None,
           topics: dict[str, str] | None = None) -> str:
    titles = titles or {}
    topics = {**TOPICS, **(topics or {})}
    pages = data.get("pages", [])
    site = data.get("url", "")
    audit_id = data.get("audit_id", "")
    all_counts = Counter()
    for p in pages:
        all_counts += page_counts(p.get("criteria", []))

    metrics = coverage_metrics(data, pages)
    overall = metrics["verified_percent"]
    dur = data.get("duration_ms")
    dur_txt = f"{dur/1000/60:.0f} min {dur/1000%60:.0f} s" if isinstance(dur, (int, float)) else "—"
    generated = datetime.now(timezone.utc).astimezone().strftime("%d/%m/%Y %H:%M %Z")

    out: list[str] = []
    a = out.append
    a("<!DOCTYPE html>\n<html lang=\"fr\">\n<head>\n<meta charset=\"utf-8\">")
    a('<meta name="viewport" content="width=device-width, initial-scale=1">')
    a(f"<title>Rapport RGAA 4.1.2 — {e(site)}</title>")
    a(f"<style>{CSS}</style>\n</head>\n<body>\n<div class=\"wrap\">")

    # ---- header
    a("<header>")
    a(f"<h1>Rapport d’audit d’accessibilité RGAA 4.1.2</h1>")
    a(f'<p class="sub">{e(site)}</p>')
    a("<dl>")
    for dt, dd in [
        ("Audit", audit_id or "—"),
        ("Pages auditées", len(pages)),
        ("Critères par page", data.get("total_criteria", 106)),
        ("Durée", dur_txt),
        ("Rapport généré le", generated),
    ]:
        a(f"<div><dt>{e(dt)} :</dt><dd>{e(dd)}</dd></div>")
    a("</dl></header>")

    metadata = data.get("audit_metadata") or {}
    if metadata:
        a('<section class="block"><h2>Actualisation des contrôles automatisés</h2>')
        if metadata.get("model"):
            a(f'<p class="lead"><strong>Modèle IA de l’audit source :</strong> {e(metadata["model"])}</p>')
        for item in metadata.get("notes", []):
            a(f'<p class="lead">{e(item)}</p>')
        a("</section>")

    # ---- summary cards
    engine_rate = data.get("verified_compliance_percent", data.get("taux_global", data.get("overall_compliance")))
    a('<div class="cards">')

    def card(label, value, cls, note=""):
        a(f'<div class="card"><h3>{e(label)}</h3><div class="v {cls}">{e(value)}</div>'
          + (f'<div class="n">{e(note)}</div>' if note else "") + "</div>")

    card("Conformité vérifiée", f"{overall:.1f} %", "accent",
         "Pass vérifiés / (Pass + Fail vérifiés), estimations IA exclues")
    card("Couverture des verdicts automatiques", f"{metrics['automatic_percent']:.1f} %", "accent",
         f"{metrics['automatic_count']} / {metrics['expected_automatic']} critères-page avec prédiction")
    card("Couverture des tests avec preuve", f"{metrics['evidence_percent']:.1f} %", "accent",
         f"{metrics['evidence_count']} / {metrics['expected_tests']} tests avec preuve non issue du modèle")
    audit_complete = data.get("audit_complete")
    completion_label, completion_class = {
        True: ("Audit automatique complet", "pass"),
        False: ("Audit automatique incomplet", "fail"),
        None: ("Complétude non renseignée", "review"),
    }[audit_complete if isinstance(audit_complete, bool) else None]
    card("État de l’audit automatique", completion_label, completion_class)
    card("Statut brut : conformes", all_counts["pass"], "pass", "sur l’ensemble des pages")
    card("Statut brut : non conformes", all_counts["fail"], "fail", "à corriger en priorité")
    card("Statut brut : à vérifier", all_counts["needs_review"], "review", "revue manuelle requise")
    card("Statut brut : non testés", all_counts["not_tested"], "nt", "hors couverture automatisée")
    card("Statut brut : non applicables", all_counts["not_applicable"], "na", "critère sans objet sur la page")
    a("</div>")

    # ---- reading note
    a('<section class="block"><h2>Comment lire ce rapport</h2>')
    a('<p class="lead">La conformité vérifiée utilise les seuls statuts confirmés par des preuves '
      'non issues du modèle ou par une revue humaine. Les prédictions automatiques restent '
      'distinctes et les estimations IA seules ne sont pas comptées comme conformité vérifiée.</p>')
    if isinstance(engine_rate, (int, float)) and abs(engine_rate - overall) > 0.05:
        a('<div class="note"><strong>Écart avec le moteur.</strong> '
          f'Le moteur annonce {engine_rate:.2f} % de conformité globale ; recalculé à partir des '
          f'statuts vérifiés, le taux est de {overall:.1f} %. '
          'Les compteurs agrégés du JSON source '
          f'(conformes {data.get("passed")}, non conformes {data.get("failed")}, '
          f'non applicables {data.get("na")}) ne totalisent pas '
          f'{len(pages)} × {data.get("total_criteria", 106)} critères — '
          'à vérifier côté agrégation du moteur.</div>')
    nr = all_counts["needs_review"] + all_counts["not_tested"]
    if nr:
        a('<div class="note"><strong>Portée de l’automatisation.</strong> '
          f'{nr} évaluations sur {sum(all_counts.values())} ne sont pas tranchées '
          '(« à vérifier » ou « non testé »). Ce rapport ne constitue donc pas une '
          'déclaration de conformité : il faut compléter par un audit manuel, en particulier '
          'pour le focus visible (10.7), les contenus au survol (10.13) et la validité du code (8.2).</div>')
    a("</section>")

    # ---- per page summary table
    a('<section class="block"><h2>Synthèse par page</h2>')
    a('<p class="lead">Conformité vérifiée et répartition des statuts bruts pour les 106 critères RGAA.</p>')
    a("<table><thead><tr><th>Page</th><th>Conformité vérifiée</th>"
      + "".join(f"<th>Statut brut : {e(STATUS_LABELS[s])}</th>" for s in STATUS_ORDER)
      + "</tr></thead><tbody>")
    for i, p in enumerate(pages):
        c = page_counts(p.get("criteria", []))
        rate = verified_rate(p.get("criteria", []))
        a(f'<tr><td><a href="#page-{i}">{e(p.get("title") or p.get("url"))}</a><br>'
          f'<span class="tag">{e(p.get("url"))}</span></td>'
          f'<td class="num"><strong>{f"{rate:.1f} %" if rate is not None else "—"}</strong></td>'
          + "".join(f'<td class="num">{c[s]}</td>' for s in STATUS_ORDER)
          + "</tr>")
    a("</tbody></table></section>")

    # ---- action list
    issues = []
    for p in pages:
        for c in p.get("criteria", []):
            if c.get("status") == "fail":
                issues.append((p, c))
    a('<section class="block"><h2>Points bloquants à corriger</h2>')
    if issues:
        a(f'<p class="lead">{len(issues)} critère(s) non conforme(s), avec la règle technique en cause.</p>')
        a("<table><thead><tr><th>Critère</th><th>Page</th><th>Règle</th>"
          "<th>Impact</th><th>Éléments</th></tr></thead><tbody>")
        for p, c in sorted(issues, key=lambda t: crit_sort_key(t[1]["criterion_id"])):
            vs = c.get("violations") or [{}]
            for v in vs:
                a(f'<tr><td class="num"><strong>{e(c["criterion_id"])}</strong></td>'
                  f'<td><span class="tag">{e(p.get("url"))}</span></td>'
                  f'<td><code>{e(v.get("rule_id","—"))}</code><br>{e(v.get("description",""))}</td>'
                  f'<td>{e(v.get("impact","—"))}</td>'
                  f'<td class="num">{e(v.get("nodes_affected","—"))}</td></tr>')
        a("</tbody></table>")
    else:
        a('<p class="lead">Aucun critère en échec sur les pages auditées.</p>')
    a("</section>")

    # ---- detail per page
    a('<section class="block"><h2>Détail des 106 critères</h2>')
    a('<p class="lead">Chaque ligne sépare la prédiction automatique du statut vérifié et expose '
      'son fondement, ses preuves, sa confiance et son historique de revue. Dépliez-la pour les détails.</p>')
    a('<div class="controls">')
    a('<input type="search" id="q" placeholder="Filtrer par numéro, intitulé ou mot-clé…" '
      'aria-label="Filtrer les critères">')
    for s in STATUS_ORDER:
        a(f'<button type="button" class="chip" data-status="{s}" aria-pressed="false">'
          f'{e(STATUS_LABELS[s])} ({all_counts[s]})</button>')
    a("</div>")

    for i, p in enumerate(pages):
        c = page_counts(p.get("criteria", []))
        total = sum(c.values()) or 1
        a(f'<div data-pagegroup id="page-{i}">')
        a(f'<h3 style="margin:1.4rem 0 .2rem;font-size:1rem">{e(p.get("title") or p.get("url"))}</h3>')
        a(f'<p style="font-size:.82rem;color:var(--muted)"><span class="tag">{e(p.get("url"))}</span></p>')
        a('<div class="bar">')
        for s in STATUS_ORDER:
            if c[s]:
                a(f'<span class="s-{s}" style="width:{100*c[s]/total:.2f}%" '
                  f'title="{e(STATUS_LABELS[s])} : {c[s]}"></span>')
        a("</div>")
        a('<p class="legend">' + "".join(
            f'<span><i class="s-{s}" style="background:var(--{ {"pass":"pass","fail":"fail","needs_review":"review","not_tested":"nt","not_applicable":"na"}[s] })"></i>'
            f'{e(STATUS_LABELS[s])} : {c[s]}</span>' for s in STATUS_ORDER) + "</p>")

        crits = sorted(p.get("criteria", []), key=lambda x: crit_sort_key(x["criterion_id"]))
        last_topic = None
        for crit in crits:
            cid = crit["criterion_id"]
            topic = topics.get(cid.split(".")[0], "")
            if topic != last_topic:
                a(f'<h4 data-topichead style="margin:1.1rem 0 .4rem;font-size:.78rem;'
                  f'text-transform:uppercase;letter-spacing:.05em;color:var(--muted)">'
                  f'{e(cid.split(".")[0])}. {e(topic)}</h4>')
                last_topic = topic
            st = status_token(crit.get("status", "not_tested"))
            title = clean_title(crit.get("title"))
            from_catalog = False
            if not title:
                title = clean_title(titles.get(cid))
                from_catalog = bool(title)
            just = unwrap_justification(crit.get("justification"))
            conf = crit.get("confidence")
            raw_conf = crit.get("raw_confidence")
            src = crit.get("source", "—")
            considered = crit.get("considered_sources") or []
            haystack = " ".join([cid, title, just, src, " ".join(considered)]).lower()
            a(f'<details class="crit" data-status="{st}" data-search="{e(haystack)}">')
            a("<summary>")
            a(f'<span class="cid">{e(cid)}</span>')
            a(f'<span class="badge-cell"><span class="tag">Statut brut</span> '
              f'<span class="badge b-{st}">{e(STATUS_LABELS[st])}</span></span>')
            if title:
                a(f'<span class="q">{e(title)}</span>')
            else:
                a('<span class="q empty">Intitulé indisponible</span>')
            shown_conf = conf if isinstance(conf, (int, float)) else raw_conf
            a('<span class="conf">'
              + (f"confiance {shown_conf:.0%}" if isinstance(shown_conf, (int, float)) else "")
              + "</span>")
            a("</summary>")
            a('<div class="crit-body">')
            if from_catalog:
                a('<p style="font-size:.78rem;color:var(--muted)">Intitulé repris du '
                  'référentiel RGAA 4.1.2 : le moteur ne l’a pas renvoyé pour ce résultat.</p>')
            a(f'<h4>Classification</h4><p>{e(CLASSIFICATION_LABELS.get(crit.get("classification",""), crit.get("classification","—")))}</p>')
            a(f'<h4>Mécanisme décisionnaire</h4><p><span class="tag">{e(src)}</span>'
              + (" &nbsp;mécanismes consultés : " + ", ".join(f'<span class="tag">{e(s)}</span>' for s in considered)
                 if considered else "") + "</p>")
            auto = status_token(crit.get("automated_verdict")) if crit.get("automated_verdict") else None
            basis = crit.get("verdict_basis") or []
            basis_labels = {"axe": "axe", "deterministic": "déterministe", "browser": "navigateur", "model_estimate": "estimation IA"}
            a("<h4>Verdict automatique</h4><p>"
              + (e(STATUS_LABELS.get(auto, auto)) if auto else "Aucune prédiction")
              + (" — estimation" if auto and (any(status_token(x) == "model_estimate" for x in basis) or model_source(src)) else "")
              + "</p>")
            a("<h4>Fondement</h4><p>" + (e(", ".join(basis_labels.get(status_token(x), str(x)) for x in basis)) if basis else "—") + "</p>")
            explicit_verified = crit.get("verified_status")
            verified = verified_status(crit)
            a("<h4>Statut vérifié</h4><p>"
              + (e(STATUS_LABELS.get(verified, verified)) if verified else "Non vérifié")
              + (" (revue humaine)" if explicit_verified and crit.get("review_events") else "")
              + "</p>")
            a("<h4>Revue humaine requise</h4><p>"
              + ("Oui" if crit.get("review_required") else "Non")
              + (f" — {e(crit.get('review_reason'))}" if crit.get("review_required") and crit.get("review_reason") else "")
              + "</p>")
            if isinstance(raw_conf, (int, float)) or isinstance(conf, (int, float)):
                a("<h4>Confiance brute</h4><p>"
                  + (f"{raw_conf:.0%}" if isinstance(raw_conf, (int, float)) else "—")
                  + "</p><h4>Confiance calibrée</h4><p>"
                  + (f"{conf:.0%}" if isinstance(conf, (int, float)) else "—")
                  + (f" ; version : {e(crit.get('confidence_calibration_version'))}" if crit.get("confidence_calibration_version") else "")
                  + "</p>")
            refs = crit.get("evidence") or []
            test_evidence = [test for test in (crit.get("tests") or [])
                             if isinstance(test.get("evidence"), str) and test["evidence"].strip()]
            a("<h4>Éléments de preuve</h4><ul>"
              + ("".join("<li>" + e(ref.get("kind", "preuve")) + " — "
                         + e(ref.get("location", "")) + " — " + e(ref.get("hash", "")) + "</li>" for ref in refs)
                 + "".join("<li>" + e(test.get("source", "mécanisme")) + " — "
                          + e(test.get("evidence")) + "</li>" for test in test_evidence)
                 if refs or test_evidence else "<li>Aucun élément référencé</li>")
              + "</ul>")
            events = crit.get("review_events") or []
            if events:
                a("<h4>Historique des revues</h4><ul>"
                  + "".join("<li>" + e(STATUS_LABELS.get(status_token(event.get("status")), event.get("status")))
                           + " — " + e(event.get("author", "")) + " — " + e(event.get("reviewed_at", ""))
                           + " — " + e(event.get("reason", "")) + "</li>" for event in events)
                  + "</ul>")
            tests = crit.get("tests") or []
            non_model_evidence = any(not model_source(test.get("source")) and isinstance(test.get("evidence"), str)
                                     and test["evidence"].strip() for test in tests)
            if auto and "model_estimate" in [status_token(x) for x in basis] and not non_model_evidence:
                a('<p class="note">Cette valeur est une estimation automatique sans preuve de contrôle non issue du modèle.</p>')
            a("<h4>Justification</h4><p>"
              + (e(just) if just else "<em>Aucune justification fournie par le moteur.</em>")
              + "</p>")
            for v in crit.get("violations") or []:
                a(f'<div class="viol"><code>{e(v.get("rule_id","—"))}</code> — '
                  f'{e(v.get("description",""))}<br>impact : {e(v.get("impact","—"))}, '
                  f'éléments concernés : {e(v.get("nodes_affected","—"))}</div>')
            a("</div></details>")
        a('<p class="empty-msg" hidden>Aucun critère ne correspond au filtre sur cette page.</p>')
        a("</div>")
    a("</section>")

    a(f'<footer>Audit {e(audit_id)} — référentiel RGAA 4.1.2 — '
      f'généré par rgaa-cli le {e(generated)}.<br>'
      'Ce rapport automatisé ne vaut pas déclaration de conformité : '
      'les critères « à vérifier » et « non testés » doivent être instruits manuellement.</footer>')
    a(f"</div>\n<script>{JS}</script>\n</body>\n</html>")
    return "\n".join(out)


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("source", help="audit JSON file, or - for stdin")
    ap.add_argument("-o", "--output", help="output HTML path (default: stdout)")
    ap.add_argument("--catalog", help="path to RGAA criteres.json "
                    "(default: rgaa-core's bundled 4.1.2 catalog)")
    args = ap.parse_args()
    raw = sys.stdin.read() if args.source == "-" else open(args.source, encoding="utf-8").read()
    titles, topics = load_catalog(args.catalog)
    doc = render(json.loads(raw), titles, topics)
    if args.output:
        with open(args.output, "w", encoding="utf-8") as fh:
            fh.write(doc)
        print(f"wrote {args.output} ({len(doc.encode()):,} bytes)", file=sys.stderr)
    else:
        sys.stdout.write(doc)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
