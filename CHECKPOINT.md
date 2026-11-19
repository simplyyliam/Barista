# Checkpoint

Keep this file short: where we are, the next step, and facts a new session needs.
When a release ships, move its notes to `docs/release/history/` and start fresh.

## Where we are (2026-11-19)
- **v0.9.0 shipped** (tag `v0.9.0`, not pushed yet: Liam runs `pnpm release`, then publishes the draft).
  Notes in `docs/release/history/v0.9-checkpoint.md` (local only).
- **v0.10 "Ship it" is on `feature/v0.10-ship-it`**, PR "v0.10 Ship it" (never merged by Claude). Scope was widened by Liam:
  menu bar only (no dock), UI in scope, production cleanup, real components on the landing page, no Pro paywall during the beta.
- **v0.10.x (Liam iterates):** product videos and the highlights explorer (`TourSection` stays commented out in `apps/web/src/app/page.tsx`),
  the remaining hand-made landing pieces (Monitors tile, Settings sliders, Windows-mode switches), then two more releases before v1.
- **Spec departures Liam confirmed (2026-11-13), do not "fix" them:** icon-only pills are 28px wide, the battery is
  `examples/battery`'s drawing, the hover is a 999px capsule.

## What v0.10 added
- **Distribution (ADR 0038):** release Action writes a *draft* into the public `simplyyliam/barista-releases` repo; updater module
  (`updater.rs`, off until the public key exists); per-user NSIS installer with hooks (`installer-hooks.nsh`, `--uninstall-cleanup`).
- **Hardening:** single instance, rotating logs (Settings > General > Log files), `eprintln` replaced with `log`, release profile
  (LTO, size, strip, unwind), CI advisories check and `clippy -D warnings`, version-sync test, dead frontend code removed.
  Production code has no `.unwrap()` (all 308 are in tests).
- **Website:** docs (getting started, install, menus, popovers, Electron, protocol spec, scripts, app menus), README, theme toggle,
  dark landing page, the real bar UI as demo frames (ADR 0037), phone layout fixes.
- Commands/events added: `checkForUpdate`, `installUpdate`, `openLogsFolder`, event `updateAvailable`, setting `behavior.autoUpdate`
  (all wired in Settings > General).

## Liam's to-dos before the first public release
1. Create the public `simplyyliam/barista-releases` repo **with one commit** (a README: GitHub cannot create a release in an empty repo); add repo secrets `RELEASES_TOKEN`, `TAURI_SIGNING_PRIVATE_KEY`,
   `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`, `NPM_TOKEN`.
2. Run `pnpm updater:keygen` (puts the public key in `tauri.conf.json`; until then updates are off).
3. Make the code repo private when ready. Vercel: root `apps/web`, build command `pnpm build` (it builds the demo first).
4. Review the privacy wording in the docs; an EULA is not written yet.
5. Bring back before v1.0: Pricing/Pro (`docs/release/plan/hidden-on-website.md`) and the hidden settings pages
   (`docs/release/plan/hidden-in-settings.md`). Both files are local only.

## Fix next
(filled in by the dogfooding pass at the end of v0.10)

## Useful facts

- Write scripts with the Write tool; Bash heredocs with apostrophes break here. No Python on this machine: use Node (`.mjs`).
- Headless Edge works for visual checks (`msedge --headless=new --screenshot`); the Chrome extension was not connected.
  A tiny CDP driver (click, eval, screenshot) lives only in the session scratchpad.
- `docs/release/plan`, `releases` and `history` are git-ignored (private); `docs/decisions` and `docs/perf` are tracked.
