# Architecture decision records

Decisions about conventions, tooling, dependencies and architecture — the ones someone will later
ask "why is it like this?" about — are recorded here as Architecture Decision Records (ADRs), one
`NNNN-<slug>.md` file each. [CONTRIBUTING.md](../../CONTRIBUTING.md) states the rules; these
records keep the reasoning and the options that were turned down.

They are for contributors, so they live outside `docs/source/` and are not part of the published
user documentation. Create a new one with `uv run invoke adr <slug> --title "<title>"`;
`_template.md` is the template it expands.
