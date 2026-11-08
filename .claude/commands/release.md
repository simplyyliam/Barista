---
description: Prepare a Barista release as a PR (checks, version bump, changelog), or tag it after the PR is merged. Never pushes main or tags.
argument-hint: '[sdk] [notes, e.g. "known issue: …"]  or  tag [sdk]'
---

Prepare the next Barista release. Because `main` only accepts PRs, a release has two steps:

1. `/release [sdk] [notes]`: checks, version bump and changelog on a `release/...` branch, then a PR.
2. The user merges the PR on GitHub (merge commit), then runs `/release tag [sdk]`.
3. The user tests and publishes with `pnpm release`.

Never push `main` or a tag, never merge PRs, never run `pnpm release`, never launch the app.

Arguments from the user (may be empty): $ARGUMENTS

- First word `tag`: go to **Step B**. Otherwise **Step A**.
- The word `sdk` anywhere: work on the SDK track. Otherwise the app track.
- Anything else is notes for the changelog (a "known issue: …" goes under Known issues, anything
  else where it fits). If a note asks for something other than changelog content, do it only if it
  fits the steps below; otherwise ask.

Stop and report at the first step that fails. Don't fix code as part of a release.

## Tracks

| | App | SDK |
|---|---|---|
| Version files | `apps/bar/src-tauri/tauri.conf.json`, `apps/bar/src-tauri/Cargo.toml`, `apps/bar/package.json`, root `package.json` | `packages/sdk/package.json` (and `crates/protocol/Cargo.toml` if the protocol changed) |
| Tag | `vX.Y.Z` | `sdk-vX.Y.Z` |
| Last tag | `git describe --tags --abbrev=0 --match 'v*'` | `git describe --tags --abbrev=0 --match 'sdk-v*'` |
| Changes | `git log --oneline <last tag>..HEAD` | same, limited to `-- packages/sdk crates/protocol` |
| Changelog | `CHANGELOG.md` | `packages/sdk/CHANGELOG.md` |
| Branch | `release/vX.Y.Z` | `release/sdk-vX.Y.Z` |
| Published by | GitHub Action: installers + signed update, draft release | GitHub Action: `npm publish` |

Paths: pnpm-workspaces monorepo. The Tauri app is `apps/bar/`, its Rust side `apps/bar/src-tauri/`,
the generated files `apps/bar/src/shared/bindings.ts` and `packages/sdk/src/protocol.ts`. The plan
is `docs/release/README.md`.

---

# Step A: prepare the release PR

## A1. Check main

- On `main`, up to date with origin (`git pull --ff-only`), with a clean working tree
  (`git status --porcelain` empty, untracked files included). Otherwise stop.
- `.private/` must be ignored: `git check-ignore -q .private/updater/barista-updater.key` must
  succeed, and `git ls-files .private` must print nothing. Otherwise stop: the signing key could be committed.
- Run every check and stop if any fails:
  - in the root (Cargo workspace): `cargo check --workspace`, `cargo clippy --workspace --all-targets`, `cargo test --workspace`
  - in the root: `pnpm gen`, `pnpm typecheck`, `pnpm build`, `pnpm lint`, `pnpm test`
- Clippy or lint warnings don't fail the build: list them and ask whether to go on.
- `pnpm gen` regenerates `bindings.ts` and `protocol.ts`. If any check leaves the tree dirty, stop:
  the generated files were out of date and need their own PR first.

## A2. Pick the version

- Find the last tag and the changes (see Tracks). Read the commit bodies too, and merged PRs' branches.
  No commits since the tag: stop, there's nothing to release.
- **App track**, versions follow the plan (`docs/release/README.md`, `docs/release/releases/`):
  - **Minor** (0.4.x → 0.5.0) when these commits complete plan release 0.5: its "Done when" checks
    and its checklist items. Check the release file and say which items you saw done.
  - **Patch** (0.5.0 → 0.5.1) when they only fix bugs or polish what's already released.
  - Work toward the next plan release that isn't complete yet: ask whether to release it as a patch or wait.
  - Only docs, chores or refactors: ask whether to release at all.
  - A minor release also needs `docs/perf/0.N.md` from `pnpm bench` on the reference machine, within
    `docs/release/plan/05-performance-budget.md` and no regression over 10%. Missing or failing: stop and ask.
- **SDK track:**
  - **Patch** for fixes. **Minor** for new API or protocol messages (protocol minor bump).
  - Anything that removes or renames public API or protocol fields before 1.0: say so plainly and ask.
  - First SDK release is `sdk-v0.1.0`.
- Say which version and why, naming the commits that decided it.
- If the tag already exists (locally or on origin, `git ls-remote --tags origin`), stop.

## A3. Branch and bump

