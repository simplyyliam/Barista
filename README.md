<div align="center">

<img src="apps/bar/src-tauri/icons/128x128.png" alt="Barista" width="96" height="96">

# Barista

**A menu bar for Windows.**<br>
Your tray icons, notifications and quick controls in one quiet bar at the top of every screen.

[Download](https://github.com/simplyyliam/Barista/releases/latest) · [Documentation](apps/web/content/docs/index.mdx) · [SDK on npm](https://www.npmjs.com/package/@baristabar/sdk)

![Windows 11](https://img.shields.io/badge/Windows-11-0078D4)
![License: MIT](https://img.shields.io/badge/license-MIT-green)
![Built with Tauri](https://img.shields.io/badge/built%20with-Tauri-24C8DB)

</div>

---

## Welcome

Windows gives you a taskbar. Barista adds the other half: a menu bar, like the one on a Mac.

It is free, open source and made to stay out of your way. It idles near 0% CPU, reserves its own space so
your windows sit below it, and does not collect anything about you.

## What you get

- **Tray icons in the bar.** Every app from the Windows tray lives in the bar. Click one and its own menu
  opens right under it.
- **Notifications.** Read, clear and open them from a panel under the clock.
- **Control center.** The switches you reach for all day, one click away.
- **Every monitor.** Pick the screen for the bar. Sizes follow each display's scale, so it stays sharp on
  mixed screens.
- **Space reserved, like macOS.** Windows are laid out below the bar, not under it.
- **Light on your PC.** The bar waits for events from Windows instead of polling.
- **An SDK for your own apps.** Add real menu bar items from Node, Electron or any language (see below).

## Install

1. Download the installer from the [latest release](https://github.com/simplyyliam/Barista/releases/latest).
2. Run it. It installs for your user only, so it does not ask for admin rights.
3. The bar appears at the top of your screen and your tray icons move into it.

Barista checks for updates on its own and asks before installing anything. You can switch that off in
Settings > General. Every Windows replacement (such as the notification center) can be switched back to the
Windows version in Settings.

> **Windows may say "Windows protected your PC" the first time.** The installer is not code-signed yet,
> because a certificate costs money. Choose **More info**, then **Run anyway**. The installer is built by
> GitHub Actions from this repository, so you can check where it came from.

**Requirements:** Windows 11. The installer sets up WebView2 if you do not have it.

## Put your own app in the bar

Any app can add a status item with a few lines. You need Barista running and [Node.js](https://nodejs.org) 22
or newer.

```sh
mkdir my-item && cd my-item
npm init -y
npm pkg set type=module
npm install @baristabar/sdk
```

Save this as `index.js`:

```js
import * as BA from "@baristabar/sdk"

await BA.connect({ appName: "Clock" })
const item = await BA.StatusItem.create({ id: "time", label: "12:00", tooltip: "The time" })

setInterval(() => item.setLabel(new Date().toLocaleTimeString()), 1000)
item.on("click", ({ button }) => console.log("clicked with", button))
```

Run `node index.js` and a clock appears in the bar. Stop the script and it disappears. Start the script
before Barista and the item shows up when the bar starts. Restart the bar and it comes back by itself.

**The bar owns the bar.** Your app never draws into it. You send a menu and Barista draws it, or you open
your own window and Barista tells you where to anchor it under the icon.

| You want | Read |
|---|---|
| A list of actions under your item | [Menus](apps/web/content/docs/menus.mdx) |
| Your own window under your item | [Popovers](apps/web/content/docs/popovers.mdx), [Electron](apps/web/content/docs/electron.mdx) |
| An item from a PowerShell, Python or Node file, with no SDK | [Script items](apps/web/content/docs/script-items.mdx) |
| The menus of the app in front (File, Edit, View) | [Application menus](apps/web/content/docs/app-menus.mdx) |
| An SDK for another language | [Protocol](apps/web/content/docs/protocol.mdx) |

More examples live in [`examples/`](examples) (a clock, a system monitor, now playing, a mic mute switch and more).

## Privacy

Barista sends nothing about you anywhere. No telemetry, no accounts. The only network call is the update
check (a small file from the releases page). Logs stay in a folder on your PC (Settings > General > Log files).

## Scripts are not sandboxed

A script item runs with your own permissions, like any program you start. Barista asks before a script runs
the first time and again when the file changes. Only add scripts you trust.

## Build it yourself

You need Windows 11, [Rust](https://rustup.rs), [Node.js](https://nodejs.org) 22.18 or newer and
[pnpm](https://pnpm.io).

```sh
git clone https://github.com/simplyyliam/Barista.git
cd Barista
pnpm install
pnpm dev
```

| Command | What it does |
|---|---|
| `pnpm dev` | Runs the bar in development, plus a few example apps that connect to it |
| `pnpm test` | Runs the JS and TS tests (`vitest`) |
| `cargo test --workspace` | Runs the Rust tests |
| `pnpm typecheck` / `pnpm lint` | Type checks and lints the JS and TS code |
| `pnpm gen` | Regenerates the bindings and the protocol types (never edit those by hand) |
| `pnpm test-client` | Starts a tiny app that registers items, handy for trying the bar |
| `cargo clippy --workspace --all-targets` | Lints the Rust code |

### How it is laid out

```text
apps/bar/          the Tauri app: React UI (src/) and the Rust that does the real work (src-tauri/)
apps/web/          website and SDK docs (Next.js)
crates/protocol/   the messages between the bar and apps; the source of the SDK's types
packages/sdk/      @baristabar/sdk, the public SDK
examples/          small apps and scripts that use the SDK
docs/              decisions (ADRs), design notes, performance results
```

Barista is a Rust app with a React face. Rust holds the logic (reserving screen space, the IPC server, the item
registry, the tray bridge) and the frontend only draws the state it is given. The reasons behind the big choices
are written down as short notes in [`docs/decisions/`](docs/decisions).

## Contributing

You are welcome here. Found a bug, have an idea or want to fix something?

1. Open an [issue](https://github.com/simplyyliam/Barista/issues) to say hi or describe the problem.
2. For code, fork the repo, branch from `main` and open a pull request. Small ones are easiest to review.
3. Please run `pnpm typecheck`, `pnpm lint`, `pnpm test` and `cargo test --workspace` first.

Public API names stay plain and conventional (`StatusItem`, `Menu`, `Popover`), and protocol changes must stay
backward compatible. If a change needs a new design choice, add a short note in `docs/decisions/`.

## Credits

Barista is built with the help of many libraries. Their names and licences are in [CREDITS.md](CREDITS.md).

## License

[MIT](LICENSE). Free to use, change and share.
