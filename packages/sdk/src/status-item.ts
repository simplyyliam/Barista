import type { MouseButton } from "./protocol.js"
import { BarError, connect, type StatusBar } from "./status-bar.js"
import { assertItemId, assertLabel, assertTooltip, toIconDataUrl } from "./validate.js"

/** What you get when the user clicks your item. */
export interface ClickEvent {
  button: MouseButton
}

/** Options for {@link StatusItem.create}. */
export interface StatusItemOptions {
  /**
   * Your own name for the item, unique within your app. 1 to 64 characters from
   * `a-z 0-9 - _ .`
   */
  id: string
  /** PNG or SVG as bytes (`fs.readFileSync("icon.svg")`) or a `data:` URL. Max 256 KB. */
  icon?: Uint8Array | string
  /** Short text next to the icon. Max 128 characters. */
  label?: string
  /** Text shown when the pointer rests on the item. Max 512 characters. */
  tooltip?: string
  /** The connection to use. Leave out to use the shared one from {@link connect}. */
  bar?: StatusBar
}

type Listener<T> = (event: T) => void

/**
 * Runs a listener of the app. If it throws, the error is raised afterwards (so the app still
 * crashes loudly like with any uncaught error), but the other listeners still run.
 */
function callSafely(call: () => void): void {
  try {
    call()
  } catch (error) {
    queueMicrotask(() => {
      throw error
    })
  }
}

/**
 * One item in the Barista bar: an icon, a label, or both. The bar draws it; you only say what
 * it shows and react to clicks.
 *
 * Setters send the change right away and return at once; if the bar refuses something it
 * tells you through `on("error")`. Up to 8 items per app.
 *
 * @example
 * ```ts
 * import * as BA from "@barista/sdk"
 *
 * const item = await BA.StatusItem.create({ id: "counter", label: "0" })
 * item.on("click", ({ button }) => console.log("clicked with", button))
 * let n = 0
 * setInterval(() => item.setLabel(String(++n)), 1000)
 * ```
 */
export class StatusItem {
  /** The id you passed to `create`. */
  readonly id: string

  readonly #bar: StatusBar
  readonly #clickListeners = new Set<Listener<ClickEvent>>()
  readonly #errorListeners = new Set<Listener<BarError>>()
  #removed = false

  /** @internal The request id of the `item.create`, to recognise a refused create. */
  readonly createRequestId: string

  private constructor(id: string, bar: StatusBar, createRequestId: string) {
    this.id = id
    this.#bar = bar
    this.createRequestId = createRequestId
  }

  /**
   * Puts a new item in the bar. Connects to the bar first if you have not yet. Rejects if
   * the bar is not running, or (without sending anything) if an argument breaks a rule
   * such as a bad id or an icon over 256 KB.
   *
   * It resolves once the request is sent, not when the bar accepted it. If the bar refuses
   * the create (for example `"limit_reached"`), the error goes to `item.on("error")` (or the
   * connection's `error` listeners) and the item is forgotten, so its id can be used again.
   *
   * @example
   * ```ts
   * const item = await BA.StatusItem.create({
   *   id: "mail",
   *   icon: fs.readFileSync("mail.svg"),
   *   label: "3",
   *   tooltip: "3 unread messages",
   * })
   * ```
   */
  static async create(options: StatusItemOptions): Promise<StatusItem> {
    assertItemId(options.id)
    if (options.label !== undefined) assertLabel(options.label)
    if (options.tooltip !== undefined) assertTooltip(options.tooltip)
    const icon = options.icon === undefined ? undefined : toIconDataUrl(options.icon)
    const bar = options.bar ?? (await connect())
    if (bar.items.has(options.id)) {
      throw new Error(`This app already has an item with the id ${JSON.stringify(options.id)}`)
    }
    const requestId = bar.requestIdFor(options.id)
    // Send first: if the message is invalid (too big) this throws before the item is registered.
    bar.send({
      type: "item.create",
      requestId,
      id: options.id,
      ...(icon !== undefined && { icon }),
      ...(options.label !== undefined && { label: options.label }),
      ...(options.tooltip !== undefined && { tooltip: options.tooltip }),
    })
    const item = new StatusItem(options.id, bar, requestId)
    bar.items.set(options.id, item)
    return item
  }

