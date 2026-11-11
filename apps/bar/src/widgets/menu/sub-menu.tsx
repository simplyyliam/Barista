import { useCallback, useEffect, useRef, useState } from "react"
import { commands, events, type MenuEntry } from "@/shared/bindings"
import { MenuList, MenuPanel } from "@/features/menu"

const BLINK_MS = 45

type Shown = { itemId: string; entries: MenuEntry[]; showId: number }

/**
 * The submenu window. It never takes focus, so it has no keyboard: it draws the entries it
 * gets, shows the highlight Rust tells it to (keyboard) or the one under the mouse, and
 * reports mouse hovers back so the main menu window can follow.
 */
export default function SubMenu() {
  const [shown, setShown] = useState<Shown | null>(null)
  const [highlight, setHighlight] = useState(-1)
  const [blink, setBlink] = useState(false)
  const [closing, setClosing] = useState(false)
  const [acrylic, setAcrylic] = useState(true)
  const picking = useRef(false)

  useEffect(() => {
    let cancelled = false
    let stops: Array<() => void> = []

    async function subscribe() {
      try {
        const unlisten = await Promise.all([
          events.submenuShow.listen(({ payload }) => {
            picking.current = false
            setClosing(false)
            setHighlight(-1)
            setShown((current) => ({
              itemId: payload.itemId,
              entries: payload.entries,
              showId: (current?.showId ?? 0) + 1,
            }))
          }),
          events.submenuHighlight.listen(({ payload }) => setHighlight(payload.index ?? -1)),
          events.submenuHide.listen(() => setShown(null)),
          events.menuClose.listen(() => setClosing(true)),
          events.menuOpen.listen(({ payload }) => {
            setAcrylic(payload.anchor.acrylic)
            setClosing(false)
            setShown(null)
          }),
          events.menuChanged.listen(() => setShown(null)),
        ])
        if (cancelled) unlisten.forEach((stop) => stop())
        else stops = unlisten
      } catch (error) {
        console.error("Could not start the submenu window:", error)
      }
    }

    void subscribe()
    return () => {
      cancelled = true
      stops.forEach((stop) => stop())
    }
  }, [])

  const onHover = useCallback(
    (index: number) => {
      const entry = shown?.entries[index]
      if (!entry || entry.kind === "separator") return
      if (!entry.enabled) {
        setHighlight(-1)
        return
      }
      if (index === highlight) return
      setHighlight(index)
      void commands.submenuHover(index)
    },
    [shown, highlight],
  )

  const onLeave = useCallback(() => {
    setHighlight(-1)
    void commands.submenuHover(null)
  }, [])

  const onPick = useCallback(
    async (index: number) => {
      const entry = shown?.entries[index]
      if (!shown || !entry?.id || !entry.enabled || closing || picking.current) return
      if (entry.kind === "separator" || entry.kind === "submenu") return

      picking.current = true
      setBlink(true)
      await new Promise((resolve) => window.setTimeout(resolve, BLINK_MS))
      setBlink(false)
      try {
        const result = await commands.menuItemClicked(shown.itemId, entry.id)
        if (result.status === "error") console.error("Could not activate the menu item:", result.error)
      } finally {
        picking.current = false
      }
    },
    [shown, closing],
  )

  if (!shown) return null

  return (
    <MenuPanel
      key={shown.showId}
      closing={closing}
      acrylic={acrylic}
      label="Submenu"
      onSize={(width, height) => void commands.submenuResized(width, height)}
    >
      <MenuList
        entries={shown.entries}
        highlighted={blink ? -1 : highlight}
        idPrefix="submenu"
        onHover={onHover}
        onLeave={onLeave}
        onPick={(index) => void onPick(index)}
      />
    </MenuPanel>
  )
}
