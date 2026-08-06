"""Task definitions using Invoke."""

import sys

from invoke import task


def check_env(c):
    """Ensure the development environment (.venv) exists."""
    result = c.run("test -d ./.venv", warn=True)
    if result.ok:
        return

    uv_check = c.run("command -v uv", warn=True)
    if uv_check.failed:
        print("Error: 'uv' is not installed. Please install 'uv' first.")
        sys.exit(1)

    print("Setting up the development environment: 'uv sync --dev'")
    c.run("uv sync --dev")


@task
def docs(c, output="html"):
    """Build the documentation (default: HTML)."""
    check_env(c)
    # The API reference is generated from the compiled extension's docstrings, so
    # it has to be rebuilt first -- a plain `uv sync` reinstalls a cached wheel
    # and would silently document a stale build.
    c.run("uv run maturin develop")
    c.run(f"make -C docs {output}", warn=True)
