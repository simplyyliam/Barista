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
  (real mouse clicks on the bar), bench in `docs/perf/0.3.md`. PR "v0.3 Menus" open; waiting for Liam's review, the menu
  UI, and approval of ADR 0012 (tray) before v0.6.
- v0.4b System panels is planned (`releases/v0.4b.md`, local only).

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
