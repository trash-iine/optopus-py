"""Run the Python code blocks of the user documentation, and keep its translation in step.

Each page's ```python blocks run in order in one namespace, so a block may use what an
earlier block on the same page defined, but nothing from another page. That holds for the
English pages under docs/source and their Japanese translations under docs/source-ja
(ADR 0008), which also record which version of the English page they translate.
"""

import hashlib
import re
from pathlib import Path

import pytest

DOCS = Path(__file__).resolve().parents[1] / "docs"
ENGLISH = DOCS / "source"
JAPANESE = DOCS / "source-ja"
FENCE = re.compile(r"^```python\n(.*?)^```$", re.MULTILINE | re.DOTALL)
SOURCE_HASH = re.compile(r"\A---\n(?:.*\n)*?source_sha256: ([0-9a-f]+)\n(?:.*\n)*?---\n")
VERSION = re.compile(r"\d+\.\d+\.\d+")


def code_blocks(page):
    """Return the Python blocks of a page, in order, with the line each starts on."""
    text = page.read_text("utf-8")
    return [(text.count("\n", 0, m.start()) + 2, m.group(1)) for m in FENCE.finditer(text)]


def english_hash(page):
    """The hash a translation of `page` records, version numbers left out.

    `invoke set-version` rewrites the versioned URLs of both editions of installation.md, so
    a release is not a change the translation has to follow.
    """
    text = VERSION.sub("X.Y.Z", page.read_text("utf-8"))
    return hashlib.sha256(text.encode()).hexdigest()[:12]


PAGES = sorted(
    path for source in (ENGLISH, JAPANESE) for path in source.glob("*.md") if code_blocks(path)
)
TRANSLATIONS = sorted(JAPANESE.glob("*.md"))


def page_id(path):
    return f"{path.parent.name}/{path.name}"


def test_pages_with_code_are_found():
    assert any(page.parent == ENGLISH for page in PAGES), f"no Python code blocks under {ENGLISH}"
    assert any(page.parent == JAPANESE for page in PAGES), f"no Python code blocks under {JAPANESE}"


@pytest.mark.parametrize("page", PAGES, ids=page_id)
def test_code_blocks_run(page, tmp_path, monkeypatch):
    monkeypatch.chdir(tmp_path)
    namespace = {"__name__": "__docs__"}
    for line, source in code_blocks(page):
        exec(compile(source, f"{page_id(page)}:{line}", "exec"), namespace)


def test_every_english_page_is_translated():
    untranslated = sorted(
        page.name for page in ENGLISH.glob("*.md") if not (JAPANESE / page.name).exists()
    )
    assert not untranslated, f"no Japanese page in {JAPANESE.name}/ for {untranslated}"


@pytest.mark.parametrize("translation", TRANSLATIONS, ids=page_id)
def test_translation_follows_the_english_page(translation):
    english = ENGLISH / translation.name
    assert english.exists(), f"{page_id(translation)} has no English page {page_id(english)}"
    recorded = SOURCE_HASH.match(translation.read_text("utf-8"))
    assert recorded, f"{page_id(translation)} has no `source_sha256:` in its front matter"
    current = english_hash(english)
    assert recorded.group(1) == current, (
        f"{page_id(english)} changed since {page_id(translation)} was translated: bring the "
        f"translation up to date, then set its `source_sha256: {current}`"
    )
