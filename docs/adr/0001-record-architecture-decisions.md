# 0001. Record architecture decisions

- Date: 2026-09-28
- Status: Accepted

## Context

The development rules live in CONTRIBUTING.md (and its digest for AI agents, AGENTS.md), but
**why** a rule was adopted, and which alternatives were rejected, is not written down anywhere
except scattered PR descriptions and commit messages. Without that, a rule cannot be re-evaluated
when circumstances change, the same discussion gets repeated, and a contributor — human or AI —
may break a rule without knowing what it protects.

## Options considered

- **Put the reasoning in CONTRIBUTING.md**: one place to look, but the rule list fills up with
  history, and recording rejected options bloats it further.
- **Keep relying on PR descriptions and commit messages**: already happens, but the reasoning is
  split per PR and hard to find later.
- **ADRs inside the Sphinx site (`docs/source/adr/`)**, as python-project-template, which this
  repository is based on, does: one file per decision, scaffolded by `invoke adr`, and checked by
  the strict docs build. But the published site is documentation for users of the package, and
  records about development practice do not belong in it.
- **ADRs under `docs/adr/`, outside the Sphinx source**: the same files and `invoke adr`, read on
  GitHub next to CONTRIBUTING.md instead of on the published site.

## Decision

Use ADRs, kept outside the Sphinx source. Each is `docs/adr/NNNN-<slug>.md` with the sections
Context / Options considered / Decision / Consequences, created with
`uv run invoke adr <slug> --title "<title>"`. CONTRIBUTING.md holds the rules, ADRs hold the
reasons.

## Consequences

- A PR that changes development rules, tooling, dependency choices or architecture adds an ADR.
- Accepted ADRs are not rewritten. A changed decision gets a new ADR, and the old one's status
  becomes `Superseded by NNNN`.
- ADRs are for contributors, so they stay outside `docs/source/`: the published Sphinx site is
  documentation for users of the package.
