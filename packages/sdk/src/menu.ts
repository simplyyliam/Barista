// Menus for status items, Electron-style. The bar draws them; only ids go over the wire and
// `click` callbacks stay here, found again by id when the bar reports `menu.click`.

import {
  MAX_ACCELERATOR_LEN,
  MAX_MENU_DEPTH,
  MAX_MENU_ICON_BYTES,
  MAX_MENU_ITEMS,
  type MenuItemType,
  type MenuRole,
  type MenuTemplateItem,
} from "./protocol.js"
import { assertItemId, assertLabel, assertText, toIconDataUrl } from "./validate.js"

/** What a menu `click` callback gets besides the item. */
export interface MenuClickEvent {
  /** New state of a checkbox or radio entry (the bar flips it before telling you). */
  checked?: boolean
}

/** One entry of a template for {@link Menu.fromTemplate}. Same fields as Electron's. */
export interface MenuItemOptions {
  /**
   * Your name for the entry, unique in the whole menu (`a-z 0-9 - _ .`). Optional: the SDK
   * makes one up if you leave it out, but set it to find the entry with `getItemById`.
   */
  id?: string
  label?: string
  /** `normal` (default), `checkbox`, `radio`, `separator` or `submenu` (default when `submenu` is set). */
  type?: MenuItemType
  /** For `checkbox` and `radio` entries. Adjacent radio entries form one group. */
  checked?: boolean
  /** Default `true`. A disabled entry is greyed out and can't be clicked. */
  enabled?: boolean
  /** Default `true`. */
  visible?: boolean
  /** Shown as text next to the label, like `"Ctrl+S"` (`CmdOrCtrl` becomes `Ctrl`). Not a hotkey. */
  accelerator?: string
  /** `quit`, `about` or `settings`: the bar places them in the standard spot and labels them if you don't. */
  role?: MenuRole
  /** PNG or SVG bytes or a `data:` URL, drawn at 16 px. Max 16 KB. */
  icon?: Uint8Array | string
  /** Entries of a submenu, at most 3 levels deep in total. */
  submenu?: MenuItemOptions[]
  /** Runs when the user picks the entry. */
  click?: (item: MenuItem, event: MenuClickEvent) => void
}

/** One entry of a {@link Menu}. Read `checked` to know the current state of a checkbox. */
export class MenuItem {
  readonly id: string
  readonly type: MenuItemType
  label: string | undefined
  checked: boolean | undefined
  enabled: boolean
  visible: boolean
  accelerator: string | undefined
  role: MenuRole | undefined
  /** Icon as a `data:` URL. */
  icon: string | undefined
  readonly submenu: Menu | undefined
  click: MenuItemOptions["click"]

  /** @internal Use {@link Menu.fromTemplate}. */
  constructor(id: string, options: MenuItemOptions, submenu: Menu | undefined) {
    this.id = id
    this.type = options.type ?? (options.submenu ? "submenu" : "normal")
    this.label = options.label
    this.checked = options.checked
    this.enabled = options.enabled ?? true
    this.visible = options.visible ?? true
    this.accelerator =
      options.accelerator === undefined ? undefined : toAcceleratorText(options.accelerator)
    this.role = options.role
    this.icon = options.icon === undefined ? undefined : toMenuIcon(options.icon)
    this.submenu = submenu
    this.click = options.click
  }
}

/**
 * Turns Electron's cross-platform names into what Windows users read: `CmdOrCtrl+S` → `Ctrl+S`.
 * @internal
 */
export function toAcceleratorText(accelerator: string): string {
  const names: Record<string, string> = {
    cmdorctrl: "Ctrl",
    commandorcontrol: "Ctrl",
    cmd: "Ctrl",
    command: "Ctrl",
    control: "Ctrl",
    ctrl: "Ctrl",
    option: "Alt",
    alt: "Alt",
    shift: "Shift",
    super: "Win",
    meta: "Win",
  }
  const text = accelerator
    .split("+")
    .map((part) => names[part.trim().toLowerCase()] ?? part.trim())
    .join("+")
  assertText("accelerator", text, MAX_ACCELERATOR_LEN)
  return text
}

function toMenuIcon(icon: Uint8Array | string): string {
  const url = toIconDataUrl(icon)
  const base64 = url.slice(url.indexOf(",") + 1)
  if (Buffer.byteLength(base64, "base64") > MAX_MENU_ICON_BYTES) {
    throw new TypeError(`A menu icon can be at most ${MAX_MENU_ICON_BYTES / 1024} KB`)
  }
  return url
}

/**
 * A menu for a status item. Build it from a template, give it to {@link StatusItem.setMenu},
 * and the bar shows it when the user clicks the item.
 *
 * @example
 * ```ts
 * const menu = BA.Menu.fromTemplate([
 *   { label: "Start", accelerator: "Ctrl+S", click: () => start() },
 *   { id: "sounds", label: "Sounds", type: "checkbox", checked: true },
 *   { type: "separator" },
 *   { role: "quit" },
 * ])
 * item.setMenu(menu)
 * ```
 */
