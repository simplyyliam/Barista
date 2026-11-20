# Barista

A menu bar for Windows, like the one on a Mac. Tray icons live in it, the bar reserves its own space at
the top of the screen, and any app can add its own items with a small SDK.

**Download:** the installer is on the [releases page](https://github.com/simplyyliam/barista-releases/releases/latest).
It installs for your user only (no admin), and it updates itself.

> Windows may show "Windows protected your PC" the first time, because the installer is not signed with a
> paid certificate yet. Choose **More info**, then **Run anyway**.

## Add your own item in 5 minutes

You need Barista running and [Node.js](https://nodejs.org) 22 or newer.

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

Run `node index.js`. A clock appears in the bar. Stop the script and it disappears. Start the script before
Barista and the item shows up when the bar starts; restart the bar and it comes back by itself.

From here: menus, popovers, Electron apps, script items (a PowerShell, Python or Node file in a folder) and
the protocol for writing an SDK in another language are all in the docs. Every public function has a short
example in its TSDoc.

## Privacy

Barista sends nothing about you anywhere. No telemetry, no accounts. The only network call is the update check
(a small file from the releases page). Logs stay in a folder on your PC (Settings > General > Log files).

## Scripts are not sandboxed

A script item runs with your own permissions, like any program you start. Barista asks before a script
runs the first time and again when the file changes. Only add scripts you trust.

## Credits

Barista is built with the help of many libraries. Their names and licences are in
[CREDITS.md](CREDITS.md).

## License

[MIT](LICENSE). Free to use, change and share.
