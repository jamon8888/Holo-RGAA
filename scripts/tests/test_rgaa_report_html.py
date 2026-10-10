import importlib.util
import pathlib
import unittest


SCRIPT = pathlib.Path(__file__).resolve().parents[1] / "rgaa-report-html.py"
SPEC = importlib.util.spec_from_file_location("rgaa_report_html", SCRIPT)
REPORT = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
SPEC.loader.exec_module(REPORT)
CATALOG = pathlib.Path(__file__).resolve().parents[1].parent / "rgaa-rs/crates/rgaa-core/data/rgaa-4.1.2/criteres.json"
CATALOG_DOC = __import__("json").loads(CATALOG.read_text(encoding="utf-8"))
CRITERION_IDS = [
    f"{topic['number']}.{entry.get('criterium', entry)['number']}"
    for topic in CATALOG_DOC["topics"]
    for entry in topic.get("criteria", [])
]


def criterion(cid: str, *, auto="pass", status="needs_review", source="agent-estimate") -> dict:
    return {
        "criterion_id": cid,
        "title": f"Critère {cid}",
        "status": status,
        "source": source,
        "automated_verdict": auto,
        "verdict_basis": ["model_estimate"],
        "tests": [],
    }


def page(index: int, *, predictions: int = 106, status="needs_review") -> dict:
    criteria = [criterion(cid, auto="pass" if i < predictions else None, status=status)
                for i, cid in enumerate(CRITERION_IDS)]
    return {"url": f"https://example.test/{index}", "title": f"Page {index}", "criteria": criteria}


class ReportMetricTests(unittest.TestCase):
    def test_four_pages_use_424_automatic_slots_and_missing_prediction(self):
        pages = [page(i) for i in range(4)]
        pages[3]["criteria"][105]["automated_verdict"] = None
        metrics = REPORT.coverage_metrics({}, pages)
        self.assertEqual(metrics["expected_automatic"], 424)
        self.assertEqual(metrics["automatic_count"], 423)
        self.assertAlmostEqual(metrics["automatic_percent"], 100 * 423 / 424)
        self.assertEqual(metrics["verified_percent"], 0.0)

    def test_evidence_uses_258_test_slots_and_excludes_model_rows(self):
        pages = [page(0), page(1)]
        pages[0]["criteria"][0]["tests"] = [
            {"test_key": "1", "source": "axe-core", "evidence": "img#logo has alt"},
            {"test_key": "1", "source": "axe-core", "evidence": "duplicate observation"},
        ]
        pages[1]["criteria"][0]["tests"] = [
            {"test_key": "1", "source": "agent-estimate", "evidence": "model pointer"},
            {"test_key": "999", "source": "axe-core", "evidence": "unknown key"},
        ]
        metrics = REPORT.coverage_metrics({}, pages)
        self.assertEqual(metrics["expected_tests"], 516)
        self.assertEqual(metrics["evidence_count"], 1)
        self.assertAlmostEqual(metrics["evidence_percent"], 100 / 516)

    def test_verified_compliance_uses_human_and_non_model_results_not_agent_pass(self):
        page_data = {
            "criteria": [
                criterion("1.1", auto="pass", status="pass", source="agent"),
                {**criterion("1.2", status="needs_review"), "verified_status": "pass",
                 "review_events": [{"status": "pass", "author": "auditrice"}]},
                criterion("1.3", status="pass", source="axe-core"),
                criterion("1.4", status="fail", source="gap-fix"),
            ]
        }
        self.assertAlmostEqual(REPORT.verified_rate(page_data["criteria"]), 200 / 3)
        metrics = REPORT.coverage_metrics({}, [page_data])
        self.assertAlmostEqual(metrics["verified_percent"], 200 / 3)
        self.assertIsNone(REPORT.verified_status(criterion(
            "1.5", status="pass", source="agent-error"
        )))
        explicit_review = criterion("1.6", status="pass", source="axe-core")
        explicit_review["verified_status"] = "needs_review"
        self.assertIsNone(REPORT.verified_status(explicit_review))

    def test_multi_page_coverage_aggregates_raw_counts_before_rounding(self):
        pages = [page(0, predictions=1), page(1, predictions=1), page(2, predictions=2)]
        metrics = REPORT.coverage_metrics({}, pages)
        expected = 100 * 4 / 318
        self.assertEqual(metrics["automatic_count"], 4)
        self.assertEqual(metrics["expected_automatic"], 318)
        self.assertAlmostEqual(metrics["automatic_percent"], expected)
        rounded_page_average = (round(100 / 106, 1) + round(100 / 106, 1) + round(200 / 106, 1)) / 3
        self.assertNotAlmostEqual(metrics["automatic_percent"], rounded_page_average)

    def test_new_fields_render_separately_and_legacy_json_still_renders(self):
        modern = page(0, predictions=1)
        modern["criteria"][0].update({
            "raw_confidence": 0.82,
            "confidence": 0.75,
            "confidence_calibration_version": "cal-v1",
            "review_required": True,
            "review_reason": "Confirmer l’équivalence",
            "verified_status": "fail",
            "evidence": [{"kind": "dom", "hash": "sha256:abc", "location": "#logo"}],
            "review_events": [{"status": "fail", "author": "auditrice", "reviewed_at": "2026-10-07", "reason": "Alternative absente"}],
        })
        html = REPORT.render({"audit_id": "modern", "url": modern["url"], "pages": [modern]})
        for label in (
            "Couverture des verdicts automatiques", "Couverture des tests avec preuve",
            "Conformité vérifiée", "Verdict automatique", "Statut vérifié",
            "Statut brut",
            "Confiance brute", "Confiance calibrée", "Revue humaine requise",
            "Confirmer l’équivalence", "sha256:abc", "auditrice", "estimation",
        ):
            self.assertIn(label, html)
        self.assertNotIn("Couverture moteur", html)

        legacy = REPORT.render({"audit_id": "legacy", "url": "https://old.test", "pages": [
            {"url": "https://old.test", "criteria": [{"criterion_id": "1.1", "status": "pass", "source": "axe-core"}]}
        ]})
        self.assertIn("Couverture des verdicts automatiques", legacy)
        self.assertIn("Conformité vérifiée", legacy)
        self.assertIn("Statut vérifié</h4><p>Conforme", legacy)


if __name__ == "__main__":
    unittest.main()
