// Prints the bar's item registry, or fakes a click (debug builds of the bar only).
//   pnpm inspect                     list every item
//   pnpm inspect --watch             list again every 250 ms until Ctrl+C
//   pnpm inspect --click s1/counter  click an item (add "right" or "middle" for another button)
//   pnpm inspect --menu-click s1/timer sounds   pick an entry in an item's menu
//   pnpm inspect --system [--watch]  print clock, battery, volume, brightness, Wi-Fi, Bluetooth, airplane, hotspot, outputs, user (--watch: on change)
//   pnpm inspect --panel test s1/counter [width height]   open a system panel under an item
//   pnpm inspect --close-panel       close the panel
//
// It speaks the protocol by hand (about 20 lines) because `debug.*` messages are not part of
// the public SDK.
import net from "node:net"
import { PROTOCOL_VERSION, type BarMessage, type ClientMessage, type ItemInfo } from "@barista/sdk"

const PIPE = "\\\\.\\pipe\\barista"

function frame(message: ClientMessage): Buffer {
  const json = Buffer.from(JSON.stringify(message))
  const out = Buffer.alloc(4 + json.length)
  out.writeUInt32LE(json.length)
  json.copy(out, 4)
  return out
}

/** Opens a connection, says hello, and calls `onMessage` for every message after welcome. */
function open(onMessage: (m: BarMessage, send: (m: ClientMessage) => void) => void) {
  const socket = net.connect(PIPE)
  let buffer = Buffer.alloc(0)
  const send = (m: ClientMessage) => socket.write(frame(m))
  socket.on("connect", () =>
    send({
      type: "hello",
      protocolVersion: PROTOCOL_VERSION,
      appName: "inspect",
      pid: process.pid,
      sdk: { name: "inspect", version: "0" },
    }),
  )
  socket.on("data", (chunk) => {
    buffer = Buffer.concat([buffer, chunk])
    while (buffer.length >= 4 && buffer.length >= 4 + buffer.readUInt32LE(0)) {
      const length = buffer.readUInt32LE(0)
      const message = JSON.parse(buffer.subarray(4, 4 + length).toString()) as BarMessage
      buffer = buffer.subarray(4 + length)
      onMessage(message, send)
    }
  })
  socket.on("error", () => {
    console.error("Could not reach the bar. Is it running (pnpm dev)?")
    process.exit(1)
  })
  return socket
}

function print(items: ItemInfo[]): void {
  if (items.length === 0) return console.log("(no items)")
  for (const item of items) {
    const parts = [
      item.id.padEnd(22),
      item.appName.padEnd(14),
      `pid ${item.pid}`.padEnd(10),
      item.hasIcon ? "icon" : "    ",
      item.hasMenu ? "menu" : "    ",
      item.hasPopover ? "popover" : "       ",
      JSON.stringify(item.label ?? ""),
      item.visible ? "" : "(hidden)",
    ]
    console.log(parts.join("  "))
  }
}

const args = process.argv.slice(2)
const clickAt = args.indexOf("--click")
const menuClickAt = args.indexOf("--menu-click")
const watch = args.includes("--watch")
const panelAt = args.indexOf("--panel")

/** Sends one debug request, then reports success (no error within 200 ms) or the error. */
function request(message: ClientMessage, done: string) {
  let timer: NodeJS.Timeout | undefined
  const socket = open((reply, send) => {
    if (reply.type === "welcome") {
      send(message)
      timer = setTimeout(() => {
        console.log(done)
        socket.end()
      }, 200)
    } else if (reply.type === "error") {
      clearTimeout(timer)
      console.error(`${reply.code}: ${reply.message}`)
      process.exitCode = 1
      socket.end()
    }
  })
}

if (args.includes("--system")) {
  // Asks again every 250 ms in watch mode (this tool polls; the bar itself does not) and prints
  // only when something differs, so a plugged-in charger or a volume key shows up on its own line.
  let last = ""
  let timer: NodeJS.Timeout | undefined
  const socket = open((message, send) => {
    if (message.type === "welcome") {
      send({ type: "debug.system" })
      if (watch) timer = setInterval(() => send({ type: "debug.system" }), 250)
    } else if (message.type === "debug.system") {
      if (message.snapshot !== last) {
        last = message.snapshot
        const s = JSON.parse(message.snapshot) as Record<string, unknown>
        console.log(new Date().toLocaleTimeString(), "---")
        for (const [name, value] of Object.entries(s)) console.log(`${name.padEnd(8)}${JSON.stringify(value)}`)
      }
      if (!watch) socket.end()
    } else if (message.type === "error") {
      console.error(`${message.code}: ${message.message}`)
      process.exitCode = 1
      clearInterval(timer)
      socket.end()
    }
  })
} else if (args.includes("--close-panel")) {
  request({ type: "debug.closePanel" }, "panel closed")
} else if (panelAt >= 0) {
  const [name, anchor, w, h] = args.slice(panelAt + 1)
  if (!name || !anchor) {
    console.error("usage: pnpm inspect --panel <name, like battery> <item id, like s1/counter> [width height]")
    process.exit(1)
  }
  request(
    { type: "debug.openPanel", name, anchor, width: Number(w ?? 320), height: Number(h ?? 240) },
    `panel ${name} opened under ${anchor}`,
  )
} else if (menuClickAt >= 0) {
  const id = args[menuClickAt + 1]
  const menuItemId = args[menuClickAt + 2]
  if (!id || !menuItemId) {
    console.error("usage: pnpm inspect --menu-click <item id, like s1/timer> <menu entry id>")
    process.exit(1)
  }
  request({ type: "debug.menuClick", id, menuItemId }, `picked ${menuItemId} in ${id}`)
} else if (clickAt >= 0) {
  const id = args[clickAt + 1]
  const button = (args[clickAt + 2] ?? "left") as "left" | "right" | "middle"
  if (!id) {
    console.error("usage: pnpm inspect --click <item id, like s1/counter> [left|right|middle]")
    process.exit(1)
  }
  request({ type: "debug.click", id, button }, `click (${button}) sent to ${id}`)
} else {
  let timer: NodeJS.Timeout | undefined
  const socket = open((message, send) => {
    if (message.type === "welcome") {
      send({ type: "debug.listItems" })
      if (watch) timer = setInterval(() => send({ type: "debug.listItems" }), 250)
    } else if (message.type === "debug.items") {
      if (watch) console.clear()
      print(message.items)
      if (!watch) socket.end()
    } else if (message.type === "error") {
      console.error(`${message.code}: ${message.message}`)
      process.exitCode = 1
      clearInterval(timer)
      socket.end()
    }
  })
}
