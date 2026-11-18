# Checkpoint

Keep this file short: where we are, the next step, and facts a new session needs.
When a release ships, move its notes to `docs/release/history/` and start fresh.

## Where we are (2026-11-16)
- **v0.8 "Daily driver" built, PR open** (branch `feature/v0.8-daily-driver`, Liam reviews and merges). Rust: grouped settings with
  migration (ADR 0029), `settingsChanged`, start with Windows and taskbar auto-hide (ADR 0031), blur / full screen / tone
  settings, restartable tray host (ADR 0032), protocol 0.7. SDK: `item.ready`, `item.updateMenuItem`. First-party items
  (ADR 0030): `examples/win-helper` (Rust, mic and media over JSON lines), `mic-mute`, `now-playing`, `system-monitor` (Electron,
  popover in the Found on Cosmos style). UI: settings window (frameless, own titlebar, shadcn controls, dnd-kit editors for the
  menu bar and the control center), the bar and the control center follow the saved layout. **New for the UI** is all wired.
  Left for Liam: the full week of use, the 8 hour bench (`docs/perf/0.8.md`), and a look at the settings window.
- **Release checks (2026-11-16):** `/code-review medium` (10 findings, 8 fixed: bounds of the bar's own items were dropped by
  the new stale-bounds filter, reset and import skipped autostart and tray prefs, reset all cleared the monitor, hide-on-full-screen
  ignored an app already in front, `item.ready` could hang; two left as noise: panic-hook doc comment, `blur_allowed` syscalls
  under a lock), `/security-review` (no findings), end-to-end on the release bar with fresh settings (defaults, every setting
  changed, live effect seen, clean quit, restart: settings identical; mic mute clicked through the SDK twice, monitor ticking,
  now playing waiting hidden, all items gone when the clients were killed; a real taskbar auto-hide on and off, the Run key,
  tray mode windows and back with 11 icons). The test found now-playing crashing on a menu id with capitals (fixed) and Node
  clients using 58 MB too much through the icon package's index (fixed). Not tested: now playing with real music (nothing was
  playing), a second monitor, Windows 10.
- Bar settings window pieces: `widgets/settings`, `features/settings/*` (pages, editors, `sortable.tsx`), store
  `shared/stores/settings.ts`, `shared/hooks/use-apply-settings.ts`. Liam will rework the settings frontend himself.

