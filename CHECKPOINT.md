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

## "Done when" evidence (2026-11-19)
- **Fresh install:** the built NSIS installer installed per-user without admin (22 s); the installed bar ran, logged to
  `%LOCALAPPDATA%\dev.barista.app\logs`, and the test client's item appeared in the registry (`s10/timer ... icon menu "25:00"`) and went away
  when the client was killed. Uninstall stopped the bar and removed the program, the registry entry and the startup entry.
- **Update to a newer build:** two throwaway builds (0.10.1, 0.10.2, signed with a disposable key, local http server, never released).
  The installed 0.10.1 asked the server 31 s after start; Settings > General offered "Barista 0.10.2 is ready"; "Update now" downloaded it,
  the old bar shut down cleanly, the installer ran, and the new bar started (product version 0.10.2). All driven through the real Settings window.
- **Update, second run (builds that include `get_pending_update`):** the background check found 0.10.2 on its own; Settings, opened
  afterwards for the first time, already said "Barista 0.10.2 is ready" with "Update now" (no manual check); one click and the installed and
  running exe were 0.10.2.
- **Third run, on builds from the final commit (5e75af6):** scripted end to end: install 0.10.1, the bar found 0.10.2 by itself, a second launch
  handed over to the running bar (still one process) and opened Settings, General said "Barista 0.10.2 is ready" with "Update now", one click
  updated and restarted into 0.10.2, then uninstall left no bar and no registry entry.
- **Not exercised (needs your repo, secrets and a tag push, which I must not do):** the release Action itself (signed installer + `latest.json` into a draft in
  `barista-releases`, `sdk-v*` npm publish) and an update served from a real GitHub release instead of my local server.
- **Clean checkout:** a fresh `git clone` of the pushed branch: `pnpm install --frozen-lockfile` 145 s, typecheck, 157 tests and eslint pass,
  `pnpm web:build` 107 s and `out/demo` is in the output.
- **README to working item:** `npm install` took 24 s; the README snippet put a ticking `Clock` item in the bar and removed it when stopped.

## Fix next
(Final-state `/code-review` also fixed: a failed install now relaunches the bar after releasing the single-instance lock; the updater re-checks "installing" after its request; `BARISTA_DEBUG` raises the log level.)
Found by dogfooding, the installer tests and the two reviews. Nothing here blocks the PR.
1. **Publish a new SDK before announcing.** The repo README and docs describe things the published `@baristabar/sdk@0.1.1` does not have
   (`item.ready`, `StatusBar.isAvailable()`, ...). The five-minute README path works on 0.1.1; the docs pages for menus and popovers should be
   re-checked against whatever version goes out (`sdk-v*` tag).
2. **The test client starts in about 15 s.** `@hugeicons/core-free-icons` is a huge import under Node's type stripping. The README example
   avoids icons; consider a lighter import in `examples/test-client` and a note in the docs.
3. **Uninstall time varies a lot:** 22 s, 45 s, 22 s and 3 s in four runs (the 3 s one had the bar already closed by the update), and a silent `uninstall.exe /S` run without `_?=` looked like it did nothing for 27 s. Find where the time goes
   (probably the NSIS uninstaller waiting on the bar or on deleting the WebView2 profile).
4. ~~Settings did not show an update found before it was opened~~ fixed with `get_pending_update` and re-tested on an installed build.
5. **Taskbar restore on uninstall** uses a null window handle (like the panic hook). Not tested on a PC where Barista had forced auto-hide on.
6. **Unsigned installer:** SmartScreen warns on first run (documented). No EULA yet; privacy wording needs your eyes.
7. **Memory 138 MB vs the 120 MB budget** (`docs/perf/0.10.md`). The pass was measured: ~3/4 is WebView2 browser + renderer, driven by the always-on windows. Saving real memory means lazy menu-sub and panel windows (slower first open): **your call**.
8. **Not tested:** Windows 10, no monitor at launch, the release workflow itself (needs the repo, token and key), a real phone for the site.
9. **Silent first minute:** a script started before the bar prints its "Waiting for the Barista bar" line only after about 10 s.
10. **Landing page leftovers:** the Monitors tile, the Settings sliders and the Windows-mode switches are still hand-made; the demo cannot show the
    settings window yet. The explorer and videos are v0.10.x.
11. **Review notes left as they are:** the demo-settings test writes `apps/bar/demo/settings.json` (same pattern as `bindings.ts`); the
    installer kills the bar by image name.

## Useful facts

- Write scripts with the Write tool; Bash heredocs with apostrophes break here. No Python on this machine: use Node (`.mjs`).
- Headless Edge works for visual checks (`msedge --headless=new --screenshot`); the Chrome extension was not connected.
  A tiny CDP driver (click, eval, screenshot) lives only in the session scratchpad.
- `docs/release/plan`, `releases` and `history` are git-ignored (private); `docs/decisions` and `docs/perf` are tracked.