  /**
   * Changes the icon. Pass an empty string to remove it.
   *
   * @example
   * ```ts
   * item.setIcon(fs.readFileSync("mail-unread.svg"))
   * ```
   */
  setIcon(icon: Uint8Array | string): void {
    this.#update({ icon: toIconDataUrl(icon) })
  }

  /**
   * Changes the label. Pass an empty string to remove it.
   *
   * @example
   * ```ts
   * item.setLabel("42%")
   * ```
   */
  setLabel(label: string): void {
    assertLabel(label)
    this.#update({ label })
  }

  /**
   * Changes the tooltip. Pass an empty string to remove it.
   *
   * @example
   * ```ts
   * item.setTooltip("Last synced just now")
   * ```
   */
  setTooltip(tooltip: string): void {
    assertTooltip(tooltip)
    this.#update({ tooltip })
  }

  /**
   * Hides or shows the item without removing it.
   *
   * @example
   * ```ts
   * item.setVisible(false) // gone from the bar, still yours
   * ```
   */
  setVisible(visible: boolean): void {
    this.#update({ visible })
  }

  /**
   * Takes the item out of the bar for good. Create it again to bring it back.
   *
   * @example
   * ```ts
   * process.on("exit", () => item.remove())
   * ```
   */
  remove(): void {
    if (this.#removed) return
    this.#removed = true
    this.#bar.items.delete(this.id)
    this.#bar.send({
      type: "item.remove",
      requestId: this.#bar.requestIdFor(this.id),
      id: this.id,
    })
  }

  /**
   * Runs when the user clicks the item.
   *
   * @example
   * ```ts
   * item.on("click", ({ button }) => {
   *   if (button === "left") openMyWindow()
   * })
   * ```
   */
  on(event: "click", listener: Listener<ClickEvent>): this
  /**
   * Runs when the bar refuses something about this item, for example `"limit_reached"`.
   * Without a listener the error goes to the connection's `error` listeners.
   *
   * @example
   * ```ts
   * item.on("error", (error) => console.error(error.code))
   * ```
   */
  on(event: "error", listener: Listener<BarError>): this
  on(event: "click" | "error", listener: Listener<never>): this {
    if (event === "click") this.#clickListeners.add(listener as Listener<ClickEvent>)
    else this.#errorListeners.add(listener as Listener<BarError>)
    return this
  }

  /** Stops a listener added with {@link StatusItem.on}. */
  off(event: "click" | "error", listener: Listener<never>): this {
    if (event === "click") this.#clickListeners.delete(listener as Listener<ClickEvent>)
    else this.#errorListeners.delete(listener as Listener<BarError>)
    return this
  }

  /** @internal The bar refused to create this item, so it does not exist there. */
  markGone(): void {
    this.#removed = true
  }

  /** @internal Called by the connection when the bar reports a click. */
  emitClick(button: MouseButton): void {
    for (const listener of this.#clickListeners) callSafely(() => listener({ button }))
  }

  /** @internal Returns false when nobody listens, so the connection can report it. */
  emitError(error: BarError): boolean {
    if (this.#errorListeners.size === 0) return false
    for (const listener of this.#errorListeners) callSafely(() => listener(error))
    return true
  }

  #update(fields: { icon?: string; label?: string; tooltip?: string; visible?: boolean }): void {
    if (this.#removed) throw new Error(`Item ${JSON.stringify(this.id)} was removed`)
    this.#bar.send({
      type: "item.update",
      requestId: this.#bar.requestIdFor(this.id),
      id: this.id,
      ...fields,
    })
  }
}
