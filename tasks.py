"""Task definitions using Invoke."""

import datetime as dt
import re
import string
import sys
import webbrowser
from pathlib import Path

from invoke import task

ROOT = Path(__file__).parent
DOCS_BUILD = ROOT / "docs" / "build"
ADR_DIR = ROOT / "docs" / "adr"
ADR_TEMPLATE = ADR_DIR / "_template.md"
ADR_SLUG_PATTERN = re.compile(r"^[a-z0-9]+(-[a-z0-9]+)*$")
ADR_GLOB = "[0-9][0-9][0-9][0-9]-*.md"

# The checks .github/workflows/ci.yml runs, in the same order. Keep the two in sync.
# `maturin develop` builds the extension from the current sources; `pytest` then
# tests that build. Commands run bare rather than through `uv run`, because a uv
# sync would reinstall a cached `optopus` wheel over the fresh build (see
# docs/Makefile) -- run the tasks themselves with `uv run invoke`.
CI_CHECKS = (
    ("cargo fmt", "cargo fmt --check"),
    ("clippy", "cargo clippy --all-targets -- -D warnings"),
    ("ruff check", "ruff check ."),
    ("ruff format", "ruff format --check ."),
    ("build", "maturin develop"),
    ("pytest", "pytest"),
)

# The wheel filenames release.yml produces. MACOSX_DEPLOYMENT_TARGET is pinned
# there so these platform tags stay predictable, because README.md and the
# installation page spell them out in full rather than linking a "latest" URL
# (GitHub's /releases/latest/download/ still needs the exact filename, and ours
# carries the version).
ASSET_TEMPLATES = (
    "optopus-{v}-cp39-abi3-manylinux_2_17_x86_64.manylinux2014_x86_64.whl",
    "optopus-{v}-cp39-abi3-macosx_11_0_arm64.whl",
    "optopus-{v}-cp39-abi3-macosx_11_0_x86_64.whl",
    "optopus-{v}.tar.gz",
)

# Files carrying v-prefixed tags and versioned wheel filenames.
URL_FILES = ("README.md", "docs/source/installation.md")


def package_version():
    """The [package] version from Cargo.toml, scoped so pyo3's version is safe."""
    text = (ROOT / "Cargo.toml").read_text()
    table = re.search(r"^\[package\]\s*$(.*?)(?=^\[|\Z)", text, re.M | re.S)
    if not table:
        sys.exit("error: Cargo.toml has no [package] table")
    version = re.search(r'^version\s*=\s*"([^"]+)"', table.group(1), re.M)
    if not version:
        sys.exit("error: Cargo.toml [package] has no version")
    return version.group(1)


def validate_adr_slug(slug):
    """Reject an ADR slug that is not lowercase kebab-case."""
    if not ADR_SLUG_PATTERN.match(slug):
        sys.exit(f"error: ADR slug must be kebab-case (e.g. 'use-invoke-for-tasks'): {slug!r}")


def next_adr_number(adr_dir):
    """One past the highest existing `NNNN-<slug>.md` number, or 1 if there are none."""
    return max((int(path.name[:4]) for path in adr_dir.glob(ADR_GLOB)), default=0) + 1


def render_adr(template, number, title, date):
    """Fill the `$number`, `$title` and `$date` placeholders of the ADR template."""
    return string.Template(template).substitute(
        number=f"{number:04d}", title=title, date=date.isoformat()
    )


@task
def ci(c):
    """Run the CI checks to completion and summarize which failed."""
    failed = []
    for name, command in CI_CHECKS:
        print(f"\n==> {command}")
        if c.run(command, warn=True, pty=True).failed:
            failed.append(name)

    print("\n==> summary")
    for name, _ in CI_CHECKS:
        print(f"  {'FAIL' if name in failed else 'ok  '}  {name}")
    if failed:
        sys.exit(1)


@task
def fix(c):
    """Apply rustfmt, Ruff's auto fixes and Ruff's formatter.

    Keeps going when Ruff leaves violations it cannot fix; `invoke ci` reports them.
    """
    c.run("cargo fmt", pty=True)
    c.run("ruff check --fix .", warn=True, pty=True)
    c.run("ruff format .", pty=True)


@task(
    help={
        "output": "Sphinx builder passed to `make -C docs` (default: html)",
        "clean": "Remove previous build output first",
        "strict": "Turn Sphinx warnings into errors",
        "open": "Open the built index.html in a browser",
    }
)
def docs(c, output="html", clean=False, strict=False, open=False):
    """Build the documentation (default: HTML)."""
    # The API reference is generated from the compiled extension's docstrings, so
    # it has to be rebuilt first -- a plain `uv sync` reinstalls a cached wheel
    # and would silently document a stale build.
    c.run("maturin develop")
    if clean:
        c.run("make -C docs clean")
    sphinxopts = ' SPHINXOPTS="-W --keep-going"' if strict else ""
    c.run(f"make -C docs {output}{sphinxopts}", pty=True)
    if open:
        webbrowser.open((DOCS_BUILD / output / "index.html").resolve().as_uri())


