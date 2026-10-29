#!/usr/bin/env python3
"""Garde-fous du lot W (`lots/resume_tableau_noms.py`).

Le premier jet de ce lot réutilisait quatre tableaux déjà montrés à l'étiqueteur : il se présentait
comme un test hors échantillon sans l'être. Ces tests rendent cette erreur, et celle d'un lot
déséquilibré, impossibles à refaire sans que quelqu'un le voie.

    python3 -m unittest test_lot_noms -v        # depuis tools/calibration/, exige Pillow
"""
import os
import re
import unittest

from lots import resume_tableau_noms as lot

LOTS = os.path.join(os.path.dirname(os.path.abspath(__file__)), "lots")


def images_used_elsewhere():
    """Noms de tableaux dessinés par les AUTRES générateurs de lots (`table("nom", ...)`)."""
    seen = set()
    for f in os.listdir(LOTS):
        if f.endswith(".py") and f != "images_noms.py":
            seen |= set(re.findall(r'table\(\s*"([a-z_0-9]+)"', open(os.path.join(LOTS, f)).read()))
    return seen


class LotW(unittest.TestCase):
    def test_no_table_was_ever_shown_in_another_lot(self):
        mine = {r[0] for r in lot.ROWS}
        shared = mine & images_used_elsewhere()
        self.assertFalse(shared, f"tableaux déjà montrés à l'étiqueteur : {sorted(shared)}")

    def test_the_lot_is_balanced_so_the_trivial_baseline_is_low(self):
        p = sum(1 for r in lot.ROWS if r[2] == "P")
        self.assertEqual(p, len(lot.ROWS) - p, "le lot doit être équilibré 12/12")
        self.assertLessEqual(max(p, len(lot.ROWS) - p) / len(lot.ROWS), 0.60)

    def test_anchors_meet_the_protocol(self):
        anchors = [r for r in lot.ROWS if r[4]]
        self.assertGreaterEqual(len(anchors), 4)
        self.assertTrue(any("faux" in r[3] for r in anchors), "il faut une ancre factuellement fausse")

    def test_every_table_has_both_pertinent_and_non_pertinent_cards(self):
        for image in {r[0] for r in lot.ROWS}:
            preds = {r[2] for r in lot.ROWS if r[0] == image}
            self.assertEqual(preds, {"P", "N"}, f"{image} ne porte qu'un seul verdict attendu")

    def test_summaries_are_unique(self):
        texts = [r[1] for r in lot.ROWS]
        self.assertEqual(len(texts), len(set(texts)))


if __name__ == "__main__":
    unittest.main()
