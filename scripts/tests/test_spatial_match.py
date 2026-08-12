"""Tests for duplicate-instance minimum-cost spatial pairing."""

from __future__ import annotations

import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from spatial_match import minimum_cost_pairs


class MinimumCostPairsTest(unittest.TestCase):
    def test_finds_global_assignment_instead_of_greedy_nearest(self) -> None:
        # Greedy picks reference 0 -> candidate 0 (distance 4), stranding the
        # second reference 11.18pt away. The global optimum crosses the pairs.
        references = [(0.0, 0.0), (10.0, 0.0)]
        candidates = [(4.0, 0.0), (0.0, 5.0)]

        self.assertEqual(minimum_cost_pairs(references, candidates), [(0, 1), (1, 0)])

    def test_rectangular_assignment_preserves_reference_indexes(self) -> None:
        references = [(0.0, 0.0), (10.0, 0.0), (20.0, 0.0)]
        candidates = [(1.0, 0.0), (19.0, 0.0)]

        self.assertEqual(minimum_cost_pairs(references, candidates), [(0, 0), (2, 1)])

    def test_empty_side_has_no_pairs(self) -> None:
        self.assertEqual(minimum_cost_pairs([], [(1.0, 1.0)]), [])
        self.assertEqual(minimum_cost_pairs([(1.0, 1.0)], []), [])


if __name__ == "__main__":
    unittest.main()
