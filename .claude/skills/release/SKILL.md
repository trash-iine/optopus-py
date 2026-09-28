---
name: release
description: Cut a release of optopus-py — run the version bump through `invoke set-version`, open the release PR, and tag its merge commit. Use this whenever the user wants to release, cut, publish, or ship a version, bump the version, tag a version, or prepare a release ("リリースして" / "タグを打って" / "0.2.0 出して"), and consult it before touching Cargo.toml's version, the CHANGELOG heading, or any v-prefixed URL in README.md or docs/source/installation.md, because those four files have to move together and a wrong wheel filename is a 404 no test catches.
---

# Releasing optopus-py

The mechanical part is `invoke`. What this skill adds is the ordering, which is not obvious and which two workflows constrain:

- **`release.yml`** has a `check-version` job that *reads* `Cargo.toml` and fails if the tag name is not `v<version>`. A tag never sets the version, so someone commits it first.
- **`docs.yml`** deploys the Sphinx site to GitHub Pages on **every push to `main`**, and `README.md` / `docs/source/installation.md` spell out wheel URLs with the version in the filename. A bumped `main` without a matching tag publishes install instructions that 404.

Hence: **one PR moves everything version-shaped, and the tag goes on its merge commit.** That keeps the 404 window down to one CI run instead of leaving it open indefinitely. The corollary is that a version bump never rides along in a feature PR — if asked to add one, say why it belongs in its own PR and offer that instead. (This repo learned it the hard way; `git log --grep="Leave the version bump"` has the reasoning.)

## Before starting

Stop and ask if any of these is surprising:

1. **`main` is green and already contains what the release should ship.** `gh pr list --state open` — an open PR that belongs in this release means the release is premature.
2. **`CHANGELOG.md`'s `## Unreleased` has real entries.** Read them; they decide the number.
3. **The number follows from those entries.** A `### Removed` section, or anything breaking under `### Changed`, means a minor bump at 0.x (`0.1.0` → `0.2.0`), not a patch. Say which entries drove the choice so the user can push back.
4. **`git submodule status`** points at the `vendor/optopus` commit you mean — it is a path dependency, so whatever it names is what ships.

## Steps

### 1. Branch and bump

```bash
git switch main && git pull
git switch -c release/v0.2.0
uv run invoke set-version 0.2.0
```

`set-version` rewrites all four files: `Cargo.toml`'s `[package] version` (`pyproject.toml` is `dynamic = ["version"]` and reads it from there), the `## Unreleased` heading into `## 0.2.0 (today)`, and every wheel URL and `@vX.Y.Z` pin in `README.md` and `docs/source/installation.md`. Do not hand-edit those filenames — `release.yml` pins `MACOSX_DEPLOYMENT_TARGET` precisely so they stay predictable, and one wrong character is a 404 rather than a test failure.

Then read the dated changelog entry as a user would. This is the last cheap moment to fix a wrong claim: each breaking change should say what a reader has to do differently.

### 2. Verify

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
uv run maturin develop          # the printed wheel name should show the new version
uv run --no-sync pytest tests -q
uv run invoke docs
uv run invoke check-version
```

The documented URLs still point at the previous release right now, and that is correct — they only become valid once the tag exists, so do not try to curl them yet.

### 3. Open the PR

Keep it to the version move so a reviewer can see at a glance that nothing else changed.

```bash
gh pr create --base main --title "Release v0.2.0" --body "..."
gh pr checks --watch --fail-fast
```

### 4. Merge, then tag the merge commit

The tag has to land on the commit that carries the bumped `Cargo.toml`. The release branch's own head is the wrong object: under squash merge it is not in `main`'s history at all.

```bash
git switch main && git pull
uv run invoke check-version     # confirm 0.2.0 before tagging
git tag -a v0.2.0 -m "v0.2.0"
git push origin v0.2.0
```

Pushing the tag publishes a release, so **confirm with the user before pushing it** unless they have already said to carry the whole thing through.

### 5. Watch it and close the loop

```bash
gh run list --workflow=release.yml --limit 1
gh run watch <run-id>
gh release view v0.2.0 --json assets --jq '.assets[].name'
```

Four assets: three wheels and an sdist. Then check the thing the whole ordering exists to protect — that the URLs the docs now advertise actually resolve:

```bash
for u in $(grep -ohE 'https://github.com/trash-iine/optopus-py/releases/download/[^ ]+whl' \
             README.md docs/source/installation.md | sort -u); do
  printf '%s  %s\n' "$(curl -s -o /dev/null -w '%{http_code}' -L "$u")" "$(basename "$u")"
done
```

Three `200`s. Anything else means a filename in the docs disagrees with what was built.

## Optional dry run

`release.yml` accepts `workflow_dispatch`, and its `release` job is gated on `github.ref_type == 'tag'` — so a manual run builds every wheel and publishes nothing:

```bash
gh workflow run release.yml
```

Worth it when the release touches the build (a new target, a maturin or pyo3 bump, an MSRV change), because retracting a failed tag is awkward.

## When it goes wrong

- **`check-version` fails.** The tag and `Cargo.toml` disagree. Do not force it: `git push --delete origin v0.2.0`, fix the version on `main` through a PR, tag again.
- **A wheel URL 404s afterwards.** The docs name a file that was not built. Compare `gh release view --json assets` with the docs and fix the docs in a follow-up; the release itself is fine.
- **A feature PR with a version bump already landed.** `main` is advertising wheels that do not exist. Either tag immediately or revert the version part on `main` — do not leave it.
