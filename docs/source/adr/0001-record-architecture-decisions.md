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
- **ADRs under `docs/source/adr/`**: one file per decision, published with the Sphinx docs, and
  scaffolded by `invoke adr`. This is what python-project-template, which this repository is
  based on, does.

## Decision

Use ADRs. Each is `docs/source/adr/NNNN-<slug>.md` with the sections Context / Options considered /
Decision / Consequences, created with `uv run invoke adr <slug> --title "<title>"`. CONTRIBUTING.md
holds the rules, ADRs hold the reasons.

## Consequences

- A PR that changes development rules, tooling, dependency choices or architecture adds an ADR.
- Accepted ADRs are not rewritten. A changed decision gets a new ADR, and the old one's status
  becomes `Superseded by NNNN`.
- ADRs are part of the Sphinx site, so `invoke docs --strict` checks them.