@task(
    help={
        "slug": "Kebab-case file name part, e.g. use-invoke-for-tasks",
        "title": "Heading of the record (default: the slug)",
    }
)
def adr(c, slug, title=""):
    """Create the next Architecture Decision Record from the template."""
    validate_adr_slug(slug)
    number = next_adr_number(ADR_DIR)
    path = ADR_DIR / f"{number:04d}-{slug}.md"
    today = dt.datetime.now(tz=dt.timezone.utc).date()
    path.write_text(render_adr(ADR_TEMPLATE.read_text(), number, title or slug, today))
    print(f"Created {path.relative_to(ROOT)}")
    print("Fill in the sections.")


@task(
    help={
        "version": "The new version, e.g. 0.2.0",
        "date": "Release date for the changelog (default: today)",
    }
)
def set_version(c, version, date=None):
    """Move every version-pinned reference to a new version, for a release PR.

    The version reaches four files, and three of them embed it in wheel download
    URLs that docs.yml publishes to GitHub Pages. A typo in one of those
    filenames is a 404 no test catches, which is why this is a task rather than
    a hand edit.
    """
    if not re.fullmatch(r"\d+\.\d+\.\d+", version):
        sys.exit(f"error: {version!r} is not a bare X.Y.Z version")

    old = package_version()
    if old == version:
        print(f"Cargo.toml is already at {version}; run `invoke check-version` to verify the rest.")
        return
    today = date or dt.date.today().isoformat()
    print(f"{old} -> {version}\n")

    # Cargo.toml: only the [package] table, so the pyo3 dependency is untouched.
    path = ROOT / "Cargo.toml"
    head, sep, tail = path.read_text().partition("\n[dependencies]")
    updated = head.replace(f'version = "{old}"', f'version = "{version}"', 1)
    path.write_text(updated + sep + tail)
    print(f"  Cargo.toml        {'updated' if updated != head else 'UNCHANGED -- check by hand'}")

    # CHANGELOG.md: date the entry on the day it ships. `[ \t]*$` rather than
    # `\s*$` so the blank line under the heading survives.
    path = ROOT / "CHANGELOG.md"
    text = path.read_text()
    if re.search(r"^## +Unreleased[ \t]*$", text, re.M):
        path.write_text(
            re.sub(r"^## +Unreleased[ \t]*$", f"## {version} ({today})", text, count=1, flags=re.M)
        )
        print(f"  CHANGELOG.md      ## Unreleased -> ## {version} ({today})")
    elif re.search(rf"^## +{re.escape(version)} ", text, re.M):
        print(f"  CHANGELOG.md      already headed '## {version} (...)', left alone")
    else:
        print("  CHANGELOG.md      no '## Unreleased' heading -- CHECK THIS BY HAND")

    for rel in URL_FILES:
        path = ROOT / rel
        before = path.read_text()
        after = before.replace(f"v{old}", f"v{version}")
        for template in ASSET_TEMPLATES:
            after = after.replace(template.format(v=old), template.format(v=version))
        after = after.replace(f"optopus-{old}-", f"optopus-{version}-")
        path.write_text(after)
        changed = sum(1 for a, b in zip(before.splitlines(), after.splitlines()) if a != b)
        print(f"  {rel:17} {changed} line(s) retargeted")

    print(
        "\nNext: rebuild so Cargo.lock follows and the wheel name shows the new version,\n"
        "then open the release PR. The documented URLs stay 404 until the tag exists --\n"
        "that is expected, and why the tag goes on that PR's merge commit."
    )


@task(name="check-version")
def check_version(c):
    """Report every version reference and flag any that disagrees with Cargo.toml.

    Worth running before opening a release PR and again before tagging:
    release.yml's own check-version job compares the tag against Cargo.toml, and
    a stale wheel filename in the docs is a 404 rather than a test failure.
    """
    version = package_version()
    print(f"Cargo.toml [package] version = {version}")
    problems = []

    headings = re.findall(r"^## +(.+)$", (ROOT / "CHANGELOG.md").read_text(), re.M)
    top = headings[0] if headings else "<none>"
    print(f"CHANGELOG.md first heading  = {top}")
    if re.match(r"^\d+\.\d+\.\d+ \(", top) and not top.startswith(version + " "):
        problems.append(f"CHANGELOG.md is headed {top!r} but Cargo.toml says {version}")

    for rel in URL_FILES:
        text = (ROOT / rel).read_text()
        tags = set(re.findall(r"[/@]v(\d+\.\d+\.\d+)", text))
        wheels = set(re.findall(r"optopus-(\d+\.\d+\.\d+)[-.]", text))
        print(f"{rel:32} tags={sorted(tags) or '-'} wheel filenames={sorted(wheels) or '-'}")
        for found in sorted(tags | wheels):
            if found != version:
                problems.append(f"{rel} references {found}, Cargo.toml says {version}")

    if problems:
        print("\nMISMATCHES:")
        for problem in problems:
            print(f"  - {problem}")
        sys.exit(1)
    print("\nAll version references agree.")
