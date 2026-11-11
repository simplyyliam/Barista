# Checkpoint

Keep this file short: where we are, the next step, and facts a new session needs.
When a release ships, move its notes to `docs/release/history/` and start fresh.

## Where we are (2026-11-11)

- **v0.3.0 shipped** (tag `v0.3.0` on the merge of PR #13, not pushed yet: Liam runs `pnpm release`). Plan releases
  v0.0 to v0.3 are done; notes in `docs/release/history/` (local only).
- **v0.4 Popovers: built on `feature/v0.4-popovers`, PR open for review** (not merged). Rust, protocol 0.4, SDK
  (`setPopover`, `@barista/sdk/electron`, `on("open" | "close")`), examples (`test-client`, `spotify`, `quick-note`),
  ADR 0014, bench `docs/perf/0.4.md`. v0.4b System panels is next (`releases/v0.4b.md`, local only); it reuses
  `popover.rs`, `placement.rs` and `dismiss.rs`.
- ADR 0012 (tray) still needs Liam's approval before v0.6 starts.

## Facts a new session needs

- One Cargo workspace at the root (`apps/bar/src-tauri`, `crates/protocol`). `pnpm gen` = `cargo test --workspace export_`
  (writes `bindings.ts` and `protocol.ts`). specta crates are pinned exactly (ADR 0003). `CREDITS.md` needs a row for
  every new dependency (a test checks).
- shadcn lives in `packages/ui` (`@barista/ui`, preset `base-mira`); add components with
  `pnpm dlx shadcn@latest add <name> -c apps/bar`. Icons via `@barista/ui/icons` (`<Icon>`).
- Website: `apps/web` (Next.js 16 + Fumadocs, static export, ADR 0004). Docs search off until v0.10.
- UI already has: `getBarInfo`/`setMonitor`/`barChanged`, `getItems`/`itemClicked`/`itemsChanged`,
  `getWindowRadius`/`windowRadiusChanged` (+ `--window-radius`), `getBarAppearance`/`barAppearanceChanged` (+ `data-tone`).
- New for the UI (v0.3, not wired yet): window `menu` (loads `index.html`; App must render the menu there by window
  label), `menuOpen`/`menuChanged`/`menuClose`, `menuItemClicked`, `menuResized` (returns submenu side), `closeMenu`,
  `getOpenMenu`, `setItemBounds`.
- New windows: register with `window_radius::register(app, label, Mode::Tracked | Mode::Normal)` (popover = `Normal`), and
  create them with the same `additionalBrowserArgs` as `tauri.conf.json` (else WebView2 error 0x8007139F).
- `debug.listItems` / `debug.click` (and `pnpm inspect`) work in debug builds or with `BARISTA_DEBUG` set.
- Memory budget counts private memory (62 MB vs 120 MB); `--disable-gpu --renderer-process-limit=1` keeps it there.
- Driving the menu window without the UI: start `pnpm dev` with
  `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9222`, then evaluate JS in the `menu` / `main` page over
  CDP (`window.__TAURI_INTERNALS__.invoke(...)`). Debug only, never committed.
- `pnpm bench` refuses to run while a bar is running; it needs a release build. Since v0.3 it also times menus
  (`scripts/menu-visible.ps1` polls the menu window's visibility).
- Stopping `pnpm dev` from a background task leaves vite (port 1420) and a self-restarting bar behind: kill them by
  command line (`vite.js`, `target\debugarista.exe`) before starting again.
- Never set the real wallpaper in a test (it can't be restored); `wallpaper_live` only reads.

- New for the UI (v0.4, not wired yet): event `popoverChanged { itemId | null }` and command `getOpenPopover`: show the
  item as "open" while a popover is up. Item clicks already go through `itemClicked`: nothing else changes for the UI.
- Popovers: `popover.rs` (coordinator, one popover at a time, 1.5 s `popover.shown` timeout), `dismiss.rs` (the open/close
  Machine shared with menus), `placement.rs` (bar strip, work area, item rects), `platform/windows/foreground.rs`
  (`AllowSetForegroundWindow` + `SetWinEventHook(EVENT_SYSTEM_FOREGROUND)`). Click rule in `menu_window::route_click`.
- Testing popovers by hand: `debug.click` (and `pnpm inspect --click`) shows the window but can NOT give it focus (Windows
  needs real input). Use real clicks: `scripts/popover-latency.ps1` / `popover-visible.ps1`. Electron examples: run with
  `./node_modules/.bin/electron src/main.ts`; never `await app.whenReady()` at the top level of the main file (deadlock).
- Killing a dev bar: kill `barista.exe` by PID, then check port 1420 (vite); don't match processes by command line.

## Fix next

- From dogfooding the v0.4 popovers (Spotify mini player in demo mode, quick-note, test client under Electron):
  - Spotify: I (Claude) only ran `--demo` mode, with no credentials here. Liam says the real player works (2026-11-11);
    that is his report, not something I checked.
  - Other monitors and scales: Liam tests those himself (2026-11-11). Until he does, real runs were on one monitor at
    150% only; the rest is placement unit tests (100, 125, 150, 200 % and negative coordinates).
  - An Electron app needs about 15 lines of boilerplate for the window (`show: false, frame: false, skipTaskbar,
    alwaysOnTop`, deny navigation and new windows, preload, wait for `ready`). Want `electronPopover` to accept a window
    factory, or a helper that makes the window.
  - Electron's `ready` never fires while the main file is in a top-level `await`: the first version of the test client hung.
    The README for the adapter should say so.
  - `item.on("open")` can't hold the popover back: the window shows before the app finished refreshing. Maybe
    `on("open", async)` with a short wait, or a documented "render the last state first".
  - An app can't close its own popover on purpose (a "Save" button) except `win.hide()`; the adapter now reports the
    `hide` event as a close, but a plain `item.closePopover()` would read better.
  - The window frame: an Electron frameless window measures about 7 DIP bigger than the visible rect (invisible resize
    border). It lines up visually; check with a transparent window.
  - No automated test can check focus (needs real input). Only the manual scripts do. A small input-simulating test
    helper would make "focus on every monitor and scale" checkable in CI.
  - Only one monitor at 150% was available: other monitors and scales are covered by placement unit tests, not run for real.
  - A label that grows (quick-note shows the start of the note) shifts the icon; an open popover stays where it was placed.
  - Esc is handled by the app (adapter), not the bar: another framework must do it in `PopoverWindow.onClose`.
  - Click to menu window visible is 52 / 71 ms (budget 50); `main` is 64 / 81. Private memory is 93 MB (was 74 at v0.3).
    Both are in `docs/perf/0.4.md`; not caused by v0.4 as far as measured, but worth a look before v0.5.
  - From `/code-review` and not fixed in v0.4: a late `popover.shown` from an older show is accepted for a newer show of
    the same item (needs a per-show token in the protocol); a `setPopover` size change while the popover is open is not
    applied until the next open; on a swap the UI sees no "closed" for the old item before the new one opens.
  - `pnpm dev` now also starts the test client under Electron, quick-note and the Spotify player (it uses SPOTIFY_CLIENT_ID
    from `.env`; without it the popover says so).
  - Release scripts: `pnpm test-client` was in CLAUDE.md but missing from `package.json`; added (and `test-client:electron`).

- From dogfooding the focus timer (v0.3 menus):
  - Changing one entry (Start enabled, "Resume" label) means rebuilding the whole menu and calling `setMenu` again.
    Want `item.updateMenuItem(id, { enabled, label, checked })` or setters on `MenuItem` that sync by themselves.
  - `click(entry, { checked })`: `checked` is optional even for a checkbox, so apps write `checked ?? !x`.
    `entry.checked` is already mirrored; document that and maybe drop `checked` from the event.
  - Done in v0.4: `item.on("open")` runs right before a menu opens (the menu itself is already drawn, so a changed menu
    redraws a moment later).
  - `inspect` keys change every connection (`s22/timer`, `s48/timer`); accept `appName/itemId` too.
  - Accelerators are text only; "CmdOrCtrl+Enter" next to Start suggests a hotkey that doesn't exist. Say so in docs.
  - Fixed while dogfooding: role entries default their id to the role (`quit`), separators don't use up ids.

- Silent client (sends `hello`, then nothing) holds a connection slot until it disconnects: fixed by the v0.5 heartbeat.
- SDK papercuts from dogfooding `examples/clock`:
  - Accept raw SVG markup (`<svg...`) as `icon`, maybe a file-path helper.
  - No bar running = `connect()` rejects; needs wait/retry (v0.5), README should show try/catch until then.
  - `create()` resolves on send, not on accept (`limit_reached` arrives later as `error`); consider `await item.ready`.
  - README: say a label over 128 chars throws `TypeError`; mention `appName` in the quick start.
  - Done in v0.4: `item.on("click")` gives `anchor`; `item.on("open" | "close")` exists (menus and popovers).
- Not tested on real hardware: no monitor at launch (starts on a placeholder monitor).

## Useful facts

- Write scripts with the Write tool; Bash heredocs with apostrophes break here.
  Python on Windows: open files with `encoding='utf-8', newline=''`.
