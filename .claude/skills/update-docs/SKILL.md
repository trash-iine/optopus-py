---
name: update-docs
description: Update the Sphinx documentation — add or edit pages under docs/source, keep the toctree complete, refresh docstrings that feed the API reference, and verify with a strict build. Use when asked to document something, add a docs page, or check that the docs build.
allowed-tools: Bash(uv run *) Read Write Edit
---

# Update the docs

Update the Sphinx + MyST documentation following CONTRIBUTING.md, "Documentation".

## 1. Decide what changes

Do every item that applies:

- **API reference**: it is generated from the `///` docstrings on the `#[pyclass]` /
  `#[pymethods]` items in `src/*.rs`. Edit those (Google style, Python types such as
  `int | None`), not a page under `docs/`. A new class also needs its own `.. autoclass::` entry
  in the matching section of `docs/source/api_reference.md`.
- **New page**: step 2.
- **Editing an existing page**: go straight to step 3.
- **Recording a design decision**: use the `/adr` skill instead.

## 2. Add a page

- Write it as Markdown (MyST) in `docs/source/`.
- Add it to the `{toctree}` in `docs/source/index.md`. A missing entry shows up in step 3 as
  "document isn't included in any toctree".
- Setup and installation steps belong in `README.md` / `installation.md`; do not duplicate them
  elsewhere.
- Code examples should run as shown and print what their comments claim.

## 3. Build

```bash
uv run invoke docs --strict
```

This rebuilds the extension first, because autodoc reads the compiled module's docstrings.
`--strict` turns warnings into errors: fix them and rebuild. Output goes to `docs/build/html/`
(untracked); add `--open` to show it to the user, `--clean` to rule out stale output.

## 4. Report

List the pages and docstrings changed and the build result. Mention that merging to `main`
deploys the site to GitHub Pages if that matters to the user.
