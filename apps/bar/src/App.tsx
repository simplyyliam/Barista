import { Menubar, MenuWindow } from "@/widgets";
import { getCurrentWindow } from "@tauri-apps/api/window";

export default function App() {
  return getCurrentWindow().label === "menu" ? <MenuWindow /> : <Menubar />
}
