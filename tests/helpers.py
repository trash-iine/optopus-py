"""Instances and helpers shared by the test modules."""

import optopus

# The triangle's optimum cut puts one vertex alone, cutting the two heaviest edges.
TRIANGLE = [(0, 1, 1.0), (1, 2, 2.0), (0, 2, 3.0)]
TRIANGLE_OPTIMUM = 5.0

UNIT_SQUARE = [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)]


def stop(iterations):
    return optopus.StopCondition(max_iteration=iterations)
