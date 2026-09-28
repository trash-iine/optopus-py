# 0004. Keep agent skills in .agents/skills

- Date: 2026-09-28
- Status: Proposed

## Context

The project's skills (`quality-check`, `create-pr`, `release`, `port-to-rust`, ...) lived in
`.claude/skills/`, so only Claude Code picked them up. They are plain Agent Skills (a directory
with a `SKILL.md` and its files, per the open specification at agentskills.io), and other coding
agents now load the same format, but from different directories. At the time of writing:

- Codex, Gemini CLI, Cursor, GitHub Copilot and OpenCode read project skills from
  `.agents/skills/`, the vendor-neutral location.
- Some of them also read `.claude/skills/`, but Codex and Gemini CLI do not.
- Claude Code reads only `.claude/skills/`.

No single directory reaches every agent, and a contributor should not have to care which agent
another contributor uses.

## Options considered

- **Leave the skills in `.claude/skills/` and point to them from AGENTS.md.** No layout change, but
  an agent that does not scan that directory sees the skills only if it follows the pointer and
  reads the files by hand, without discovery by description.
- **`.claude/skills/` as the source, `.agents/skills` a symlink to it.** Discovered everywhere,
  but the tracked files sit under one vendor's directory while the neutral path is the alias.
- **`.agents/skills/` as the source, `.claude/skills` a symlink to it.** Discovered everywhere,
  and the files live at the path the specification and most agents use; Claude Code, the one
  agent that needs the alias, gets it.
- **A copy per agent directory, kept in sync by a script or a CI check.** Works without symlinks,
  but every skill exists twice and every edit needs the sync step.

## Decision

Keep the skills in `.agents/skills/` and track `.claude/skills` as a relative symlink to
`../.agents/skills`. One copy, found by every agent above, with the neutral path as the source.

Write the skills for any agent: no frontmatter outside the specification except the experimental
`allowed-tools` (agents that do not support it ignore it), no Claude Code-only substitutions such
as `$ARGUMENTS`, refer to other skills by name ("the `adr` skill") rather than as a `/adr` slash
command, and name agent-specific tools only as an example of a generic capability.

## Consequences

- Paths to skill files in docs and skills use `.agents/skills/`.
- On Windows, git checks the symlink out as a plain file unless symlinks are enabled
  (`core.symlinks` with Developer Mode); Claude Code then finds no skills there, while agents that
  read `.agents/skills/` are unaffected. Contributors using Claude Code on Windows enable
  symlinks.
- If Claude Code starts reading `.agents/skills/`, the symlink can be removed with a new ADR.
