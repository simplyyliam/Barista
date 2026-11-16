# Checkpoint

Keep this file short: where we are, the next step, and facts a new session needs.
When a release ships, move its notes to `docs/release/history/` and start fresh.

## Where we are (2026-11-16)

- **v0.7 "Tray, live" built, PR open** (branch `feature/v0.7-tray-live`, `UI: rust-only` in the /goal, then Liam pointed at the `CLAUDE (UI)` heading and the UI was built too; Liam reviews and merges; ADRs 0027 clicks, 0028 layout).
  Tray clicks: `tray/click.rs` (message runs per button and version, wParam/lParam encoding, DPI conversion of the anchor, double-click
  tracker) + `platform/windows/tray_input.rs`; `route_click` sends `tray/...` ids to the owner after `AllowSetForegroundWindow`. Layout:
  `registry/layout.rs` (pure), settings `itemLayout { order, pinned, collapsed }`, stable ids (`app:<app>/<item>`, `tray:<guid>` or
  `tray:<exe>/<id>`). **UI (wired, real mouse tested):** `features/panel/tray-panel.tsx` (tile clicks, pin, drag to reorder, collapse button), `features/menubar/tray-items.tsx` (pinned tray pills in the bar, same clicks), `use-item-space.ts` (`setItemSpace`), `shared/lib/item-click.ts`, `shared/hooks/use-collapsed.ts`. Uses `StatusItemView.pinned` and `.placement` (`bar` | `overflow`, list in display order),
  commands `setPinned`, `setOrder`, `setCollapsed`, `getCollapsed`, `setItemSpace(width, gap)`, event `collapsedChanged`. Defaults: collapsed,
  client items pinned, tray icons not (Liam's choice). Tools: `scripts/tray-probe.ps1` (fake app that logs the callback messages, optional
  menu), `scripts/tray-click.ps1` (real click on the bar, then `inspect --click`, reports windows/menus that opened), `inspect --click <id> double`.
  Bench in `docs/perf/0.7.md`. Verified live with the probe (sequences, anchor, menu opens at the pointer) and over CDP (pin, order, restart keeps them).
  **Reviews done:** `/code-review medium` (9 findings: fixed width updates from overflow, snapshot cost of `note_widths`, pins dropped at the cap, save race, failed click leaving a double pending, reorder refused for a vanished id; the rest are in Fix next), `/security-review` (no findings), end-to-end (test client item and menu, gone after kill, pin / order / restart). **Checked against Explorer (2026-11-16):** Liam recorded Explorer's clicks on the probe's two icons; the bar now sends the same runs (fixed: a
  version 4 double click ends with `NIN_SELECT`; the anchor is the physical pointer position, not DPI-converted). Differences kept on purpose: one
  `WM_MOUSEMOVE` before a click, no hover messages (ADR 0027).

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

- Through the UI (2026-11-16, end-to-end with the 10 tray apps on this machine, bar expanded so each icon is a pill): the clicks were fired as DOM events on
  the pills (the UI's own `onClick` / `onContextMenu` -> `itemClicked`), because scripted pointer input (`SetCursorPos`, error 203) stopped working halfway
  through the session; the real-mouse checks below were done before that. Result, left / right / double: Medal window / window / window, AMD none / menu / window,
  SteelSeries GG none / menu / window, Twinkle Tray none / window / none, ISLC none / window / none, Windhawk none / menu / none, Explorer's Safely Remove
  menus, Bluetooth menu / none / Settings window. **No reaction** for Raycast and Sonar (Raycast reacted in the first sweep). Restart with the UI: saved order and
  pins -> Rust -> the bar draws the two pinned icons and the panel shows the same order with the pins marked.
  - Startup: on one start no tray app re-added its icon after the bar's `TaskbarCreated` broadcast (0 icons; the host worked: a probe appeared; a manual
    broadcast brought 12 back; the next start was fine). **Fixed:** if the tray is still empty 4 s after the start the host broadcasts again, and once more
    at 12 s (`tray::retry_announce`, `tray_host::announce`; two sleeps, not a poll). The failure could not be reproduced, so the retry path was not seen
    firing live: only that a normal start is unchanged (10 icons, no retry logged).
  - The panel: once `getOpenPanel` still said `tray` while the window was hidden and the arrow's next click toggled it shut. `panel.rs` closes on blur after
    a short grace and clears the state then, so this was most likely that grace window; not reproduced, nothing changed.
- From wiring the v0.7 UI (real mouse, dev bar): pin (hover shows the pin), right-click on a pinned icon in the bar opens the app's own menu under the
  icon (AMD: "Open AMD Software / Video Profile / Hide System Tray"), drag a tile to the first place and it stayed after the event, "Show all in the bar"
  moved all 16 icons into the bar, a tile click opened Medal and closed the panel. Not done / to look at:
  - Pinned pills in the bar cannot be dragged (reorder is in the panel only); no way to unpin from the bar itself (open the panel).
  - The pin only shows on hover when unpinned: not obvious. Needs Liam's eye (no design exists).
  - With the bar expanded, 16 icons fill the right side; `setItemSpace` sends the room but it was not tested on a narrow bar.
  - In one screenshot an editor window sat in front of the open panel (the panel is not always on top?): not investigated, may be my test setup.
  - The panel route has no keyboard path for pin or reorder.
