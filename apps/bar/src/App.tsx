import { Menubar, MenuWindow, PanelWindow } from "@/widgets";
import { getCurrentWindow } from "@tauri-apps/api/window";

// "menu" and "menu-sub" are the two menu windows; everything else is the bar.
const MENU_LABELS = ["menu", "menu-sub"];

export default function App() {
  const label = getCurrentWindow().label
  if (label === "panel") return <PanelWindow />
  return MENU_LABELS.includes(label) ? <MenuWindow /> : <Menubar />
}
