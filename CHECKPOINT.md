# Checkpoint

Keep this file short: where we are, the next step, and facts a new session needs.
When a release ships, move its notes to `docs/release/history/` and start fresh.

## Where we are (2026-11-08)

- v0.0 Setup is built on branch `chore/setup` (PR open, waiting for Liam). Plan in `docs/release/` (v0.0 Setup to v1.0, each with a `/goal` prompt).
- Decisions so far:
  - Tauri app; Liam writes the React frontend (`apps/bar/src/`, widgets/features), Claude writes
    the Rust, SDK and generated bindings. Rust may run up to two releases ahead of the UI.
  - Client apps use their own popover windows (not our styling); a menu style (bar draws a JSON
    menu) is also offered.
  - All tray icons show in the bar (no tray opener), with collapse/expand and pinning; they keep
    their normal context menus.
  - The bar reserves screen space like macOS.
  - Public SDK `@barista/sdk`, conventional names with a `BA` prefix (`BA.StatusItem`).
  - PRs only into `main` (ruleset); releases go through `/release` → PR → `/release tag` → `pnpm release`.
- v0.0 facts: one Cargo workspace at the root (`apps/bar/src-tauri`, `crates/protocol`); `pnpm gen`
  = `cargo test --workspace export_` (writes `bindings.ts` and `protocol.ts`); specta crates are
  pinned exactly (ADR 0003); `CREDITS.md` needs a row for every new dependency (a test checks).
- UI package: shadcn lives in `packages/ui` (`@barista/ui`, preset `base-mira`). Apps import
  `@barista/ui/styles.css` from their `index.css` (it holds Tailwind, the theme and the `@source`
  line). Add components with `pnpm dlx shadcn@latest add <name> -c apps/bar`; they land in the package.
- **Next:** Liam reviews and merges the setup PR and builds `apps/bar/src/` (his structure and components). Then Claude starts v0.1 (rust-only): ADRs 0001 and 0002, bar window, AppBar.

## Fix next

- Empty.

## Useful facts (keep only what still matters)

- Write scripts with the Write tool; Bash heredocs with apostrophes break here.
  Python on Windows: open files with `encoding='utf-8', newline=''`.
