/**
 * @barista/sdk: put your app in the Barista menu bar.
 *
 * Use it as a namespace:
 *
 * ```ts
 * import * as BA from "@barista/sdk"
 * // later releases: BA.StatusItem, BA.Menu, BA.Popover
 * ```
 *
 * v0.0 only ships the protocol types and version; the classes arrive from v0.2 on.
 */

// Generated from crates/protocol by `pnpm gen`: message types and PROTOCOL_VERSION.
export * from "./protocol.js"

/** This package's name, sent to the bar in the `hello` message. */
export const SDK_NAME = "@barista/sdk"

/** This package's version. A test keeps it equal to package.json. */
export const SDK_VERSION = "0.0.0"
