import { Menubar, MenuWindow } from "@/widgets";
import { getCurrentWindow } from "@tauri-apps/api/window";

// "menu" and "menu-sub" are the two menu windows; everything else is the bar.
const MENU_LABELS = ["menu", "menu-sub"];

export default function App() {
  return MENU_LABELS.includes(getCurrentWindow().label) ? <MenuWindow /> : <Menubar />
}
