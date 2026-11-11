import { getCurrentWindow } from "@tauri-apps/api/window"
import RootMenu from "./root-menu"
import SubMenu from "./sub-menu"

// Two windows load this same page: `menu` (the main panel) and `menu-sub` (the open submenu).
// Each is exactly one panel, because the Windows acrylic blur covers a whole window (ADR 0013).
const SUBMENU_LABEL = "menu-sub"

export default function MenuWindow() {
  return getCurrentWindow().label === SUBMENU_LABEL ? <SubMenu /> : <RootMenu />
}
