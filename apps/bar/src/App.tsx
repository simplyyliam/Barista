import { lazy, Suspense } from "react"
import { getCurrentWindow } from "@tauri-apps/api/window"

// Every window loads this same page, so each widget is a separate chunk: a window downloads, parses and
// keeps in memory only the code of its own widget (the bar does not carry the settings window, the
// panel does not carry the menu, ...). See docs/perf/0.8.md.
const Menubar = lazy(() => import("@/widgets/menubar/menubar"))
const MenuWindow = lazy(() => import("@/widgets/menu/menu"))
const PanelWindow = lazy(() => import("@/widgets/panel/panel"))
const DebugWindow = lazy(() => import("@/widgets/debug/debug"))
const SettingsWindow = lazy(() => import("@/widgets/settings/settings"))

// "menu" and "menu-sub" are the two menu windows; everything else is the bar.
const MENU_LABELS = ["menu", "menu-sub"]

function widget() {
  const label = getCurrentWindow().label
  if (label === "panel") return <PanelWindow />
  if (label === "debug") return <DebugWindow />
  if (label === "settings") return <SettingsWindow />
  return MENU_LABELS.includes(label) ? <MenuWindow /> : <Menubar />
}

export default function App() {
  return <Suspense fallback={null}>{widget()}</Suspense>
}
