import {
  ComputerTerminal01Icon,
  DockIcon,
  GridViewIcon,
  InformationCircleIcon,
  PanelTopIcon,
  PlugSocketIcon,
  Settings01Icon,
  SidebarLeftIcon,
  SlidersHorizontalIcon,
  SourceCodeIcon,
} from "@barista/ui/icons"

export type PageId =
  | "general"
  | "menu-bar"
  | "dock"
  | "sidedock"
  | "connected-apps"
  | "script-items"
  | "script-gallery"
  | "advanced"
  | "developer"
  | "about"

export interface Page {
  id: PageId
  label: string
  /** The icon inside the coloured block, chosen to say what the page is about. */
  icon: typeof Settings01Icon
  /** The block's colour (the design: one colour per page, like Apple's settings). */
  color: string
  /** Pages with nothing to set yet say which release brings them. */
  comingIn?: string
}

/** The sidebar, in the order and groups of docs/design/settings-window.png. */
export const SIDEBAR: { title?: string; pages: Page[] }[] = [
  { pages: [{ id: "general", label: "General", icon: Settings01Icon, color: "#3b5bff" }] },
  {
    pages: [
      { id: "menu-bar", label: "Menu bar", icon: PanelTopIcon, color: "#2fb344" },
      { id: "dock", label: "Dock", icon: DockIcon, color: "#e5484d", comingIn: "a later release" },
      { id: "sidedock", label: "Sidedock", icon: SidebarLeftIcon, color: "#f08a24", comingIn: "a later release" },
    ],
  },
  {
    title: "Apps & Scripts",
    pages: [
      { id: "connected-apps", label: "Connected Apps", icon: PlugSocketIcon, color: "#e5484d" },
      { id: "script-items", label: "Script Items", icon: SourceCodeIcon, color: "#b44de0", comingIn: "v0.9" },
      { id: "script-gallery", label: "Script Gallery", icon: GridViewIcon, color: "#16a5b8", comingIn: "v1.1" },
    ],
  },
  {
    title: "Advanced",
    pages: [
      { id: "advanced", label: "Advanced", icon: SlidersHorizontalIcon, color: "#e5484d" },
      { id: "developer", label: "Developer", icon: ComputerTerminal01Icon, color: "#b44de0", comingIn: "v1.0" },
    ],
  },
  { pages: [{ id: "about", label: "About", icon: InformationCircleIcon, color: "#2fb344" }] },
]

export const PAGES: Page[] = SIDEBAR.flatMap((group) => group.pages)
