# Checkpoint

Keep this file short: where we are, the next step, and facts a new session needs.
When a release ships, move its notes to `docs/release/history/` and start fresh.

## Where we are (2026-11-07)

- Project not started. Plan in `docs/release/` (v0.0 Setup to v1.0, each with a `/goal` prompt).
- Decisions so far:
  - Tauri app; Liam writes the React frontend (`apps/bar/src/`, widgets/features), Claude writes
    the Rust, SDK and generated bindings. Rust may run up to two releases ahead of the UI.
  - Client apps use their own popover windows (not our styling); a menu style (bar draws a JSON
    menu) is also offered.
  - All tray icons show in the bar (no tray opener), with collapse/expand and pinning; they keep
    their normal context menus.
  - The bar reserves screen space like macOS.
  - Public SDK `@barista/sdk`, conventional names with an `BA` prefix (`BA.StatusItem`).
  - PRs only into `main` (ruleset); releases go through `/release` → PR → `/release tag` → `pnpm release`.
- **Next:** run the v0.0 Setup `/goal` (`docs/release/releases/v0.0.md`). Then Liam sets up
  `apps/bar/src/`, and Claude starts v0.1 (rust-only).

## Fix next

- Empty.

## Useful facts (keep only what still matters)

- Write scripts with the Write tool; Bash heredocs with apostrophes break here.
  Python on Windows: open files with `encoding='utf-8', newline=''`.
