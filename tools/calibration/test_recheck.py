#!/usr/bin/env python3
"""Tests de recheck.py, en particulier sur un export PARTIEL.

Un export partiel est le cas normal : on étiquette un lot en plusieurs séances, et l'export de la
première ne couvre qu'une partie des cartes de la clé. `recheck.py` indexait les étiquettes de
TOUTES les cartes de la clé et plantait (`KeyError`) sur la première non étiquetée.

    python3 -m unittest test_recheck -v        # depuis tools/calibration/, stdlib seule
"""
import contextlib
import io
import json
import os
import tempfile
import unittest
from types import SimpleNamespace

import recheck


def lot(tmp, n_cards):
    """Un lot de `n_cards` cartes u01..uNN ; les impairs sont attendus pertinents ('P')."""
    items, key = [], {}
    for i in range(1, n_cards + 1):
        qid = f"u{i:02d}"
        items.append({"qid": qid, "id": f"U{i:03d}", "alt": f"résumé {i}", "ctx": "contexte"})
        key[qid] = {"pred": "P" if i % 2 else "N", "kind": "test", "anchor": False}
    d = os.path.join(tmp, "lot")
    os.makedirs(d)
    json.dump(items, open(os.path.join(d, "items.json"), "w", encoding="utf-8"))
    json.dump(key, open(os.path.join(d, "key.json"), "w", encoding="utf-8"))
    return d


def export(tmp, name, verdicts):
    """Un export de feuille : {qid: 'pertinent' | 'non_pertinent'}."""
    path = os.path.join(tmp, name)
    json.dump(
        {"sheet": "t", "labeler": "x", "labels": [
            {"qid": q, "verdict": v, "comment": f"motif {q}"} for q, v in verdicts.items()]},
        open(path, "w", encoding="utf-8"),
    )
    return path


def build_args(lot_dir, labels, **kw):
    return SimpleNamespace(lot_dir=lot_dir, labels=labels, extra=kw.get("extra", ""),
                           n=kw.get("n", 14), seed=7, title="")


def run(fn, args):
    out, err = io.StringIO(), io.StringIO()
    with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
        fn(args)
    return out.getvalue(), err.getvalue()


