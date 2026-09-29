"""Shared paths for the tests: the repository's public files they read."""

from pathlib import Path

import pytest

REPO = Path(__file__).resolve().parents[3]


@pytest.fixture
def repo() -> Path:
    """The repository's root."""
    return REPO