export class Menu {
  readonly items: readonly MenuItem[]

  private constructor(items: MenuItem[]) {
    this.items = items
  }

  /**
   * Builds a menu from an Electron-style template and checks it the way the bar will: at most
   * 3 levels deep, at most 50 entries in total, ids unique. Throws a `TypeError` otherwise.
   *
   * @example
   * ```ts
   * const menu = BA.Menu.fromTemplate([
   *   { label: "Presets", submenu: [
   *     { id: "p25", label: "25 min", type: "radio", checked: true },
   *     { id: "p50", label: "50 min", type: "radio" },
   *   ] },
   * ])
   * ```
   */
  static fromTemplate(template: MenuItemOptions[]): Menu {
    if (!Array.isArray(template)) throw new TypeError("A menu template must be an array")
    const used = new Set<string>()
    let count = 0
    // First pass: user ids and limits, so generated ids never collide with them.
    const scan = (level: MenuItemOptions[], depth: number) => {
      if (depth > MAX_MENU_DEPTH) {
        throw new TypeError(`Menus can be nested at most ${MAX_MENU_DEPTH} levels deep`)
      }
      for (const options of level) {
        if (++count > MAX_MENU_ITEMS) {
          throw new TypeError(`A menu can have at most ${MAX_MENU_ITEMS} entries`)
        }
        if (options.id !== undefined) {
          assertItemId(options.id)
          if (used.has(options.id))
            throw new TypeError(`Menu id ${JSON.stringify(options.id)} is used twice`)
          used.add(options.id)
        }
        if (options.label !== undefined) assertLabel(options.label)
        const type = options.type ?? (options.submenu ? "submenu" : "normal")
        if (type === "submenu" && !options.submenu) {
          throw new TypeError("A submenu entry needs a submenu list")
        }
        if (options.submenu && type !== "submenu") {
          throw new TypeError("Only submenu entries can have a submenu")
        }
        if (options.checked !== undefined && type !== "checkbox" && type !== "radio") {
          throw new TypeError("Only checkbox and radio entries can be checked")
        }
        if (options.submenu) scan(options.submenu, depth + 1)
      }
    }
    scan(template, 1)

    let next = 0
    const freshId = () => {
      let id: string
      do id = `menu-${++next}`
      while (used.has(id))
      used.add(id)
      return id
    }
    const build = (level: MenuItemOptions[]): Menu =>
      new Menu(
        level.map((options) => {
          const submenu = options.submenu ? build(options.submenu) : undefined
          return new MenuItem(options.id ?? freshId(), options, submenu)
        }),
      )
    return build(template)
  }

  /**
   * Finds an entry anywhere in the menu, submenus included.
   *
   * @example
   * ```ts
   * if (menu.getItemById("sounds")?.checked) playSound()
   * ```
   */
  getItemById(id: string): MenuItem | undefined {
    for (const item of this.items) {
      if (item.id === id) return item
      const found = item.submenu?.getItemById(id)
      if (found) return found
    }
    return undefined
  }

  /** @internal The template as it goes over the wire (no callbacks, separators without ids). */
  toWire(): MenuTemplateItem[] {
    return this.items.map((item) => {
      const wire: MenuTemplateItem = {}
      if (item.type !== "separator") wire.id = item.id
      if (item.type !== "normal") wire.type = item.type
      if (item.label !== undefined) wire.label = item.label
      if (item.checked !== undefined) wire.checked = item.checked
      if (!item.enabled) wire.enabled = false
      if (!item.visible) wire.visible = false
      if (item.accelerator !== undefined) wire.accelerator = item.accelerator
      if (item.role !== undefined) wire.role = item.role
      if (item.icon !== undefined) wire.icon = item.icon
      if (item.submenu) wire.submenu = item.submenu.toWire()
      return wire
    })
  }

  /**
   * @internal The bar reported a click: mirror its checked state here (radio siblings too)
   * and return the entry.
   */
  applyClick(id: string, checked: boolean | undefined): MenuItem | undefined {
    const level = this.#levelOf(id)
    if (!level) return undefined
    const index = level.findIndex((item) => item.id === id)
    const item = level[index]
    if (!item) return undefined
    if (checked !== undefined) item.checked = checked
    if (item.type === "radio" && checked) {
      // Same rule as the bar: a radio group is a run of adjacent radio entries.
      const isRadio = (i: number) => level[i]?.type === "radio"
      for (let i = index - 1; isRadio(i); i--) level[i]!.checked = false
      for (let i = index + 1; isRadio(i); i++) level[i]!.checked = false
    }
    return item
  }

  #levelOf(id: string): readonly MenuItem[] | undefined {
    if (this.items.some((item) => item.id === id)) return this.items
    for (const item of this.items) {
      const found = item.submenu ? item.submenu.#levelOf(id) : undefined
      if (found) return found
    }
    return undefined
  }
}
