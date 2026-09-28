---
name: create-pr
description: Take the current changes to an open pull request following CONTRIBUTING.md — branch-name check, the CI checks, plain English commits, changelog and ADR checks, push, and `gh pr create`. Use when asked to commit and open a PR, "create a PR", or "send this for review".
allowed-tools: Bash(git *) Bash(gh *) Bash(uv run *)
---

# Create a pull request

Commit the current changes according to CONTRIBUTING.md and open a pull request.

## 1. Branch

Check where you are with `git branch --show-current` and `git status`.

- **On `main`**: never commit there. Propose a `<type>/<short-kebab-description>` name from the
  changes and switch to it first.
- **Branch name breaks the convention**: suggest `git branch -m <new-name>`.
- `type` is one of `feat|fix|docs|refactor|test|ci|chore|release`.
- A version bump does not belong in a feature PR. If the changes include one, stop and point to
  the `/release` skill.

## 2. Checks

Follow the `/quality-check` skill (`uv run invoke ci`) and continue only once everything passes.

## 3. Commit

- Imperative mood, sentence case, ≤72 characters, no gitmoji or type prefix (e.g.
  `Bind the new optopus heuristics and graph generators`). Explain *why* in the body if the
  subject cannot.
- Split independent changes into separate commits.
- User-visible change (API, behavior, install, published docs)? Make sure `CHANGELOG.md` has an
  entry under `## Unreleased`.
- Development rules changed? Make sure `CONTRIBUTING.md`, `AGENTS.md` and the affected skills are
  updated in this PR.
- A decision about conventions, tooling, dependencies or architecture? Make sure an ADR exists in
  `docs/source/adr/`; if not, create one with the `/adr` skill.

## 4. Push and open the PR

1. `git push -u origin <branch>`
2. `gh pr create --base main` with an English title (like a commit subject) and a body containing:
   - **Summary**: what changed and why; reference related issues.
   - **Checks**: the commands run in step 2 and their results, taken from the `invoke ci`
     summary. Only claim what actually passed.
   - **Sample output**: only when docs or printed output changed.
3. `gh pr checks --watch` if the user wants to wait for CI.

## 5. Report

Give the PR URL and a short summary of the commits.
