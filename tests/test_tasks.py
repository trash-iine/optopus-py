"""The ADR helpers in tasks.py."""

import datetime as dt

import pytest

# CI's Python job installs only the extension and pytest, not the dev group.
pytest.importorskip("invoke")

import tasks


@pytest.mark.parametrize("slug", ["use-invoke", "adopt-ruff", "v2", "a-1-b"])
def test_validate_adr_slug_accepts_kebab_case(slug):
    tasks.validate_adr_slug(slug)


@pytest.mark.parametrize("slug", ["", "Use-Invoke", "use_invoke", "-use", "use-", "use--invoke"])
def test_validate_adr_slug_rejects_anything_else(slug):
    with pytest.raises(SystemExit):
        tasks.validate_adr_slug(slug)


def test_next_adr_number_starts_at_one(tmp_path):
    assert tasks.next_adr_number(tmp_path) == 1


def test_next_adr_number_follows_the_highest_record(tmp_path):
    for name in ["0001-first.md", "0007-seventh.md", "index.md", "_template.md"]:
        (tmp_path / name).touch()
    assert tasks.next_adr_number(tmp_path) == 8


def test_render_adr_fills_every_placeholder():
    rendered = tasks.render_adr(
        "# $number. $title\n\n- Date: $date\n", 4, "Adopt Ruff", dt.date(2026, 9, 28)
    )
    assert rendered == "# 0004. Adopt Ruff\n\n- Date: 2026-09-28\n"


def test_the_shipped_template_renders():
    rendered = tasks.render_adr(tasks.ADR_TEMPLATE.read_text(), 1, "T", dt.date(2026, 1, 1))
    assert rendered.startswith("# 0001. T\n")
    assert "$" not in rendered
