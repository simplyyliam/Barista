import { useEffect, useRef } from "react"
import type { MenuEntry } from "@/shared/bindings"
import { firstIndex, lastIndex, nextIndex, typeaheadIndex } from "../lib/navigation"
import { useLatest } from "./use-latest"

/** Typed letters join into one word until this long a pause. */
const TYPEAHEAD_RESET_MS = 600

export type MenuKeyboardApi = {
  /** The entries the keyboard drives right now (the submenu's while it has the keyboard). */
  entries: MenuEntry[]
  level: "root" | "sub"
  /** The highlighted row of that level (-1: none). */
  active: number
  /** The main entry that has an open submenu, if any. */
  sub: number | null
  setActive: (index: number) => void
  openSubmenu: (index: number) => void
  closeSubmenu: () => void
  pick: (index: number) => void
  closeMenu: () => void
}

/**
 * Keyboard for the main menu window (the submenu window never has focus, so this one window
 * handles both levels): Up/Down, Home/End, Enter/Space, Esc, Right/Left for submenus, Tab closes,
 * and typing a letter jumps to the next row that starts with it.
 */
export function useMenuKeyboard(api: MenuKeyboardApi, enabled: boolean) {
  const apiRef = useLatest(api)
  const typed = useRef({ text: "", at: 0 })

  useEffect(() => {
    if (!enabled) return

    function onKeyDown(event: KeyboardEvent) {
      const a = apiRef.current
      const entry = a.entries[a.active]
      const submenuTrigger = a.level === "root" && entry?.kind === "submenu" && entry.enabled

      switch (event.key) {
        case "Escape":
          event.preventDefault()
          if (a.level === "sub") a.closeSubmenu()
          else a.closeMenu()
          return

        case "Tab":
          event.preventDefault()
          a.closeMenu()
          return

        case "ArrowDown":
        case "ArrowUp": {
          event.preventDefault()
          const next = nextIndex(a.entries, a.active, event.key === "ArrowDown" ? 1 : -1)
          if (next >= 0) a.setActive(next)
          return
        }

        case "Home":
        case "End": {
          event.preventDefault()
          const next = event.key === "Home" ? firstIndex(a.entries) : lastIndex(a.entries)
          if (next >= 0) a.setActive(next)
          return
        }

        case "ArrowRight":
          if (submenuTrigger) {
            event.preventDefault()
            a.openSubmenu(a.active)
          }
          return

        case "ArrowLeft":
          if (a.level === "sub") {
            event.preventDefault()
            a.closeSubmenu()
          }
          return

        case "Enter":
        case " ":
          event.preventDefault()
          if (submenuTrigger) a.openSubmenu(a.active)
          else if (entry) a.pick(a.active)
          return
      }

      // A letter: type-to-jump.
      if (event.key.length === 1 && !event.ctrlKey && !event.metaKey && !event.altKey) {
        event.preventDefault()
        const now = performance.now()
        const word = now - typed.current.at > TYPEAHEAD_RESET_MS ? "" : typed.current.text
        typed.current = { text: word + event.key, at: now }
        const found = typeaheadIndex(a.entries, typed.current.text, a.active)
        if (found >= 0) a.setActive(found)
      }
    }

    window.addEventListener("keydown", onKeyDown)
    return () => window.removeEventListener("keydown", onKeyDown)
  }, [enabled, apiRef])
}
