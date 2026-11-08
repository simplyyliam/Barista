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
- Plan changed (2026-11-08): new **v0.9 Script items** (xbar-style PowerShell/Python/Node scripts become
  bar items; runner built into the bar, scripts in `%APPDATA%\Barista\scripts`, trust prompt, timeouts, folder
  watcher; demo = system monitor script);
  "Ship it" is now **v0.10**; **v1.1 Gallery** (curated `barista-scripts` repo, `barista://install`, notes only);
  the rest of v1.x is **v1.2+**. v0.8 also gets a first-party system monitor item. The plan files
  (`docs/release/plan/`, `releases/`) are git-ignored and local only; `docs/release/README.md` has the table.
- Website (2026-11-08, branch `chore/web-app`, PR open): `apps/web` = Next.js 16 + Fumadocs, static export (ADR 0004). `pnpm web:dev` / `pnpm web:build` (writes `apps/web/out/`). Landing page at `/` is a placeholder (Liam builds the real one); docs at `/docs` have two placeholder pages (content/docs/*.mdx, real content in v0.10). Docs search is off until v0.10 (static export needs a prebuilt index). Collections use `fumadocs-mdx/macro` (no source.config.ts).
- v0.1 Bar shell (2026-11-08, branch `feature/v0.1-bar-shell`, PR open once reviewed): built, rust-only.
  - Code: `bar.rs` (types + rect math, tested), `controller.rs` (state, apply loop, `bar-changed`),
    `platform/windows/{appbar,monitors,window,fullscreen}.rs`, `settings.rs`. ADRs 0001 (pipe), 0002 (AppBar).
  - Checked live (release exe, 4K screen at 150%): work area top 0 -> 48 while running, back to 0 after a clean quit
    and after `Stop-Process -Force` (Explorer cleans up itself, so no watchdog); maximized window starts under the
    bar; full-screen window hides the bar and closing it shows it again. `pnpm bench 0.1` writes `docs/perf/0.1.md`.
  - Not testable here (one monitor, 150% only): 100%/125% live, monitor switch/unplug, Explorer restart.
  - New for the UI: `commands.getBarInfo()`, `commands.setMonitor(id)`, `events.barChanged`. The window is
    transparent now, so `html, body` need `background: transparent`.
- **Next:** Liam reviews the v0.1 PR and builds the bar layout in `apps/bar/src/`. Then v0.2 (items, IPC server).

## Fix next

- Memory is over budget: the bar + WebView2 sum to ~410 MB working set (~130 MB private) vs the 120 MB budget.
  This is the WebView2 baseline with a near-empty page. Decide: change the budget to private memory, or trim.
- Not verified live yet: 100%/125% scaling, second monitor (switch and unplug), Explorer restart (`TaskbarCreated`).
- The first-run window is 800x32 until the first `apply`; fine because it starts hidden.

## Useful facts (keep only what still matters)

- Write scripts with the Write tool; Bash heredocs with apostrophes break here.
  Python on Windows: open files with `encoding='utf-8', newline=''`.
