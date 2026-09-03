"""Minimum-cost pairing for repeated visual elements.

Text alone is not an identity: charts, legends, tables, and axes routinely
repeat the same label. Pair repeated instances by their position or geometry
feature vectors so one displaced instance cannot disappear from a report.
"""

from __future__ import annotations

import math
from collections.abc import Sequence


Vector = Sequence[float]


def minimum_cost_pairs(
    references: Sequence[Vector], candidates: Sequence[Vector]
) -> list[tuple[int, int]]:
    """Return a minimum-total-distance one-to-one assignment.

    The Hungarian algorithm handles rectangular groups and remains cheap for
    pages containing many identical table values. Returned indexes always
    address ``references`` first and ``candidates`` second.
    """
    if not references or not candidates:
        return []

    swapped = len(references) > len(candidates)
    rows = candidates if swapped else references
    columns = references if swapped else candidates
    costs = [[math.dist(row, column) for column in columns] for row in rows]

    # Rectangular Hungarian algorithm, with rows <= columns.
    row_count = len(rows)
    column_count = len(columns)
    u = [0.0] * (row_count + 1)
    v = [0.0] * (column_count + 1)
    column_to_row = [0] * (column_count + 1)
    previous_column = [0] * (column_count + 1)

    for row_index in range(1, row_count + 1):
        column_to_row[0] = row_index
        current_column = 0
        minimum = [math.inf] * (column_count + 1)
        used = [False] * (column_count + 1)
        while True:
            used[current_column] = True
            current_row = column_to_row[current_column]
            delta = math.inf
            next_column = 0
            for column_index in range(1, column_count + 1):
                if used[column_index]:
                    continue
                reduced = (
                    costs[current_row - 1][column_index - 1]
                    - u[current_row]
                    - v[column_index]
                )
                if reduced < minimum[column_index]:
                    minimum[column_index] = reduced
                    previous_column[column_index] = current_column
                if minimum[column_index] < delta:
                    delta = minimum[column_index]
                    next_column = column_index
            for column_index in range(column_count + 1):
                if used[column_index]:
                    u[column_to_row[column_index]] += delta
                    v[column_index] -= delta
                else:
                    minimum[column_index] -= delta
            current_column = next_column
            if column_to_row[current_column] == 0:
                break
        while True:
            next_column = previous_column[current_column]
            column_to_row[current_column] = column_to_row[next_column]
            current_column = next_column
            if current_column == 0:
                break

    pairs = [
        (row_index - 1, column_index - 1)
        for column_index, row_index in enumerate(column_to_row[1:], start=1)
        if row_index
    ]
    if swapped:
        pairs = [(column_index, row_index) for row_index, column_index in pairs]
    return sorted(pairs)
