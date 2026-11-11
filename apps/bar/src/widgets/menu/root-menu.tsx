import { useCallback, useEffect, useReducer, useRef, type MouseEvent } from "react"
import { commands, events } from "@/shared/bindings"
import {
  initialMenuState,
  MenuList,
  menuReducer,
  MenuPanel,
  useMenuKeyboard,
  useSafeTriangle,
  type MenuState,
} from "@/features/menu"
import { useLatest } from "@/features/menu/hooks/use-latest"
import { firstIndex } from "@/features/menu/lib/navigation"

/** How long the picked row stays off, like the blink of a native menu. */
const BLINK_MS = 45

/**
 * The main menu window. It owns the keyboard and the highlight state for both levels: the
 * submenu is another window that only draws what it is told (see sub-menu.tsx).
 */
export default function RootMenu() {
  const [state, dispatch] = useReducer(menuReducer, initialMenuState)
  const stateRef = useLatest(state)
  const safe = useSafeTriangle()
  const picking = useRef(false)

  const patch = useCallback((next: Partial<MenuState>) => dispatch({ type: "patch", patch: next }), [])

  // Subscribe before asking for the current menu, so an open event can't slip through.
  useEffect(() => {
    let cancelled = false
    let gotOpen = false
    let stops: Array<() => void> = []

    async function subscribe() {
      try {
        const unlisten = await Promise.all([
          events.menuOpen.listen(({ payload }) => {
            gotOpen = true
            picking.current = false
            safe.setPlacement(null)
            dispatch({ type: "open", menu: payload })
          }),
          events.menuChanged.listen(({ payload }) => {
            safe.setPlacement(null)
            dispatch({ type: "changed", itemId: payload.itemId, entries: payload.menu })
          }),
          events.menuClose.listen(({ payload }) => {
            safe.cancel()
            dispatch({ type: "closing", itemId: payload.itemId })
          }),
          events.submenuPlaced.listen(({ payload }) =>
            // Generated f64 fields are `number | null` (NaN); a real placement is never NaN.
            safe.setPlacement({
              side: payload.side,
              x: payload.x ?? 0,
              y: payload.y ?? 0,
              width: payload.width ?? 0,
              height: payload.height ?? 0,
            }),
          ),
          events.submenuHide.listen(() => {
            safe.setPlacement(null)
            patch({ sub: null, subActive: -1, level: "root" })
          }),
          events.submenuHovered.listen(({ payload }) => {
            if (payload.index === null) return
            safe.cancel()
            patch({ level: "sub", subActive: payload.index })
          }),
        ])
        if (cancelled) {
          unlisten.forEach((stop) => stop())
          return
        }
        stops = unlisten
        // A late answer must not overwrite a newer menuOpen event.
        const current = await commands.getOpenMenu()
        if (!cancelled && !gotOpen && current) dispatch({ type: "open", menu: current })
      } catch (error) {
        console.error("Could not start the menu window:", error)
      }
    }

    void subscribe()
    return () => {
      cancelled = true
      stops.forEach((stop) => stop())
    }
  }, [safe, patch])

  const rootEntries = state.menu?.menu ?? []
  const subEntries = state.sub !== null ? (rootEntries[state.sub]?.submenu ?? []) : []

  const closeMenu = useCallback(() => void commands.closeMenu(), [])

  const closeSubmenu = useCallback(() => {
    if (stateRef.current.sub === null) return
    void commands.closeSubmenu()
    safe.setPlacement(null)
    patch({ sub: null, subActive: -1, level: "root" })
  }, [stateRef, safe, patch])

  const openSubmenu = useCallback(
    (index: number, byKeyboard: boolean) => {
      const current = stateRef.current
      const entry = current.menu?.menu[index]
      if (!entry || entry.kind !== "submenu" || !entry.enabled) return

      if (current.sub !== index) {
        // The row's top, relative to this window (the panel starts at 0,0).
        const top = document.querySelector<HTMLElement>(`[data-row="${index}"]`)?.getBoundingClientRect().top ?? 0
        void commands.openSubmenu(index, top)
      }
      if (!byKeyboard) {
        patch({ sub: index })
        return
      }
      const first = firstIndex(entry.submenu)
      patch({ sub: index, level: first >= 0 ? "sub" : "root", subActive: first })
      if (first >= 0) void commands.setSubmenuHighlight(first)
    },
    [stateRef, patch],
  )

  const applyHover = useCallback(
    (index: number) => {
      const entry = stateRef.current.menu?.menu[index]
      if (!entry || entry.kind === "separator") return
      patch({ active: entry.enabled ? index : -1, level: "root" })
      if (entry.kind === "submenu" && entry.enabled) openSubmenu(index, false)
      else closeSubmenu()
    },
    [stateRef, patch, openSubmenu, closeSubmenu],
  )

  const pick = useCallback(
    async (index: number, level: "root" | "sub") => {
      const current = stateRef.current
      const menu = current.menu
      if (!menu || current.closing || picking.current) return
      const entries = level === "root" ? menu.menu : current.sub !== null ? (menu.menu[current.sub]?.submenu ?? []) : []
      const entry = entries[index]
      if (!entry?.id || !entry.enabled || entry.kind === "separator" || entry.kind === "submenu") return

      picking.current = true
      if (level === "root") {
        patch({ blink: true })
        await new Promise((resolve) => window.setTimeout(resolve, BLINK_MS))
        patch({ blink: false })
      }
      try {
        const result = await commands.menuItemClicked(menu.itemId, entry.id)
        if (result.status === "error") console.error("Could not activate the menu item:", result.error)
      } finally {
        picking.current = false
      }
    },
    [stateRef, patch],
  )

  const onHover = useCallback(
    (index: number, event: MouseEvent) => {
      const current = stateRef.current
      if (current.closing) return
      const point = { x: event.clientX, y: event.clientY }
      if (index === current.active && current.level === "root") {
        safe.track(point)
        return
      }
      safe.move(point, () => applyHover(index))
    },
    [stateRef, safe, applyHover],
  )

  const onLeave = useCallback(() => {
    // With a submenu open the trigger row stays lit while the pointer is over the submenu.
    if (stateRef.current.sub !== null) return
    safe.cancel()
    patch({ active: -1 })
  }, [stateRef, safe, patch])

  const level = state.level
  useMenuKeyboard(
    {
      entries: level === "root" ? rootEntries : subEntries,
      level,
      active: level === "root" ? state.active : state.subActive,
      sub: state.sub,
      setActive: (index) => {
        if (level === "root") {
          // Moving off the trigger row closes its submenu.
          if (stateRef.current.sub !== null && stateRef.current.sub !== index) closeSubmenu()
          patch({ active: index })
        } else {
          patch({ subActive: index })
          void commands.setSubmenuHighlight(index)
        }
      },
      openSubmenu: (index) => openSubmenu(index, true),
      closeSubmenu,
      pick: (index) => void pick(index, level),
      closeMenu,
    },
    state.menu !== null && !state.closing,
  )

  const menu = state.menu
  if (!menu) return null

  return (
    <MenuPanel
      key={state.openId}
      closing={state.closing}
      acrylic={menu.anchor.acrylic}
      label="Barista menu"
      activeId={state.active >= 0 ? `menu-${state.active}` : undefined}
      onSize={(width, height) => void commands.menuResized(width, height)}
    >
      <MenuList
        entries={menu.menu}
        highlighted={state.blink ? -1 : state.active}
        expanded={state.sub}
        idPrefix="menu"
        onHover={onHover}
        onLeave={onLeave}
        onPick={(index) => void pick(index, "root")}
      />
    </MenuPanel>
  )
}
