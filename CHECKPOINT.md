# Checkpoint

Keep this file short: where we are, the next step, and facts a new session needs.
When a release ships, move its notes to `docs/release/history/` and start fresh.

## Where we are (2026-11-09)

- **v0.2.0 shipped** (tag `v0.2.0` on the merge of PR #8, not pushed yet: Liam runs `pnpm release`). Plan releases
  v0.0 Setup, v0.1 Bar shell and v0.2 First item are done; their full notes are in
  `docs/release/history/v0.0-v0.2-checkpoint.md` (local only).
- **In progress: v0.3 Menus** on `feature/v0.3-menus` (plan: `~/.claude/plans/iterative-weaving-reddy.md`).
  Built: protocol 0.3, menu validation, registry menus + checked state + roles, placement math, reused menu window
  (`menu_window.rs`), events/commands, SDK `BA.Menu`, focus-timer test client, `inspect --menu-click`, tray spike
  (`docs/research/tray-bridging.md`, local) + ADRs 0010-0012. Code review + security review done and fixed, e2e passed
  (real mouse clicks on the bar; keyboard-only through a throwaway handler injected over WebView2 remote debugging,
  real keystrokes; edge placement and submenu flip on the real 150 % screen), bench in `docs/perf/0.3.md`. PR "v0.3 Menus" open; waiting for Liam's review, the menu
  UI, and approval of ADR 0012 (tray) before v0.6.
- v0.4b System panels is planned (`releases/v0.4b.md`, local only).

## Menu polish (2026-11-11, branch `feature/0.3/menubar-ui`, UI: full)

- Two reused windows: `menu` (main panel) and `menu-sub` (submenu, `WS_EX_NOACTIVATE`), each with DWM acrylic,
  round corners, native shadow (ADR 0013). Windows hides them at once with its own fade (a CSS fade left a ghost panel).
- UI: `widgets/menu/{menu,root-menu,sub-menu}.tsx`, pieces in `features/menu/` (list, panel, hooks, `lib/navigation.ts`
  with tests). Tokens: `--menu-*` in `styles/tokens.css`. `motion` is installed but unused by the menu.
- Submenu intent: angle test + speed (`docs/research/submenu-intent.md`, local). Research also in
  `docs/research/menu-acrylic.md` and `progressive-blur.md` (local, gitignored).
- New commands: `menuHover`, `openSubmenu`, `closeSubmenu`, `submenuResized`, `submenuHover`, `setSubmenuHighlight`;
  `menuResized` is now `(width, height)`. New events: `submenuShow/Hide/Placed/Highlight/Hovered`. Capability lists `menu-sub`.
- Checked live (real bar + test client + `examples/integrations` GitHub/Spotify items, driven over WebView2 debugging):
  keyboard (Up/Down/Home/End/Enter/Esc/Left/Right, type-to-jump), hover-switch, same-icon toggle, submenu flip, safe
  triangle (fast vs slow moves), kill-client closes its menu, no blank submenu shell (stress), edge margin. `pnpm bench`:
  menu visible 17 ms, idle CPU 0.02%, 70 MB private. `/code-review medium` findings fixed; `/security-review`: none.
- Bugs found live: submenu window shown with raw Win32 must be hidden raw too (tao ignores `hide()`); a late show after
  `close_submenu` left an empty window; submenu acrylic was flat (inactive window) -> `WM_NCACTIVATE` fixes it.
- Not watched by eye: the native Windows fade-out on close (needs a person).
- Menus keep an 8 px margin from the left/right/bottom screen edges (`inset_work`). Progressive bar blur is research only.

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

## Fix next

- From dogfooding the focus timer (v0.3 menus):
  - Changing one entry (Start enabled, "Resume" label) means rebuilding the whole menu and calling `setMenu` again.
    Want `item.updateMenuItem(id, { enabled, label, checked })` or setters on `MenuItem` that sync by themselves.
  - `click(entry, { checked })`: `checked` is optional even for a checkbox, so apps write `checked ?? !x`.
    `entry.checked` is already mirrored; document that and maybe drop `checked` from the event.
  - No way to refresh a menu right before it opens (`item.on("open")`), e.g. "12:34 left" in a label (noted in v0.4).
  - `inspect` keys change every connection (`s22/timer`, `s48/timer`); accept `appName/itemId` too.
  - Accelerators are text only; "CmdOrCtrl+Enter" next to Start suggests a hotkey that doesn't exist. Say so in docs.
  - Fixed while dogfooding: role entries default their id to the role (`quit`), separators don't use up ids.

- Silent client (sends `hello`, then nothing) holds a connection slot until it disconnects: fixed by the v0.5 heartbeat.
- SDK papercuts from dogfooding `examples/clock`:
  - Accept raw SVG markup (`<svg...`) as `icon`, maybe a file-path helper.
  - No bar running = `connect()` rejects; needs wait/retry (v0.5), README should show try/catch until then.
  - `create()` resolves on send, not on accept (`limit_reached` arrives later as `error`); consider `await item.ready`.
  - README: say a label over 128 chars throws `TypeError`; mention `appName` in the quick start.
  - `item.on("click")` should also give the item's position (popovers, v0.4).
- Not tested on real hardware: no monitor at launch (starts on a placeholder monitor).

## Useful facts

- Write scripts with the Write tool; Bash heredocs with apostrophes break here.
  Python on Windows: open files with `encoding='utf-8', newline=''`.
