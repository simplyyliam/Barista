import {
  CodeIcon,
  ComputerTerminal01Icon,
  DashboardSquare01Icon,
  DockIcon,
  InformationCircleIcon,
  Link01Icon,
  PanelTopIcon,
  Settings02Icon,
  SidebarLeftIcon,
  SlidersHorizontalIcon,
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
  /** Chosen to say what the page is about. Plain and mono, no coloured block. */
  icon: typeof Settings02Icon
  /** Pages with nothing to set yet say which release brings them. */
  comingIn?: string
}

/** The sidebar, in the order and groups of docs/design/settings-window.png. */
export const SIDEBAR: { title?: string; pages: Page[] }[] = [
  { pages: [{ id: "general", label: "General", icon: Settings02Icon }] },
  {
    pages: [
      { id: "menu-bar", label: "Menu bar", icon: PanelTopIcon },
      { id: "dock", label: "Dock", icon: DockIcon, comingIn: "a later release" },
      { id: "sidedock", label: "Sidedock", icon: SidebarLeftIcon, comingIn: "a later release" },
    ],
  },
  {
    title: "Apps & Scripts",
    pages: [
      { id: "connected-apps", label: "Connected Apps", icon: Link01Icon },
      { id: "script-items", label: "Script Items", icon: CodeIcon },
      { id: "script-gallery", label: "Script Gallery", icon: DashboardSquare01Icon, comingIn: "v1.1" },
    ],
  },
  {
    title: "Advanced",
    pages: [
      { id: "advanced", label: "Advanced", icon: SlidersHorizontalIcon },
      { id: "developer", label: "Developer", icon: ComputerTerminal01Icon, comingIn: "v1.0" },
    ],
  },
  { pages: [{ id: "about", label: "About", icon: InformationCircleIcon }] },
]

export const PAGES: Page[] = SIDEBAR.flatMap((group) => group.pages)
