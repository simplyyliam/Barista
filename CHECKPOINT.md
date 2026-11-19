# Checkpoint

Keep this file short: where we are, the next step, and facts a new session needs.
When a release ships, move its notes to `docs/release/history/` and start fresh.

## Where we are (2026-11-18)
- **v0.8.0 shipped** (tag `v0.8.0` on the merge of PR #45, not pushed yet: Liam runs `pnpm release`, then publishes the draft).
  Plan releases v0.0 to v0.8 are done; notes in `docs/release/history/` (local only, v0.8 in `v0.8-checkpoint.md`: settings window,
  first-party items, memory decision of about 140 MB, review findings left open).
- **Next:** v0.9 "Script items" (`docs/release/releases/v0.9.md`). Rust may run at most two releases ahead of the merged UI.
- **Spec departures Liam confirmed (2026-11-13), do not "fix" them:** icon-only pills are 28px wide, the battery is
  `examples/battery`'s drawing, the hover is a 999px capsule.

- **Landing page built** (branch `feature/landing-page`, sheet-landing style, `apps/web/src/components/landing/`). Screenshots are labelled
  placeholders (`shot.tsx`); `DOWNLOAD_URL` in `site.ts` points at GitHub releases. Pro list says "coming soon" (not decided).

## v0.9 Script items (done, PR open, branch `feature/v0.9-script-items`)
- Liam chose: build the CLAUDE (UI) list too; folder watching reuses `platform/windows/folder_watch.rs` + rescan (no `notify`, ADR 0034).
- Rust: `src-tauri/src/scripts/` (`parse`, `name`, `run`, `scan`, `trust`, `menu`, `actions`, manager in `mod.rs`),
  `platform/windows/job.rs` (Job Object). Registry holds script items (`script/<id>`). UI: trust panel (`script-trust`),
  label/menu color, error/lock mark, "Script Items" settings page.
- New for the UI (all wired): commands `getScripts`, `getScriptTrustRequests`, `setScriptsEnabled`, `setScriptsDefaultInterval`,
  `setScriptEnabled`, `setScriptInterval`, `answerScriptTrust(id, sha256, allow)`, `openScriptsFolder`; events `scriptsChanged`,
  `scriptTrustRequested`; `StatusItemView.color/script`, `MenuEntry.color`; settings `scripts`.
- Testing without the UI: set `BARISTA_SCRIPTS_DIR` (debug builds or `BARISTA_DEBUG`); trust file is `script-trust.json` next to
  settings (beside the override folder when overridden). `node examples/inspect/src/index.ts` lists `script/...` items (do NOT run
  `pnpm inspect` while `pnpm dev` runs: it rebuilds the SDK).
- Code review (medium) fixes done: timer restart after a failed file read, stale result after stop, PowerShell UTF-8 wrapper, content-hashed menu ids, double run after "Run again", `2fa`-style names. Security review: nothing at confidence 8+.
- Verified live on the real UI (dev bar with `--remote-debugging-port=9222`, driven over CDP, real mouse clicks, screenshots): trust
  panel shows and Allow works ("1 of 3" cycling), items go live, label/menu color, error marks, submenu, `href` (browser opened),
  `shell` (cmd wrote a file), `refresh`, "Disable" in the error menu, "Run scripts" off (items gone, no script process) and on (back,
  no prompt), "Open scripts folder", bar restart (trusted scripts return silently), edit asks again, settings page.
  Screenshots need a DPI-aware capture (the screen is 3840 wide, PowerShell is virtualized to 2560 unless `SetProcessDPIAware`).
- `pnpm bench 0.9` ran (docs/perf/0.9.md): five scripts at 10 s = 0.07% CPU. The first run found 1.06%: the thread snapshot used to resume a new process cost 80 ms of bar CPU per run, now `NtResumeProcess`.
- Gotcha: killing the dev `barista.exe` ends the whole `pnpm dev` session (vite too). The example clients (test-client, spotify, ...) from
  `pnpm dev` were not restarted.
- Fix next (dogfooding the format): all done on `feature/script-fixes-and-app-menus` (xbar ` | ` params, Store Python, inert text lines,
  last good label on error, last run + duration, duplicate message, preview-cut note, `shell=` switch, docs per interpreter). The hash-to-read
  window is a documented accepted risk (ADR 0034): closing it would lock the file against editors during runs.
- **Application menus built** (`feature/script-fixes-and-app-menus`, ADR 0036, stacked on the v0.9 branch): `src-tauri/src/appmenu/` (service, caps and cleaning,
  `entries_from`), `platform/windows/win32_menu.rs` (GetMenu + WM_COMMAND), `uia_menu.rs` (UI Automation on a worker thread: expand, read, collapse; invoke by path),
  `app_info.rs` (app name), `foreground::watch_window`. Registry keys `appmenu/<n>` (not in the item list). UI: `features/menubar/app-menus.tsx`, app name follows the app in
  front, new built-in item `app-menus` (bar editor can hide it). New for the UI (wired): `getAppMenus`, event `appMenusChanged`.
  Verified live: System Information (classic) and Windows 11 Notepad (UIA), including picking entries. Chromium/Electron/GPU apps expose nothing: name only.
  Apps with no readable menu get a generic Edit + Window menu (SendInput chords, WM_SYSCOMMAND), only for normal app windows (`is_normal_window`). Zed, Figma, Zen verified.
  App name comes from the exe's FileDescription, read up to the first NUL (Spotify fix).
  Not done: keyboard access (Alt), item icons, hover-switching between UIA titles, a faster UIA read (about 0.8 s: one cache request per item would cut it).

## Facts a new session needs

- New for the UI (slice B, not wired): snapshots `getBrightness`, `getWifi`, `getBluetooth`, `getAirplaneMode`, `getHotspot`, `getOutputDevices`,
  `getUser` (each with an event `...Changed`, full snapshot, once per frame), commands `setBrightness(id, level)`, `setWifiEnabled`, `scanWifi`,
  `connectWifi(ssid)`, `disconnectWifi`, `setBluetoothEnabled`, `setHotspotEnabled`, `setOutputDevice(id)`, `runAction(action, confirm)`
  (restart / shutDown / signOut need `confirm: true`), `openSettings(page)`, `getControlCenterMode` / `setControlCenterMode` /
  `controlCenterModeChanged`, `openWindowsControlCenter`. Opening the `control-center` panel re-reads hotspot, displays, user, airplane and scans Wi-Fi.
- Live test of slice B without the UI: start the bar with `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9222`, then evaluate
  `window.__TAURI_INTERNALS__.invoke(...)` in the main page over CDP (do the id lookup inside the page: ids contain backslashes and shell quoting
  breaks them). `pnpm inspect --system --watch` shows every service. Never run `restart`, `shutDown`, `signOut`, `lock`, `sleep` in tests.

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

- Icon sizing (branch `fix/menubar-icon-sizing`, PR #17): the SDK sets an aspect-correct width and height on icon SVGs
  (16 x 16 square, about 25 x 12 for the battery). With Liam's OK, the string branch of `Icon` in
  `packages/ui/src/icons.tsx` now draws status images at that size (max height 22) instead of a fixed 16 x 16, in its own
  commit. Still open for Liam: the bar's own `battery.tsx` / `volume.tsx` (`size={16}` on 24-box Hugeicons) have the same
  empty-border problem.

- New for the UI (v0.4b slice A, not wired yet): commands `openPanel(name, anchor, width, height)`, `closePanel`,
  `panelResized(width, height)`, `getOpenPanel`; event `panelChanged { open: PanelOpen | null }` (window `panel` loads
  `index.html`; render the route by `open.name`, call `panelResized` once measured, call `closePanel` on Esc). System:
  `getClockSettings`/`clockSettingsChanged`, `getBattery`/`batteryChanged`, `getVolume`/`volumeChanged`,
  `setVolume(level 0..1)`, `setMuted`. All three events send the full snapshot, at most once per frame.
- Panel UI (Liam allowed `apps/bar/src` for this, 2026-11-12): minimal `widgets/panel` + `features/panel` (renders the
  route name and the live battery, volume and clock values, reports its size with `panelResized`, Esc calls `closePanel`).
  The window label `panel` must be in `capabilities/default.json`, or its events fail with an ACL error. Real designs come later.
- Debug: `pnpm inspect --system [--watch]`, `--panel <name> <item id> [w h]`, `--close-panel`; `scripts/panel-latency.ps1`.
  Press the volume keys from a PowerShell script (`keybd_event` 0xAE/0xAF/0xAD) to test live volume changes.
- `pnpm dev` dies if `pnpm inspect` builds the SDK while `sdk build --watch` runs: start only the bar with
  `pnpm --filter @barista/bar dev` when testing with inspect.

## Fix next

v0.8 went through the old list. **Fixed:** `item.ready` (protocol 0.7 `item.created`), `item.updateMenuItem`, README notes
(label limit, `appName`, `checked` mirrored, accelerators are text, raw SVG), `inspect` takes `AppName/itemId`, stale bounds
of items that left the bar, `tray_set` waking the UI for owner or pid changes, the width guesses (the UI's measured widths already
replace them: `note_widths`), `trayMode` switching while running, "hide Windows' own tray icons" (setting, off by default),
monochrome app icons (setting). **Moved with a reason** (local release files):
- To `v1.0.md`: pinned pills cannot be dragged or unpinned in the bar, pin discoverability, no keyboard path in the tray panel
  (all need a design and Liam's eye); `ExplorerHooks.dll` crash, Windows 10, no monitor at launch (need a third-party app or
  hardware); friendly owner names and stable ids for tray apps whose icon id changes (G HUB): needs the executable path and
  the file's product name, a design question for the saved layout.
- To `backlog.md`: hover tint stuck, startup `TaskbarCreated` retry never seen firing, `getOpenPanel` stale after blur,
  panel behind an editor window (none reproduced, repro steps are in `history/v0.7-checkpoint.md`); messages while the Start
  menu is open (ADR 0026, a known gap); `items-changed` carrying every icon (only if a profile shows it); Raycast and Sonar
  not reacting (the messages match Explorer's, so it is the apps).

From dogfooding the v0.8 items (SDK gaps, open):
- One item cannot have a left click action and a menu: a left click always opens the menu. Mic mute wanted "click toggles, right
  click shows the device and Quit". Maybe `setMenu(menu, { on: "right" })`.
- No way to say "this item is hidden until I have something" at create time: now-playing creates the item and then calls
  `setVisible(false)`, so it flashes for a frame.
- `item.setMenu` plus `updateMenuItem` resends the whole menu for each changed entry; now-playing calls it four times per track
  change (four messages). A batch form (`updateMenuItems({ id: changes })`) would be one.
- The popover has no theme: `popover.show` carries `tone` (dark or light wallpaper), but a popover also wants the bar's accent
  and its font setting; today the page cannot know them.
- Electron apps need ~10 s to start, so their item appears late after login: nothing for the SDK to do, but `item.ready` now
  tells the app when it is really there.
- Not done in v0.8: the 8 hour soak with the system monitor on (hand it to `scripts/soak.ps1`), a full week of daily use
  (Liam), `/goal` UI: the user will build the settings window frontend himself later.

## Useful facts

- Write scripts with the Write tool; Bash heredocs with apostrophes break here.
  Python on Windows: open files with `encoding='utf-8', newline=''`.
