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
- v0.2 First item (2026-11-09, branch `feature/v0.2-first-item`, PR open): built, rust-only. ADRs 0005 (pipe server), 0006 (items), 0007 (SDK shape).
  - Code: `crates/protocol` (protocol 0.2, limits exported into `protocol.ts`), `ipc/{frame,validate,session,mod}.rs`,
    `registry/mod.rs`, `platform/windows/pipe.rs` (user-only DACL, first-instance, client pid), `events::forward_changes`
    (one `items-changed` per 16 ms at most). SDK: `status-bar.ts`, `status-item.ts`, `frame.ts`, `validate.ts`.
  - Examples: `pnpm test-client` (counting item), `pnpm inspect [--watch | --click <key> [button]]`,
    `examples/clock` (dogfood). `pnpm bench 0.2` -> `docs/perf/0.2.md` (needs a release build; sets `BARISTA_DEBUG=1`).
  - `debug.listItems` / `debug.click` work in debug builds, or in release with `BARISTA_DEBUG` set. Otherwise `unknown_type`.
  - Checked live (debug and release bar): item shows, label counts, `inspect --click` reaches the client, bad messages
    (huge icon, bad id, garbage, unknown type, 2 MB frame) are rejected and the bar plus another client are unaffected,
    killing the client removes its item. Not testable without the UI: that items draw and clicks go through `itemClicked`.
  - New for the UI: `commands.getItems()` -> `StatusItemView[]` (`id` is the key `"s3/counter"`, `appName`, `icon` as a
    `data:` URL, `label`, `tooltip`), `commands.itemClicked(id, button)` (`"left" | "right" | "middle"`),
    `events.itemsChanged` (full list of visible items, replaces what the UI holds). Show icons with `<img src>` only, never inline the SVG.
- **Next:** Liam reviews the v0.1 and v0.2 PRs and builds the bar layout and item rendering in `apps/bar/src/`. Then v0.3 (menus).

## Fix next

- Code review v0.2 (medium): everything fixed except one on purpose: a client that sends `hello` and then goes silent,
  or stalls in the middle of a frame, holds its task and one of the 64 connection slots until it disconnects. The fix
  is the v0.5 heartbeat (ping/pong with a timeout); noted in `releases/v0.5.md`. Security review: nothing to report.
- Dogfooding v0.2 (`examples/clock`, written from the SDK README only):
  - A plain SVG string as `icon` throws ("must be a base64 data: URL"); it is the first thing anyone tries. Accept raw SVG
    markup (a string starting with `<svg`) and probably a file path helper too.
  - No bar running = `connect()` rejects and an app that does not catch it dies with a stack trace. Needs wait/retry (v0.5);
    until then the README should show a try/catch.
  - `create()` resolves when the message is sent, not when the bar accepted it, so `limit_reached` shows up later as an
    `error` event. A real "accepted" answer (or `await item.ready`) would be easier to explain.
  - README says "label up to 128 characters" but not what happens beyond: it throws a `TypeError` at the call. Say so.
  - `appName` defaults to the script name; the README quick start does not mention it besides the `connect` option.
  - `item.on("click")` only gives `button`; apps will want the item's position for anchoring (popovers, v0.4).

- Memory (decided by Liam 2026-11-08): the budget counts private memory, 62 MB vs 120 MB. The plain working-set
  sum (~330 MB) is reference only. `--disable-gpu --renderer-process-limit=1` in the window's `additionalBrowserArgs`
  got it there, so bar animations render in software: tell me if they feel slower.
- `pnpm bench` refuses to run while a bar is already running (a second bar shares its WebView2 process and
  the numbers come out wrong): close the bar and `pnpm dev` first.
- Verified by Liam: 100% and 125% scaling, and unplugging the external monitor (bar fell back to the
  laptop screen, then went back when it was plugged in again). 150% checked by Claude (4K monitor).
  Explorer restart checked by Claude on Liam's running bar: space dropped to 0 and came back at 48 px.
- Dogfooding by Liam (hands-on, 2026-11-08): ran the bar at 100% and 125% scaling and unplugged and replugged the
  external monitor; nothing broke and no awkward behaviour was reported. Nothing else added to "Fix next" from it.
- Dogfood (scripted, 4 min with Edge open: bar CPU 0.009%; Edge): maximized Edge stops under the bar, F11 hides it and leaving F11 shows it, a simulated
  `WM_DISPLAYCHANGE` leaves the bar unchanged.
- Code review is fully addressed. The last item (no monitor at launch) now starts on a placeholder monitor and
  switches to the real one on the next display change; not testable here with real hardware.
- The first-run window is 800x32 until the first `apply`; fine because it starts hidden.

## Useful facts (keep only what still matters)

- Write scripts with the Write tool; Bash heredocs with apostrophes break here.
  Python on Windows: open files with `encoding='utf-8', newline=''`.
