import { useEffect, useSyncExternalStore, type ReactNode } from "react"
import { commands } from "@/shared/bindings"
import { PanelBody, PanelFrame, useOpenPanel } from "@/features/panel"
import RootMenu from "@/widgets/menu/root-menu"
import Menubar from "@/widgets/menubar/menubar"
import { Motion } from "@/shared/motion"
import { getAnchor, getOpenMenuItem, getOpenPanelItem, getSize, subscribeSizes } from "./backend"

const EDGE = 8
const clamp = (n: number, min: number, max: number) => Math.min(Math.max(n, min), Math.max(min, max))

/**
 * In the app, a panel and a menu are windows, so their components fill them (`fixed inset-0`).
 * A transformed parent becomes the containing block of its fixed children, so each gets a box of
 * the size it reported with `panelResized` / `menuResized`, like Rust would size the window, and
 * sits where Rust would put it: a menu under the item that opened it, a panel right-aligned to its item.
 */
function WindowBox({
  which,
  fallback,
  anchored = true,
  children,
}: {
  which: "panel" | "menu"
  /** False in a tile: the bar is hidden there, so the window keeps its fixed spot. */
  anchored?: boolean
  /** Where it goes when it has no item to hang from (Tailwind classes). */
  fallback: string
  children: ReactNode
}) {
  const size = useSyncExternalStore(subscribeSizes, () => getSize(which))
  const anchor = useSyncExternalStore(subscribeSizes, () => getAnchor(which))

  // Always mounted, even before it has a size: its content is what reports the size.
  if (!size)
    return (
      <div className="absolute" style={{ width: 0, height: 0, visibility: "hidden" }}>
        {children}
      </div>
    )

  const placed = anchor && anchored
    ? {
        left: clamp(which === "panel" ? anchor.right - size.width : anchor.left, EDGE, window.innerWidth - size.width - EDGE),
        top: anchor.bottom + (which === "panel" ? 6 : 4),
      }
    : null
  return (
    <div
      data-demo-window={which}
      className={`absolute transform-gpu ${placed ? "" : fallback}`}
      style={{ width: size.width, height: size.height, ...placed }}
    >
      {children}
    </div>
  )
}

/** Runs `fn` once the item is on screen (the bar draws after the first render). */
function whenThere(selector: string, fn: () => void) {
  let tries = 0
  let frame = 0
  const look = () => {
    if (document.querySelector(selector) || ++tries > 60) return fn()
    frame = requestAnimationFrame(look)
  }
  frame = requestAnimationFrame(look)
  return () => cancelAnimationFrame(frame)
}

/** Opens the panel a scene starts with. */
function openStart(scene: string) {
  if (scene === "notifications")
    return whenThere('[data-item-id="system/clock"]', () => void commands.openPanel("notifications", "system/clock", 300, 400, "none"))
  if (scene === "control")
    return whenThere('[data-item-id="system/control-center"]', () => void commands.openPanel("control-center", "system/control-center", 320, 480, "none"))
  if (scene === "tray")
    return whenThere('[data-item-id="tray/arrow"]', () => void commands.openPanel("tray", "tray/arrow", 212, 108, "none"))
}

/** A pretend desktop: the real bar on top, and the real panel and menu where their windows would be. */
export default function Stage({ scene, bare = false }: { scene: string; bare?: boolean }) {
  const open = useOpenPanel()

  // What is already open when the page loads, so a still frame on the landing page is not empty.
  useEffect(() => openStart(scene), [scene])

  // Like the real bar: a click or a key press outside an open menu or panel dismisses it. A click on the item
  // that opened it is left alone, because that click toggles it.
  useEffect(() => {
    if (bare) return
    const onDown = (e: PointerEvent) => {
      const target = e.target instanceof Element ? e.target : null
      const item = target?.closest("[data-item-id]")?.getAttribute("data-item-id") ?? null
      const inside = (which: string) => Boolean(target?.closest(`[data-demo-window="${which}"]`))
      if (getSize("menu") && !inside("menu") && item !== getOpenMenuItem()) void commands.closeMenu()
      if (getSize("panel") && !inside("panel") && item !== getOpenPanelItem()) void commands.closePanel()
    }
    const onKey = (e: KeyboardEvent) => {
      if (e.key !== "Escape") return
      if (getSize("menu")) void commands.closeMenu()
      if (getSize("panel")) void commands.closePanel()
    }
    document.addEventListener("pointerdown", onDown, true)
    document.addEventListener("keydown", onKey)
    return () => {
      document.removeEventListener("pointerdown", onDown, true)
      document.removeEventListener("keydown", onKey)
    }
  }, [bare])

  // The small tiles have no bar to reopen a panel with, so their panel always comes back (Esc, say).
  useEffect(() => {
    if (bare && !open) return openStart(scene)
  }, [bare, open, scene])

  return (
    <div
      className="relative h-full w-full overflow-hidden bg-slate-900 bg-cover bg-center"
      style={{ backgroundImage: `url(${import.meta.env.BASE_URL}wallpaper.jpg)` }}
      onPointerDown={(e) => {
        // Like the real bar, a click on the desktop dismisses the panel. Not in a tile: it is the content.
        if (!bare && e.target === e.currentTarget) void commands.closePanel()
      }}
    >
      {/* bare: only the panel, for the small tiles on the landing page */}
      <div className={bare ? "sr-only" : "h-[28px] w-full"}>
        <Menubar />
      </div>
      <WindowBox which="panel" anchored={!bare} fallback={bare ? "top-2 left-2" : "top-[34px] right-2"}>
        <Motion>
          <PanelFrame acrylic={open?.acrylic ?? true} chrome={open?.chrome ?? "none"} openKey={open ? `${open.name}|${open.anchor}` : null}>
            <PanelBody name={open?.name ?? null} />
          </PanelFrame>
        </Motion>
      </WindowBox>
      <WindowBox which="menu" fallback="top-[34px] left-24">
        <RootMenu />
      </WindowBox>
    </div>
  )
}