- From v0.7 (tray, live), found by testing (a real afternoon with the UI is Liam's, the UI is not wired yet):
  - Clicks were checked on 11 real tray icons through `scripts/tray-click.ps1`: most open their window or menu under the pointer
    (Medal, Twinkle Tray, G HUB, SteelSeries GG, AMD, ISLC, Windhawk, Explorer's own icons), but results vary run to run and some icons
    show nothing (Raycast on a second run, SteelSeries left click): not compared with Explorer. Debug clicks are not real input, so a
    menu sometimes stays open until Escape / `WM_CANCELMODE`; the real check is once the UI calls `itemClicked`.
  - Two more rounds of the sweep (fresh bar, left / right / double each, `tray-click.ps1`): the same in both rounds: AMD Software (right = menu, double = window),
    Medal (left and double = window), ISLC (right = its menu), SteelSeries GG (right = menu, double = window), Windhawk (right = menu), Explorer's own icons
    (menus). Nothing visible in either round: Twinkle Tray and Raycast (both reacted in the earlier sweep: the detector or the apps' toggle state), and left
    on most apps (many only react to a double click or toggle an existing window). The messages themselves now match Explorer's (above), so what an app does
    with them is the app's.
  - Stable ids: some programs use a changing number as icon id (G HUB's was `532175291`), so their saved place is lost at the next start.
    Needs a friendlier id (executable path + icon id?).
  - Explorer's own icons (Safely Remove, Bluetooth, Sonar) are listed and clickable like any other: still undecided which to hide (v0.6 note).
  - Stale item bounds: an item that moved to overflow keeps its old entry in `placement::bounds()`, so a click from the overflow panel may anchor
    at the old bar position (only matters for apps that use the version 4 anchor). Drop bounds when the placement changes.
  - The width guesses (28 px icon, 14 + 7 per character label) live in `registry/mod.rs`; the UI could send the real ones once.
  - `tray_set` compares the owner fields too, so a callback change wakes the UI with an identical list (rare).
  - Bench: worst tooltip latency 1804 ms in one run (median 19 ms), disturbed by the reviews running; re-run on a quiet machine.
  - Once the dev bar died at startup with `0xc0000409` right after "appbar registration failed, will retry" (the known `ExplorerHooks.dll`
    crash, see the v0.6 notes); the next four starts were fine. Not reproduced.

- From v0.6 (tray, read), found by testing and by Liam's feedback:
  - Soak (1 h, unattended, `scripts/soak.ps1`): bar alive throughout, CPU 0.004 to 0.28 %, memory 156 / 180 / 90 MB (no growth), Explorer never restarted; says nothing about daily feel, which is Liam's to judge.
  - Not tested: Windows 10 (only Windows 11 here), no tray app clicks (v0.7).
  - **`ExplorerHooks.dll` crash:** Windows named it as the faulting module of two `0xc0000409` aborts of `barista.exe` (once after a broadcast of
    `TaskbarCreated`, once when Liam restarted Explorer). It belongs to the Store app "Transparent Taskbar for Windows" (2845PeroSoftware), which
    injects it into other processes. Not confirmed whether our hidden `Shell_TrayWnd` makes it worse: the A/B (v0.5 bar, same broadcast) was
    not run, and cannot be from here: the DLL is only in our process while that app runs, and it was not running later (checked: only
    `windhawk.dll` is loaded in the bar). To settle it: start Transparent Taskbar, start the bar, then broadcast `TaskbarCreated` or restart
    Explorer, once with `trayMode: windows` (no host) and once without. If it is ours: back off when that DLL is loaded in our process
    (like `foreign_host`), or document the conflict. Checked 2026-11-14: with 14 real + 15 fake icons the default panel leaves the bar
    exactly where it was (before/after capture), so the narrow-bar overflow is gone with the default style.
  - Hover tint stuck after leaving the bar (Liam saw it on the bench's release bar). Could not reproduce with `SetCursorPos`, `mouse_event`,
    panels opened and closed by Esc or a click outside; the page showed no hover and no tint when it happened to look lit. Needs a repro with
    `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9222` and the DOM read at that moment (`data-hover`, `:hover`, `:focus-visible`).
  - Windows' own icons (volume, network, Bluetooth, "Safely remove") arrive as tray icons next to the bar's own: probably hide the ones the bar draws.
  - Full-colour icons next to monochrome ones look noisy (Liam): desaturate to the bar tone with colour on hover? Not decided.
  - Owner names are exe names (`lghub_system_tray`, `WhatsApp.Root`, `IDMan`); a friendlier name from the file's product name.
  - The inline style can push other items off a narrow bar (the reason `panel` is the default); cap it or drop it.
  - Messages sent while the Start menu is open reach Explorer only (ADR 0026, known gap).
  - `trayMode` and `tray::init` cannot be switched while running (`SINK` is set once): plan in `releases/v0.8.md`.
  - `items-changed` carries every icon's data on each change (about 27 KB; 0.6 events/s with a real tray): diff it if it ever shows in a profile.

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

- SDK papercuts from dogfooding `examples/clock`:
  - Accept raw SVG markup (`<svg...`) as `icon`, maybe a file-path helper.
  - `create()` resolves on send, not on accept (`limit_reached` arrives later as `error`); consider `await item.ready`.
  - README: say a label over 128 chars throws `TypeError`; mention `appName` in the quick start.
  - Done in v0.4: `item.on("click")` gives `anchor`; `item.on("open" | "close")` exists (menus and popovers).
- Not tested on real hardware: no monitor at launch (starts on a placeholder monitor).

## Useful facts

- Write scripts with the Write tool; Bash heredocs with apostrophes break here.
  Python on Windows: open files with `encoding='utf-8', newline=''`.
