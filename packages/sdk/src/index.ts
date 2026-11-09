/**
 * @barista/sdk: put your app in the Barista menu bar.
 *
 * Use it as a namespace:
 *
 * ```ts
 * import * as BA from "@barista/sdk"
 *
 * const item = await BA.StatusItem.create({ id: "counter", label: "0" })
 * item.on("click", () => console.log("clicked"))
 * ```
 *
 * Every name is also exported with a `BA` prefix (`BAStatusItem`) for people who prefer
 * named imports. Menus and popovers arrive in later releases.
 */

import { connect, StatusBar } from "./status-bar.js"
import { StatusItem } from "./status-item.js"

// Generated from crates/protocol by `pnpm gen`: message types, PROTOCOL_VERSION, limits.
export * from "./protocol.js"
export { SDK_NAME, SDK_VERSION } from "./version.js"

export { BarError, connect, StatusBar } from "./status-bar.js"
export type { ConnectOptions } from "./status-bar.js"
export { StatusItem } from "./status-item.js"
export type { ClickEvent, StatusItemOptions } from "./status-item.js"

// Prefixed names, same things: `import { BAStatusItem } from "@barista/sdk"`.
export const BAStatusBar = StatusBar
export type BAStatusBar = StatusBar
export const BAStatusItem = StatusItem
export type BAStatusItem = StatusItem
export const BAConnect = connect