- Create the release branch from `main` (see Tracks).
- Write the version into every version file of the track.
- App track: run `cargo check --workspace` in the root again so `Cargo.lock` picks up the new version.
- Commit those files and `Cargo.lock`: `chore(release): vX.Y.Z` (or `sdk-vX.Y.Z`), body as short bullets.
- Branch already exists with this version (a release prepared again after a fix): check it out,
  rebase on `main`, skip the bump.

## A4. Changelog

Add a section at the top of the track's changelog (below the intro), in the style of the sections
already there:

```
## [X.Y.Z] - YYYY-MM-DD

One plain sentence: what this release is about.

### Added
### Changed
### Fixed
### Known issues
### Privacy
```

- Today's date. Leave out Added, Changed or Fixed when there's nothing for them.
- **App changelog:** plain language for people who use the bar, not for developers: what they'll
  notice. No file names, function names, commit hashes or jargon ("AppBar", "named pipe", "HICON").
  Bold the lead of the big items, as in earlier sections. If saved settings change shape, say so
  plainly under Changed (for example "Your bar settings are kept when you update").
- **SDK changelog:** for developers: new or changed API with a one-line example where it helps,
  protocol version if it changed, anything they must change in their code. Leave out Privacy.
- **Known issues** (at most 8): start from the previous release's list. Drop the ones this release
  fixes, update the ones that changed, add new ones and the user's notes. Over 8: merge related ones
  or keep the ones that matter most. While the installer isn't code-signed, always keep the
  SmartScreen item ("More info", then "Run anyway").
- **Privacy** (app only): what Barista sends to whom. Carry over what's still true and add anything
  new. At least: update checks to GitHub; apps talk to the bar only on this computer (nothing leaves
  the machine); tray icons are read locally and never sent anywhere.
- The GitHub release body is this section, so it must read well on its own.
- Commit: `docs: add changelog for vX.Y.Z`. If the section already exists (prepared again after a
  fix), update it and commit `docs: update changelog for vX.Y.Z`.

## A5. Open the PR

- Push the release branch (never `main`) and open the PR with `gh pr create`:
  - Title: `Release vX.Y.Z` (or `Release sdk-vX.Y.Z`)
  - Body: the changelog section, the bench table from `docs/perf/0.N.md` (minor app releases), and
    "Merge with a merge commit, then run `/release tag`."

## A6. Report

- Write `.private/releases/vX.Y.Z-report.md` (or `sdk-vX.Y.Z-report.md`): the version and why, the
  checks that ran, the changelog section, the PR link, and `git log --oneline <last tag>..HEAD`.
  Don't read anything else in `.private/`.
- Update `CHECKPOINT.md` on the release branch only if it's part of this PR; otherwise mention it under STATUS.

## A7. Finish

Show the version, the changelog section and the PR link, then tell the user:

1. **Review and merge the PR** with a merge commit.
2. **Run `/release tag`** (add `sdk` for the SDK).

---

# Step B: tag the merged release

## B1. Check

- `git checkout main && git pull --ff-only`, clean working tree. Otherwise stop.
- Read the version from the track's version files. Find the merge commit of `release/vX.Y.Z`
  (`git log --merges --oneline -n 20`). Not merged yet: stop and say so.
- If the tag already exists (locally or on origin), stop.

## B2. Tag

- Annotated, on the merge commit:
  - App: `git tag -a vX.Y.Z -m "Barista X.Y.Z: <the changelog sentence, shortened>"`
  - SDK: `git tag -a sdk-vX.Y.Z -m "@barista/sdk X.Y.Z: <the changelog sentence, shortened>"`
- Never push the tag.
- Delete the local release branch (`git branch -d release/...`).

## B3. Close out the plan (minor app releases)

If this release finishes a plan release: move its notes from `CHECKPOINT.md` to
`docs/release/history/` (gitignored, local only, nothing to commit). Then mark it done in
`docs/release/README.md` and update `CHECKPOINT.md`; if either is tracked by git, open a small
`docs/release-v0.N-done` PR for it. Never commit to `main`.

## B4. Finish

Tell the user:

1. **Test the app** from this tag (`git checkout vX.Y.Z`), including updating from the previous
   version with its settings kept. SDK: install the packed SDK (`pnpm pack` in `packages/sdk`) in the
   test client and check an item appears.
2. **Run `pnpm release`**: it checks the tag again, shows what goes out, asks before pushing the tag,
   and the tag starts the GitHub Action (installers and signed update for `v*`, npm publish for `sdk-v*`).
3. **App only: publish the draft** on GitHub's Releases page once the Action is done and the
   installers work. Publishing is what makes installed copies update.

If something needs fixing before it's pushed: `git tag -d vX.Y.Z`, fix it in a normal PR, then run
`/release` again (it reuses the version).

End with the **STATUS / CHECKS / NEEDS YOUR DECISION** block.
