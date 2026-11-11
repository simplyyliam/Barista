import { Menu, type MenuItemOptions } from "./menu.js"
import { resolveIcon, type IconInput } from "./icons.js"
import { assertPopoverSize, type Placement, type PopoverOptions } from "./popover.js"
import type { MouseButton, ScreenRect } from "./protocol.js"
import { BarError, connect, type StatusBar } from "./status-bar.js"
import { assertItemId, assertLabel, assertTooltip, toIconDataUrl } from "./validate.js"

/** What you get when the user clicks your item. */
export interface ClickEvent {
  button: MouseButton
  /**
   * Where the item is on screen, in physical pixels, when the bar knows it. Useful if you place
   * your own window; with {@link StatusItem.setPopover} the bar does that for you.
   */
  anchor?: ScreenRect
}

/** Options for {@link StatusItem.create}. */
export interface StatusItemOptions {
  /**
   * Your own name for the item, unique within your app. 1 to 64 characters from
   * `a-z 0-9 - _ .`
   */
  id: string
  /**
   * An icon object (`import { Clock01Icon } from "@hugeicons/core-free-icons"`, or an icon from
   * `lucide`), `{ icon, color, strokeWidth, height }`, SVG text, PNG or SVG bytes
   * (`fs.readFileSync("icon.svg")`) or a `data:` URL. Max 256 KB.
   */
  icon?: IconInput
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
  readonly #openListeners = new Set<Listener<void>>()
  readonly #closeListeners = new Set<Listener<void>>()
  #removed = false
  #menu: Menu | undefined
  #popover: PopoverOptions | undefined
  /** The popover window is up (or being shown) because the bar asked for it. */
  #popoverOpen = false
  /** Windows we already listen to, so `setPopover` with the same window twice adds one listener. */
  readonly #watchedWindows = new WeakSet<object>()

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
    const icon = options.icon === undefined ? undefined : toIconDataUrl(resolveIcon(options.icon))
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
   * Changes the icon. Pass an icon object from Hugeicons or Lucide (no SVG needed), `{ icon,
   * color, strokeWidth, height }`, SVG text, bytes or a `data:` URL. An empty string removes it.
   * Icon objects are cropped to their real glyph when `@resvg/resvg-js` is installed.
   *
   * @example
   * ```ts
   * item.setIcon(fs.readFileSync("mail-unread.svg"))
   * ```
   */
  setIcon(icon: IconInput): void {
    this.#update({ icon: toIconDataUrl(resolveIcon(icon)) })
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
   * Gives the item a menu: clicking the item then opens it (a middle click still fires
   * `on("click")`). Pass a {@link Menu}, a template, or `null` to remove the menu. Call it
   * again after changing entries (labels, `enabled`...) to show the change. Throws a
   * `TypeError` if the template breaks a rule (see {@link Menu.fromTemplate}).
   *
   * @example
   * ```ts
   * item.setMenu([
   *   { label: "Start", click: () => timer.start() },
   *   { id: "sounds", label: "Sounds", type: "checkbox", checked: true },
   *   { type: "separator" },
   *   { role: "quit" },
   * ])
   * ```
   */
  setMenu(menu: Menu | MenuItemOptions[] | null): Menu | null {
    if (this.#removed) throw new Error(`Item ${JSON.stringify(this.id)} was removed`)
    const built = menu === null ? null : menu instanceof Menu ? menu : Menu.fromTemplate(menu)
    this.#bar.send({
      type: "item.setMenu",
      requestId: this.#bar.requestIdFor(this.id),
      id: this.id,
      menu: built?.toCheckedWire() ?? [],
    })
    this.#menu = built ?? undefined
    return built
  }

  /**
   * Gives the item a popover: your own window, shown right under the icon when the user
   * left-clicks it. The bar tells you where (`window.show(placement)`), hides it when the user
   * clicks away or opens something else, and shows an "open" state on the icon. A right click
   * still opens the menu if you set one; a middle click fires `on("click")`.
   *
   * `width` and `height` are the size you would like in logical pixels (48 to 1024); the bar
   * keeps the window on screen. Pass `null` to remove the popover. Throws a `TypeError` for a
   * bad size.
   *
   * @example
   * ```ts
   * import { electronPopover } from "@barista/sdk/electron"
   * item.setPopover({ window: electronPopover(win, { screen }), width: 320, height: 380 })
   * ```
   */
  setPopover(popover: PopoverOptions | null): void {
    if (this.#removed) throw new Error(`Item ${JSON.stringify(this.id)} was removed`)
    if (popover !== null) assertPopoverSize(popover.width, popover.height)
    // A popover that is open while it is replaced goes away, and the bar must hear about it
    // (or it keeps the icon "open" and the next click only toggles it shut).
    if (this.#popoverOpen) {
      this.#closeOpenPopover()
      this.#bar.send({ type: "popover.closed", id: this.id })
    }
    this.#popover = popover ?? undefined
    if (popover && !this.#watchedWindows.has(popover.window)) {
      this.#watchedWindows.add(popover.window)
      popover.window.onClose(() => this.#windowClosed(popover.window))
    }
    this.#bar.send({
      type: "item.setPopover",
      requestId: this.#bar.requestIdFor(this.id),
      id: this.id,
      width: popover?.width ?? 0,
      height: popover?.height ?? 0,
    })
  }

