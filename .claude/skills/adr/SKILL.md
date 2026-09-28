---
name: adr
description: Record a design decision made in the conversation as an Architecture Decision Record in docs/adr — generate it with `invoke adr`, fill in context, options, decision and consequences, and handle supersession. Use when a decision about conventions, tooling, dependencies or architecture has been made, or when asked to "write an ADR" or "record this decision".
argument-hint: "[slug]"
allowed-tools: Bash(uv run *) Read Write Edit
---

# Write an ADR

Record a decision from the conversation in `docs/adr/`. Rules belong in CONTRIBUTING.md; the
ADR records **the reasoning and the options considered** (CONTRIBUTING.md, "Architecture decision
records").

## 1. Choose the slug and title

- **Slug**: kebab-case (e.g. `limit-invoke-tasks-to-composite-workflows`). Use `$ARGUMENTS` if
  given.
- **Title**: one sentence stating the decision (e.g. "Limit Invoke tasks to composite workflows").
- Check the existing records in `docs/adr/` for one this decision replaces.

Show both to the user and confirm before continuing.

## 2. Generate the file

```bash
uv run invoke adr <slug> --title "<title>"
```

This numbers the record and expands the template into `docs/adr/NNNN-<slug>.md`.

## 3. Fill it in

Replace each `<!-- -->` prompt with the content it asks for.

Under "Options considered", list only options that were **actually compared** in the
conversation. Do not invent alternatives.

Set the status to `Accepted` if the user agreed to the decision; leave it `Proposed` if it is
still under discussion.

## 4. Supersession

If this replaces an earlier decision, change the old record's status line to
`Superseded by NNNN` (the new number). Leave the old record's body as it is.

## 5. Report

Give the ADR's path and a one-line summary. If the decision changes a rule, note that
CONTRIBUTING.md, AGENTS.md and the affected skills need updating in the same PR.
