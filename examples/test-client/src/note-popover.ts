// // The window behind the test client's second icon.
// //
// // Under plain Node there is no window to show, so a fake PopoverWindow only logs where the bar
// // asked for it (used by automated checks). Under Electron (`pnpm test-client:electron`) it is a
// // real small window with a text box, so you can see it appear under the icon with keyboard focus.
// import * as BA from "@baristabar/sdk"
// import { electronPopover } from "@baristabar/sdk/electron"

// const PAGE = `<!doctype html><meta charset="utf-8"><title>Note</title>
// <style>
//   html,body{margin:0;height:100%;font:14px system-ui;color:#eee;background:#1f2328}
//   body{display:flex;flex-direction:column;gap:8px;padding:12px;box-sizing:border-box}
//   h1{font-size:13px;margin:0;opacity:.7;font-weight:600}
//   textarea{flex:1;resize:none;border:1px solid #444;border-radius:8px;background:#2b3138;color:inherit;padding:8px;font:inherit}
//   textarea:focus{outline:2px solid #4c8dff}
// </style>
// <h1>Test client popover</h1>
// <textarea autofocus placeholder="Type here: the window has keyboard focus. Esc closes it."></textarea>`

// /** A fake window that logs what the bar asks for. */
// function fakeWindow(): BA.PopoverWindow {
//   return {
//     show: (p) =>
//       console.log(
//         `popover.show at ${p.x},${p.y} ${p.width}x${p.height} (scale ${p.scale}, ${p.tone}, monitor ${p.monitor.slice(0, 24)})`,
//       ),
//     hide: () => console.log("popover.hide"),
//     onClose: () => {},
//   }
// }

// async function buildElectronWindow(): Promise<BA.PopoverWindow> {
//   const { app, BrowserWindow, screen } = await import("electron")
//   await app.whenReady()
//   const win = new BrowserWindow({
//     show: false,
//     frame: false,
//     resizable: false,
//     skipTaskbar: true,
//     alwaysOnTop: true,
//     backgroundColor: "#1f2328",
//     webPreferences: { sandbox: true, contextIsolation: true },
//   })
//   await win.loadURL(`data:text/html;charset=utf-8,${encodeURIComponent(PAGE)}`)
//   return electronPopover(win, { screen })
// }

// export function createNotePopover(): BA.PopoverWindow {
//   if (!process.versions.electron) return fakeWindow()
//   // Electron only fires "ready" after the main file finished loading, so waiting for it with a
//   // top-level `await` here would deadlock. The window is built in the background and `show`
//   // waits for it (it is long done by the first click).
//   const closeListeners: Array<() => void> = []
//   const ready = buildElectronWindow().then((window) => {
//     window.onClose(() => closeListeners.forEach((listener) => listener()))
//     return window
//   })
//   ready.catch((error: unknown) => console.error("could not create the popover window:", error))
//   return {
//     show: async (placement) => (await ready).show(placement),
//     hide: () => void ready.then((window) => window.hide()),
//     onClose: (listener) => void closeListeners.push(listener),
//   }
// }
