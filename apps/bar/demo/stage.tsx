import { useEffect, useSyncExternalStore, type ReactNode } from "react"
import { commands } from "@/shared/bindings"
import { PanelBody, PanelFrame, useOpenPanel } from "@/features/panel"
import RootMenu from "@/widgets/menu/root-menu"
import Menubar from "@/widgets/menubar/menubar"
import { Motion } from "@/shared/motion"
import { getSize, subscribeSizes } from "./backend"

/**
 * In the app, a panel and a menu are windows, so their components fill them (`fixed inset-0`).
 * A transformed parent becomes the containing block of its fixed children, so each gets a box
 * of the size it reported with `panelResized` / `menuResized`, like Rust would size the window.
 */
function WindowBox({ which, className, children }: { which: "panel" | "menu"; className: string; children: ReactNode }) {
  const size = useSyncExternalStore(subscribeSizes, () => getSize(which))
  return (
    <div
      className={`absolute transform-gpu ${className}`}
      style={size ? { width: size.width, height: size.height } : { width: 0, height: 0, visibility: "hidden" }}
    >
      {children}
    </div>
  )
}

/** A pretend desktop: the real bar on top, and the real panel and menu where their windows would be. */
export default function Stage({ scene, bare = false }: { scene: string; bare?: boolean }) {
  const open = useOpenPanel()

  // What is already open when the page loads, so a still frame on the landing page is not empty.
  useEffect(() => {
    if (scene === "notifications") void commands.openPanel("notifications", "clock", null, null, "glass")
    if (scene === "control") void commands.openPanel("control-center", "control-center", null, null, "glass")
  }, [scene])

  return (
    <div
      className="relative h-full w-full overflow-hidden bg-slate-900 bg-cover bg-center"
      style={{ backgroundImage: `url(${import.meta.env.BASE_URL}wallpaper.jpg)` }}
      onPointerDown={(e) => {
        if (e.target === e.currentTarget) void commands.closePanel()
      }}
    >
      {/* bare: only the panel, for the small tiles on the landing page */}
      <div className={bare ? "sr-only" : "h-[28px] w-full bg-black/75 backdrop-blur-xl"}>
        <Menubar />
      </div>
      <WindowBox which="panel" className={bare ? "top-2 left-2" : "top-[34px] right-2"}>
        <Motion>
          <PanelFrame acrylic chrome="glass" openKey={open ? `${open.name}|${open.anchor}` : null}>
            <PanelBody name={open?.name ?? null} />
          </PanelFrame>
        </Motion>
      </WindowBox>
      <WindowBox which="menu" className="top-[34px] left-24">
        <RootMenu />
      </WindowBox>
    </div>
  )
}
