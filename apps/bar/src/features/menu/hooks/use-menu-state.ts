import type { MenuEntry, MenuOpen, SubmenuSide } from "@/shared/bindings"
import { isSelectable } from "../lib/navigation"

/** Where Rust put the submenu window: logical px, relative to the main menu window. */
export type SubmenuPlacement = {
  side: SubmenuSide
  x: number
  y: number
  width: number
  height: number
}

/**
 * Everything the main menu window knows. One highlight for mouse and keyboard:
 * `active` is the main row, `subActive` the submenu row, `level` says which one the keyboard drives.
 */
export type MenuState = {
  menu: MenuOpen | null
  /** Bumped on every open; used as the panel's key so each open starts fresh. */
  openId: number
  closing: boolean
  active: number
  sub: number | null
  subActive: number
  level: "root" | "sub"
  /** The picked row is off for a moment, like the blink of a native menu. */
  blink: boolean
}

export const initialMenuState: MenuState = {
  menu: null,
  openId: 0,
  closing: false,
  active: -1,
  sub: null,
  subActive: -1,
  level: "root",
  blink: false,
}

export type MenuAction =
  | { type: "open"; menu: MenuOpen }
  | { type: "changed"; itemId: string; entries: MenuEntry[] }
  | { type: "closing"; itemId: string }
  | { type: "patch"; patch: Partial<MenuState> }

export function menuReducer(state: MenuState, action: MenuAction): MenuState {
  switch (action.type) {
    case "open":
      return {
        ...initialMenuState,
        menu: action.menu,
        openId: state.openId + 1,
      }

    case "changed": {
      if (state.menu?.itemId !== action.itemId) return state
      const current = action.entries[state.active]
      return {
        ...state,
        menu: { ...state.menu, menu: action.entries },
        active: current && isSelectable(current) ? state.active : -1,
        sub: null,
        subActive: -1,
        level: "root",
      }
    }

    case "closing":
      return state.menu?.itemId === action.itemId ? { ...state, closing: true } : state

    case "patch":
      return { ...state, ...action.patch }
  }
}
