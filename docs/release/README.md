# Release plan

The source of truth for what gets built and when. Split into small files so a session only
reads what it needs.

## How to use it

1. Open `CHECKPOINT.md` to see which release is current.
2. Read that release's file in `releases/` (for example `releases/v0.2.md`).
3. Read only the plan sections and research files that release file names.
4. Ideas for later go into the later release's file as notes, not code.

## Who builds what

Every release file has two lists:

- **Claude (Rust + SDK):** native code, protocol, SDK, tests, test client.
- **Liam (UI):** the bar's React frontend in `apps/bar/src/`, designed in Figma first
  (`docs/design/`).

Each release file ends with a **/goal prompt** for handing the whole release to Claude. Its
`UI:` line is `rust-only` by default; change it to `full` on a day Claude should build the UI too.

A release is done when both lists and its **Done when** checks are finished, then it ships
through a `release/v0.X.Y` PR (see `CLAUDE.md` > Releases).

## Plan sections (`plan/`)

| File | What's in it |
|---|---|
| `01-what-it-is.md` | The product in one page: problem, who it's for, what it is and isn't |
| `02-architecture.md` | Processes, layers, data flow, how the bar and clients talk |
| `03-protocol.md` | Every message between bar and client, with rules |
| `04-feature-catalog.md` | Every feature, tagged with the release it lands in |
| `05-performance-budget.md` | CPU, memory and latency targets, how they're measured |
| `06-roadmap.md` | All releases at a glance |
| `07-risks.md` | What could go wrong and what we do about it |

## Releases (`releases/`)

| Release | Name | Showcase (what Liam can do at the end) |
|---|---|---|
| v0.0 | Setup (done) | `pnpm dev` opens an empty bar window; CI passes (no tag) |
| v0.1 | Bar shell (done, in 0.2.0) | The bar sits at the top, maximized windows stop under it |
| v0.2 | First item (done, 0.2.0) | The test client puts an icon in the bar |
| v0.3 | Menus | Click the test client's icon, a menu drops down |
| v0.4 | Popovers | Click an icon, the app's own window appears under it |
| v0.4b | System panels | The bar shows the clock, battery and volume from real data; a panel (control center, notifications, calendar) opens under an item. Built-in Rust services, a reusable panel window (`releases/v0.4b.md`) |
| v0.5 | Never stuck | Kill a client, its icon goes; restart the bar, icons come back. SDK published |
| v0.6 | Tray, read | Every tray icon shows in the bar |
| v0.7 | Tray, live | Tray icons work: click, right-click menus, pin, collapse |
| v0.8 | Daily driver | Settings, first-party items (mic mute, now playing, system monitor), Liam uses it all day instead of the taskbar tray |
| v0.9 | Script items | Drop a PowerShell, Python or Node script in a folder and it shows up in the bar (a system monitor script is the demo) |
| v0.10 | Ship it | Installer, auto-updater, docs, first public release |
| v1.0 | 1.0 | Stable protocol v1, stable SDK |
| v1.1 | Gallery | Curated scripts (repo `barista-scripts`) on the Barista site, Install button with a confirm dialog (notes only, `releases/v1.1.md`) |
| v1.2+ | After the gallery | Dock, React package, Tauri plugin, C# SDK (`releases/v1.2.md`) |

## History

Finished releases move their checkpoint notes to `history/`.
