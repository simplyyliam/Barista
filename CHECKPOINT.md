# Checkpoint

Keep this file short: where we are, the next step, and facts a new session needs.
When a release ships, move its notes to `docs/release/history/` and start fresh.

## Where we are (2026-11-13)

- **Spec departures Liam confirmed (2026-11-13), do not "fix" them:** icon-only pills are 28px wide (3px padding, 2px gap, 30px pitch,
  not the 6-8px of the first spec); the battery is `examples/battery`'s drawing (no 27x12 construction or masks); the hover is a 999px
  capsule (not 4px).
- **Motion pass (ADR 0023, proposed)**: `motion/react`, values in `shared/motion.tsx`, research in `docs/research/ux-motion-and-taste.md`
  (local). Press 0.96, icon swaps (fade + scale + blur), items arrive/leave, panel-to-panel swap (`usePanelSwap`), hover capsule
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
- ADR 0012 (tray) still needs Liam's approval before v0.6 starts.
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

- From dogfooding the v0.4 popovers: the open papercuts (Electron window helper, `closePopover()`, async `on("open")`,
  stale `popover.shown` token, `setPopover` resize while open) are in `docs/release/history/v0.4-checkpoint.md`.
  Menu latency sits at 42-46 ms against the 50 ms budget (WebView2 drift, see `docs/perf/0.4.md`).

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
