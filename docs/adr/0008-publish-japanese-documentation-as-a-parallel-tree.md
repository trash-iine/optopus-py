# 0008. Publish Japanese documentation as a parallel tree

- Date: 2026-10-14
- Status: Proposed

## Context

The user documentation exists only in English, and the project has Japanese users who would
read it more easily in Japanese. Everything in the repository is written in English, so a
translation is an exception to that convention, and it needs a way to stay in step with the
English pages: about 3,400 lines of prose and over seventy runnable examples that change with
the API.

## Options considered

- **gettext through sphinx-intl.** The English pages stay the only source and the translation
  lives in `.po` catalogs, one entry per paragraph. Sphinx marks an entry fuzzy when its English
  paragraph changes, and the docstrings of the API reference can be translated too. But code
  blocks are not translated, so the examples keep English comments; the translation cannot
  depart from the English paragraph structure; `.po` diffs are hard to review; and it adds
  sphinx-intl and a catalog update step to every docs change.
- **A parallel MyST tree.** `docs/source-ja/` holds a Japanese page for each English page, with
  the same file names, built as a second Sphinx project. A page reads as natural Japanese, code
  comments included, and its examples run under the same test as the English ones. Nothing in
  Sphinx notices when an English page changes, so that has to be checked separately.

## Decision

A parallel MyST tree in `docs/source-ja/`, published under `ja/` inside the English site.

- `docs/source-ja/conf.py` imports `docs/source/conf.py` and overrides only the language and
  the paths. Each sidebar links to the same page in the other edition, or to its top page while
  the page is untranslated.
- Each Japanese page records, in a `source_sha256` front-matter field, the hash of the English
  page it translates. `tests/test_docs.py` fails when the English page no longer matches, so an
  English change and its translation land in the same pull request. Version numbers are left
  out of the hash, because `invoke set-version` rewrites both editions of `installation.md`.
- `tests/test_docs.py` runs the Japanese pages' examples like the English ones (ADR 0007).
  Their code is the English code; only comments and prose are translated.
- Translated headings carry a target named after the English heading's anchor, such as
  `(stopping)=`, so both editions have the same URL fragments.
- The API reference stays English: its Japanese page lists the same classes, and autodoc shows
  the English docstrings.

## Consequences

- Docs changes cost more: editing an English page means updating its translation, and a new
  page needs a Japanese page and a toctree entry in both editions.
- The test catches that an English page changed, not what changed in it. Whoever updates the
  translation reads the English diff.
- `docs/source-ja/` is the one place in the repository not written in English.
- The tree can be filled page by page: untranslated pages are linked from the Japanese pages to
  the English site until their translation lands.