- **Settings window redesign (2026-11-18, `UI: full` for the settings window, same branch):** 860x600, one 13px scale, no separators,
  mono sidebar icons, slim sliders, sliding segments, dark shadcn selects, black scroll fades (`scroll-fade.tsx`), previews closed by
  default with the view pills inside, About = What's New with an animated banner (`features/settings/about/`, scenes use the
  editor's `Glyph`/`pillClass`). Also: text size and font now reach the bar (`--bar-text-size`, `--bar-font` in `tokens.css`; `@theme
  inline` had baked the values in), tray host backs off while TaskbarX runs (guess, untested with TaskbarX), brightness list drops the
  laptop panel with the lid closed (untested with a real lid). Bench: bar unchanged (156 MB private, 0.08% idle, `docs/perf/0.8.md`).
  Memory flags (ADR 0033, `--in-process-gpu`, no site isolation): bar 156 to 143 MB private, the settings window costs about 28 MB on the Menu bar page, 6 MB on About, and is given back on close (148 MB);
  `NetworkServiceInProcess` tried and dropped (no effect). Release checks 2026-11-18: `/code-review medium` (10 findings: import check,
  bounded helper read and a comment fixed; the rest left, see below), `/security-review` (no findings), end to end (item appears, gone
  8 s after the client is killed; menu click not exercised).
  **Left from the review:** tray worker not stopped on `windows` mode (icons can reappear), `blur_allowed` under the menu and panel
  locks, `ids_in_bar` snapshot on every bounds report, legacy tray/control-center commands skip `settingsChanged`, `settings.ts`
  listener race, `apply()` off the UI thread (probably fine: sync Tauri commands run on the main thread).
  **Open:** bar still over the 120 MB budget (143 MB; 130 MB accepted). Measured where it goes: the hidden panel window is about 53 MB of it
  (submenu 8 MB); fix idea in `docs/perf/0.8.md`: render nothing in the panel while it is closed (frontend, Liam's call), 8 h soak, version number still 0.7.0 while About says What's new in 0.8, blur/solid not seen
  failing (applies on the next menu open). Liam says Barista is closed source, freemium, $5 once for Pro (memory `business-model-freemium`).

- **v0.7.0 shipped** (tag `v0.7.0` on the merge of PR #42, not pushed yet: Liam runs `pnpm release`). Plan releases v0.0 to v0.7 are done; notes in `docs/release/history/` (local only, v0.7 in `v0.7-checkpoint.md`: tray clicks, pin / order / collapse, new for the UI already wired). Next: v0.8 "Daily driver" (`releases/v0.8.md`; add every new `barista | windows` switch to its list).

- **v0.6.0 shipped** (tag `v0.6.0` on the merge of PR #38, not pushed yet: Liam runs `pnpm release`). Plan releases v0.0 to v0.6 are done; notes in `docs/release/history/` (local only, v0.6 in `v0.6-checkpoint.md`: tray host, `source: app | tray`, `trayMode` / `trayStyle`, new for the UI already wired). Next: v0.7 "Tray, live" (`releases/v0.7.md`).
- **v0.5.0 shipped** (tag `v0.5.0` on the merge of PR #32, pushed). `sdk-v0.1.0` is pushed and the Action
  logged a publish; do `npm install @baristabar/sdk` in a fresh folder once to confirm it is on npm. Plan releases v0.0 to v0.5 are done;
  notes in `docs/release/history/` (local only, v0.5 in `v0.5-checkpoint.md`, with its dogfooding papercuts: `item.ready`, a
  "waiting for Barista" hint, `whenAvailable()`). **New for the UI:** command `quitBar` (for a future "Quit Barista" entry, no design yet).

- **Spec departures Liam confirmed (2026-11-13), do not "fix" them:** icon-only pills are 28px wide (3px padding, 2px gap, 30px pitch,
  not the 6-8px of the first spec); the battery is `examples/battery`'s drawing (no 27x12 construction or masks); the hover is a 999px
  capsule (not 4px).
- **Release checks done (2026-11-13, after the motion pass and panel swap):** `/code-review medium` (one real bug fixed: swap generation
  race; also volume refresh gate, icon retry spacing, dead variable), `/security-review` (no findings), end-to-end (test client item
  appears with its menu, gone after the client is killed), rapid panel switching (last click wins, one panel open). Deferred from the
  review: full session rebuild on every audio state change, hidden volume panel re-renders on events, `debug` window in the release
  capability file (the window is never created in release), `SESSIONS_PENDING` cleared by the explicit refresh.
- **Motion pass (ADR 0023, proposed)**: `motion/react`, values in `shared/motion.tsx`, research in `docs/research/ux-motion-and-taste.md`
  (local). Press 0.96, icon swaps (fade + scale + blur), items arrive/leave, panel-to-panel swap (close with Windows' fade, then open: `SWAP_DELAY` in `panel.rs`), hover capsule
  (`--menubar-pill-radius: 999px`). **Motion is the exception**: not on the clock or the bar's battery percent, not on hover-in, not
  on panel open. Verified live: press holds 0.96, 0 animations at idle, anchors within 0.3 px after a click, open-from-closed stays instant.
- **Bar tuned to macOS proportions** (branch `feature/menubar-macos-proportions`, PR #27, `UI: full`): all metrics are CSS variables
  in `styles/tokens.css` (`--menubar-*`). `--menubar-height` (28px) is the source of truth: `useBarHeight` sends it to Rust
  (`setBarHeight`, saved in settings.json, default 28). Pill items (bar height - 4, radius 4, no transition); icon-only pills are 28px
  wide with a 2px gap (a 30px pitch like macOS). Custom SVG icons in `features/menubar/icons/`, clock "Thu Nov 12  1:18 PM" following
  the 12/24 h setting. **Battery is built in** (examples/battery was removed): its drawing is the example's, colours follow the bar tone,
  percentage left of the icon, data from the battery service. **Wi-Fi, battery and volume open panels** under their icon (routes `wifi`,
  `battery`, `volume`; windows mode opens Quick Settings). Volume panel = master slider, a slider per app (new service, ADR 0022:
  `getAppVolumes`, `setAppVolume`, `setAppMuted`, `appVolumesChanged`), output devices, Sound settings. `SystemSlider` has its own pointer
  handling (the head's edge sits under the cursor; verified with real clicks at 25/50/75 %). Night light now stamps like Windows' own button (first stamp = stored + 2, recorded and
  checked by Liam on screen: fast clicks all apply, ADR 0020). Debug window: Ctrl+Alt+Shift+D in debug builds.
  Left for Liam: search has no action;
  per-app volume is not measured for memory/CPU yet (`docs/perf/`).
- **Native feel**: Liam wants the product to feel native for public users. ADR 0021 is **proposed** (tier 1 remove webview tells,
  tier 3 native bar strip spike); plan in `docs/release/releases/native-feel.md` (local). Nothing built yet; Liam decides ADR 0021.
- **v0.4.0 shipped** (tag `v0.4.0` on the merge of PR #19, not pushed yet: Liam runs `pnpm release`). Plan releases
  v0.0 to v0.4 are done; notes in `docs/release/history/` (local only, v0.4 in `v0.4-checkpoint.md`).
- **v0.4b slice A done, PR #22 open** (branch `feature/v0.4b-system-panels-a`, Liam reviews and merges): panel window, clock
  settings, battery, volume (ADR 0016, `docs/perf/0.4b.md`). Next: slices B and C (`releases/v0.4b.md`, local only);
  re-measure panel latency once the UI uses `panelResized`.
- ADR 0012 (tray) accepted by Liam 2026-11-14; v0.6 may start (`releases/v0.6.md`, first task: the z-order prototype).
- **v0.4b slice C built, PR open** (branch `feature/v0.4b-notifications`, includes the spike of PR #23; ADR 0017 accepted, ADR 0018):
  notifications panel with the UI (`UI: full`): date in the bar + unread dot, notification card, shadcn calendar, Focus pills. The date sits inside the clock item (right side; anchor `system/clock`), 300 px wide cards; moving it (e.g. to the centre) is a v0.8 setting.
  Reader = `UserNotificationListener` + read-only `wpndatabase.db` (pictures) + folder watcher gated by a toast fingerprint
  (reading through the listener writes to that database: without the gate it looped at 3% CPU). Windows signals removals
  late, so opening the panel reads fresh. Switch `notificationsMode` (barista | windows) saved, no UI until v0.8 (rule 11).
  Barista focus timer is in the backlog.
- **v0.4b slice B built, PR open** (branch `feature/v0.4b-system-panels-b`, Rust only, ADR 0019): brightness (WMI + DDC/CI), Wi-Fi (WLAN API),
  Bluetooth + paired devices, airplane (read-only), hotspot, output devices + switching (undocumented `IPolicyConfig`, falls back to the sound
  page), user, actions, `openSettings`, and the `controlCenterMode` switch (rule 11). All verified live on this machine; `lock` ran (lock screen showed), `sleep` was not run, `restart` / `shutDown` / `signOut` only without `confirm` (refused). Memory is 130 MB vs 102 MB for v0.4.0 rebuilt today (budget 120): see `docs/perf/0.4b.md`. Left: UI wiring of these in the control center
  (UI is Liam's); airplane mode verified live by hand (Liam, 2026-11-12). New for the UI: see the list under "Facts".
- **Control center UI built** (branch `feature/v0.4b-control-center`, PR open, `UI: full`, ADR 0020): `features/panel/control-center/`, bar icon, real toggles
  (airplane, night light, battery saver, dark mode, Wi-Fi join with password), now playing (SMTC), stepped sliders. Not run live: airplane on (would cut the
  connection), restart / shut down. Battery saver switches through the WNF state Quick Settings uses (`RtlPublishWnfStateData`, no admin; tested live, ADR 0020). The `BaristaSaver` helper service was a dead end: run `pnpm helper:uninstall` if it is installed. Left for Liam: check on battery, the music tile with Spotify,
  a Wi-Fi join, other scales. Control center route is prebaked but not warmed (first open measures once).
- Panel is prebaked: `PanelWindow` keeps the frame and the notifications route mounted while hidden, `PanelBody` takes `name: null` when closed, `usePanelSize` takes a `null` key. No opening motion (Windows does not animate a bare transparent window, measured; in-page motion felt staggered): the window appears whole in one frame. Never resize the window around an open (WebView2 paints the top first): sizes are remembered per route, the warm-up (`panelWarm`) measures and paints it once at startup. To watch it: real clicks with `SetCursorPos` (logical px) + `mouse_event`, and `CopyFromScreen` frames (physical px). Notification list = one block per notification; the list is as tall as one 16:9 picture notification (measured) and has no minimum, so a lone notification shrinks the card. ADR 0018 has the details.
- Debug a panel without a click: run the bar with `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9222`, then from
  the `panel` page call `close_panel`, then `open_panel` and capture within ~1 s (the panel closes on blur because Windows won't
  focus it without a real click). Same-anchor `open_panel` toggles.
- CSS gotcha: a `<button>` and a scrolling (`overflow-y-auto`) box clip anything that sticks out of them, even with negative
  margins; the DOM rect still says it is fine. Notification pictures therefore sit in their own full-width wrapper
  (`overflow-hidden rounded-panel-child`) and the list has `-mx-panel-pad px-panel-pad`. Check by looking, not by `getBoundingClientRect`.
- New for the UI (slice C, wired already): see ADR 0018. `openPanel` has a 5th argument `chrome: "glass" | "none"`.

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