  /**
   * The menu set with {@link StatusItem.setMenu}, if any.
   *
   * @example
   * ```ts
   * const pause = item.menu?.getItemById("pause")
   * if (pause) {
   *   pause.enabled = true
   *   item.setMenu(item.menu!) // show the change
   * }
   * ```
   */
  get menu(): Menu | undefined {
    return this.#menu
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
    this.#closeOpenPopover()
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
   * Runs right before the item's menu or popover shows (a good moment to refresh a label or the
   * menu), after the user opened it.
   *
   * @example
   * ```ts
   * item.on("open", () => item.setLabel(`${minutesLeft()} left`))
   * ```
   */
  on(event: "open", listener: Listener<void>): this
  /**
   * Runs when the item's menu or popover closes, whatever closed it.
   *
   * @example
   * ```ts
   * item.on("close", () => saveDraft())
   * ```
   */
  on(event: "close", listener: Listener<void>): this
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
  on(event: "click" | "open" | "close" | "error", listener: Listener<never>): this {
    this.#listeners(event).add(listener)
    return this
  }

  /** Stops a listener added with {@link StatusItem.on}. */
  off(event: "click" | "open" | "close" | "error", listener: Listener<never>): this {
    this.#listeners(event).delete(listener)
    return this
  }

  #listeners(event: "click" | "open" | "close" | "error"): Set<Listener<never>> {
    const sets = {
      click: this.#clickListeners,
      open: this.#openListeners,
      close: this.#closeListeners,
      error: this.#errorListeners,
    }
    return sets[event] as Set<Listener<never>>
  }

  /** @internal The bar refused to create this item, so it does not exist there. */
  markGone(): void {
    this.#removed = true
  }

  /** @internal Called by the connection when the bar reports a click. */
  emitClick(button: MouseButton, anchor?: ScreenRect | null): void {
    const event: ClickEvent = anchor ? { button, anchor } : { button }
    for (const listener of this.#clickListeners) callSafely(() => listener(event))
  }

  /** @internal The bar opened this item's menu. */
  emitMenuOpened(): void {
    for (const listener of this.#openListeners) callSafely(() => listener())
  }

  /** @internal The bar closed this item's menu. */
  emitMenuClosed(): void {
    for (const listener of this.#closeListeners) callSafely(() => listener())
  }

  /**
   * @internal The bar says: show the popover here. Runs `open`, shows the window, then confirms
   * with `popover.shown`. If showing fails, or the bar hid it meanwhile, it does not confirm.
   */
  async emitPopoverShow(placement: Placement): Promise<void> {
    const popover = this.#popover
    // A removed item is already gone at the bar: answering would only earn a `not_found`.
    if (this.#removed) return
    if (!popover) {
      // The bar thinks there is a popover but we have none: tell it, so the icon is not stuck.
      this.#bar.send({ type: "popover.closed", id: this.id })
      return
    }
    // A repeated show (the bar asking again) must not run `open` twice without a `close`.
    const alreadyOpen = this.#popoverOpen
    this.#popoverOpen = true
    if (!alreadyOpen) for (const listener of this.#openListeners) callSafely(() => listener())
    try {
      await popover.window.show(placement)
    } catch (error) {
      this.#popoverOpen = false
      this.#bar.send({ type: "popover.closed", id: this.id })
      for (const listener of this.#closeListeners) callSafely(() => listener())
      // A window that cannot be shown is the window adapter's problem, not a reason to crash the
      // whole app: report it and carry on.
      console.error(`[barista] could not show the popover of ${JSON.stringify(this.id)}:`, error)
      return
    }
    if (!this.#popoverOpen) {
      // The bar hid it while we were still showing it (a fast second click): finish the hide.
      callSafely(() => popover.window.hide())
      return
    }
    this.#bar.send({ type: "popover.shown", id: this.id })
  }

  /** @internal The bar says: hide the popover. It does not expect an answer. */
  emitPopoverHide(): void {
    this.#closeOpenPopover()
  }

  /** Hides the popover window if the bar had it open, and runs `close`. No message goes out. */
  #closeOpenPopover(): void {
    if (!this.#popoverOpen) return
    this.#popoverOpen = false
    const popover = this.#popover
    if (popover) callSafely(() => popover.window.hide())
    for (const listener of this.#closeListeners) callSafely(() => listener())
  }

  /** The window closed itself (Esc, closed by the user): the bar must forget the open state. */
  #windowClosed(window: object): void {
    if (!this.#popoverOpen || this.#popover?.window !== window) return
    this.#popoverOpen = false
    this.#bar.send({ type: "popover.closed", id: this.id })
    for (const listener of this.#closeListeners) callSafely(() => listener())
  }

  /**
   * @internal The bar reports a pick in this item's menu: run its `click`, or the default for
   * a role without one (quit).
   */
  emitMenuClick(menuItemId: string, checked: boolean | undefined): void {
    const item = this.#menu?.applyClick(menuItemId, checked)
    if (!item) return
    if (item.click) {
      const click = item.click
      callSafely(() => click(item, checked === undefined ? {} : { checked }))
    } else if (item.role === "quit") {
      this.#bar.quit()
    }
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
