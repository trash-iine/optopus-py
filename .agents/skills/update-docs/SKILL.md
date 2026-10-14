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
- **Recording a design decision**: not user documentation; use the `adr` skill (`docs/adr/`).

Every change to a page under `docs/source/` also goes to its Japanese translation under
`docs/source-ja/` in the same PR (step 2b).

## 2. Add a page

- Write it as Markdown (MyST) in `docs/source/`.
- Add it to the matching `{toctree}` in `docs/source/index.md`. A missing entry shows up in
  step 3 as "document isn't included in any toctree".
- Setup and installation steps belong in `README.md` / `installation.md`; do not duplicate them
  elsewhere (CONTRIBUTING.md, "Documentation", on the top page's unversioned install).
- Code examples should run as shown and print what their comments claim. Follow the rules for
  them in CONTRIBUTING.md, "Documentation", and run `uv run --no-sync pytest tests/test_docs.py`
  after a build.

## 2b. Update the Japanese edition

Follow CONTRIBUTING.md, "Japanese edition" (ADR 0008):

- Translate the English diff into the page of the same name under `docs/source-ja/`, keeping its
  glossary, its heading targets such as `(stopping)=`, and the example code unchanged apart from
  comments. A new page gets its translation and an entry in the toctree of
  `docs/source-ja/index.md`; Japanese pages that linked to it on the English site link to it
  directly instead.
- Run `uv run --no-sync pytest tests/test_docs.py`. A `test_translation_follows_the_english_page`
  failure names the new hash: set it as the page's `source_sha256` only once the translation
  matches the English page.
- If the user does not want the translation done now, say that the test will fail until it is.

## 3. Build

```bash
uv run invoke docs --strict
```

This rebuilds the extension first, because autodoc reads the compiled module's docstrings, and
builds both editions. `--strict` turns warnings into errors: fix them and rebuild. Output goes to
`docs/build/html/` and `docs/build/html/ja/` (untracked); add `--open` to show it to the user,
`--clean` to rule out stale output.

## 4. Report

List the pages and docstrings changed and the build result. Mention that merging to `main`
deploys the site to GitHub Pages if that matters to the user.
