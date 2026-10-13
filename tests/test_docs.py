"""Run the Python code blocks of the user documentation.

Each page's ```python blocks run in order in one namespace, so a block may use what an
earlier block on the same page defined, but nothing from another page.
"""

import re
from pathlib import Path

import pytest

DOCS = Path(__file__).resolve().parents[1] / "docs" / "source"
FENCE = re.compile(r"^```python\n(.*?)^```$", re.MULTILINE | re.DOTALL)


def code_blocks(page):
    """Return the Python blocks of a page, in order, with the line each starts on."""
    text = page.read_text("utf-8")
    return [(text.count("\n", 0, m.start()) + 2, m.group(1)) for m in FENCE.finditer(text)]


PAGES = sorted(path for path in DOCS.glob("*.md") if code_blocks(path))


def test_pages_with_code_are_found():
    assert PAGES, f"no Python code blocks found under {DOCS}"


@pytest.mark.parametrize("page", PAGES, ids=lambda path: path.name)
def test_code_blocks_run(page, tmp_path, monkeypatch):
    monkeypatch.chdir(tmp_path)
    namespace = {"__name__": "__docs__"}
    for line, source in code_blocks(page):
        exec(compile(source, f"{page.name}:{line}", "exec"), namespace)