class PartialExportBuild(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.tmp = self._tmp.name
        self.addCleanup(self._tmp.cleanup)

    def recheck_key(self, lot_dir):
        return json.load(open(os.path.join(lot_dir, "recheck", "key.json"), encoding="utf-8"))

    def test_an_export_missing_some_cards_still_builds(self):
        d = lot(self.tmp, 20)
        # 12 cartes étiquetées sur 20 : u09..u20. Avant le correctif : KeyError: 'u01'.
        labels = export(self.tmp, "a.json", {f"u{i:02d}": "pertinent" for i in range(9, 21)})
        run(recheck.cmd_build, build_args(d, labels))
        origs = {v["orig"] for v in self.recheck_key(d).values()}
        self.assertTrue(origs, "la re-vérification est vide")
        self.assertTrue(all(int(o[1:]) >= 9 for o in origs),
                        f"une carte sans étiquette a été re-soumise : {sorted(origs)}")

    def test_unlabeled_cards_are_reported_not_silently_dropped(self):
        d = lot(self.tmp, 20)
        labels = export(self.tmp, "a.json", {f"u{i:02d}": "pertinent" for i in range(9, 21)})
        _, err = run(recheck.cmd_build, build_args(d, labels))
        self.assertIn("8 carte(s)", err, "l'absence d'étiquette sur 8 cartes n'est pas signalée")
        for q in (f"u{i:02d}" for i in range(1, 9)):
            self.assertIn(q, err, f"{q} n'est pas nommée alors qu'elle est ignorée")

    def test_divergent_cards_are_still_flagged_on_a_partial_export(self):
        d = lot(self.tmp, 20)
        # u09 attendue P (impair) ; l'humain dit non_pertinent -> divergence.
        verdicts = {f"u{i:02d}": ("pertinent" if i % 2 else "non_pertinent") for i in range(9, 21)}
        verdicts["u09"] = "non_pertinent"
        labels = export(self.tmp, "a.json", verdicts)
        run(recheck.cmd_build, build_args(d, labels))
        flagged = {v["orig"] for v in self.recheck_key(d).values() if v["flagged"]}
        self.assertEqual(flagged, {"u09"})

    def test_an_extra_card_without_a_label_is_refused_loudly(self):
        d = lot(self.tmp, 20)
        labels = export(self.tmp, "a.json", {f"u{i:02d}": "pertinent" for i in range(9, 21)})
        with self.assertRaises(SystemExit) as caught:
            run(recheck.cmd_build, build_args(d, labels, extra="u01"))
        self.assertIn("u01", str(caught.exception))

    def test_a_recheck_below_the_protocol_minimum_is_warned(self):
        d = lot(self.tmp, 20)
        labels = export(self.tmp, "a.json", {f"u{i:02d}": "pertinent" for i in range(15, 21)})  # 6
        _, err = run(recheck.cmd_build, build_args(d, labels))
        self.assertIn(str(recheck.MIN_RECHECK), err, "le minimum du protocole n'est pas rappelé")

    def test_no_labeled_card_is_an_error_not_an_empty_sheet(self):
        d = lot(self.tmp, 20)
        labels = export(self.tmp, "a.json", {})
        with self.assertRaises(SystemExit):
            run(recheck.cmd_build, build_args(d, labels))

    def divergent_export(self, n_cards, divergent):
        """Export complet où les cartes `divergent` (qids) contredisent la règle, les autres la suivent."""
        verdicts = {}
        for i in range(1, n_cards + 1):
            q = f"u{i:02d}"
            follows = "pertinent" if i % 2 else "non_pertinent"          # = la règle
            contradicts = "non_pertinent" if i % 2 else "pertinent"
            verdicts[q] = contradicts if q in divergent else follows
        return export(self.tmp, "a.json", verdicts)

    def sample(self, d):
        key = self.recheck_key(d)
        divergent = [v for v in key.values() if (v["first"] == "pertinent") != (v["pred"] == "P")]
        return key, divergent

    def test_the_sample_keeps_at_least_as_many_concordant_cards_as_divergent_ones(self):
        """Protocole §3 : sinon la re-vérification fabrique l'accord (lot R : 17 divergentes sur 21)."""
        d = lot(self.tmp, 30)
        # 8 divergentes sur 30. Avant le correctif : n=14 -> 6 concordantes seulement, 6 < 8.
        labels = self.divergent_export(30, {f"u{i:02d}" for i in range(1, 9)})
        _, err = run(recheck.cmd_build, build_args(d, labels, n=14))
        key, divergent = self.sample(d)
        concordant = len(key) - len(divergent)
        self.assertEqual(len(divergent), 8)
        self.assertGreaterEqual(concordant, len(divergent),
                                f"{concordant} concordantes pour {len(divergent)} divergentes")
        self.assertNotIn("déséquilibr", err)

    def test_an_unbalanceable_sample_is_warned_not_hidden(self):
        """Pas assez de cartes concordantes dans l'export : on le dit au lieu de publier un biais muet."""
        d = lot(self.tmp, 20)
        # 14 divergentes, 6 concordantes seulement : l'équilibre est impossible.
        labels = self.divergent_export(20, {f"u{i:02d}" for i in range(1, 15)})
        _, err = run(recheck.cmd_build, build_args(d, labels, n=14))
        key, divergent = self.sample(d)
        self.assertEqual(len(key) - len(divergent), 6, "toutes les concordantes disponibles doivent être prises")
        self.assertIn("déséquilibré", err)
        self.assertIn("6", err)
        self.assertIn("14", err)

    def test_a_repeated_extra_card_is_resubmitted_once(self):
        """Un doublon ferait compter deux fois la même carte dans la cohérence mesurée."""
        d = lot(self.tmp, 20)
        labels = self.divergent_export(20, set())
        run(recheck.cmd_build, build_args(d, labels, extra="u03,u03,u03"))
        origs = [v["orig"] for v in self.recheck_key(d).values()]
        self.assertEqual(origs.count("u03"), 1, f"u03 re-soumise {origs.count('u03')} fois")
        self.assertEqual(len(origs), len(set(origs)), "une carte apparaît plusieurs fois")

    def test_a_full_export_behaves_as_before(self):
        d = lot(self.tmp, 20)
        labels = export(self.tmp, "a.json",
                        {f"u{i:02d}": ("pertinent" if i % 2 else "non_pertinent") for i in range(1, 21)})
        out, err = run(recheck.cmd_build, build_args(d, labels))
        self.assertEqual(len(self.recheck_key(d)), 14)
        self.assertNotIn("sans étiquette", err)


class PartialCompare(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.tmp = self._tmp.name
        self.addCleanup(self._tmp.cleanup)

    def built(self):
        d = lot(self.tmp, 20)
        labels = export(self.tmp, "a.json",
                        {f"u{i:02d}": ("pertinent" if i % 2 else "non_pertinent") for i in range(1, 21)})
        run(recheck.cmd_build, build_args(d, labels))
        key = json.load(open(os.path.join(d, "recheck", "key.json"), encoding="utf-8"))
        return d, labels, key

    def test_compare_with_nothing_relabeled_is_an_error_not_a_zero_division(self):
        d, labels, _ = self.built()
        second = export(self.tmp, "b.json", {})
        args = SimpleNamespace(lot_dir=d, labels=labels, labels2=second)
        with self.assertRaises(SystemExit):
            run(recheck.cmd_compare, args)

    def test_a_partially_relabeled_recheck_says_how_many_were_not(self):
        d, labels, key = self.built()
        answered = list(key)[:5]
        second = export(self.tmp, "b.json", {r: "pertinent" for r in answered})
        args = SimpleNamespace(lot_dir=d, labels=labels, labels2=second)
        out, err = run(recheck.cmd_compare, args)
        self.assertRegex(out, r"/5 = ", "la cohérence doit être calculée sur les 5 cartes re-étiquetées")
        self.assertIn("9 carte(s) sur 14 non re-étiquetée(s)", err,
                      "les 9 cartes non re-étiquetées ne sont pas signalées")
        self.assertIn(str(recheck.MIN_RECHECK), err, "5 cartes est sous le minimum du protocole")


if __name__ == "__main__":
    unittest.main()
