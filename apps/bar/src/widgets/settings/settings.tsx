import { useState } from "react"
import { AboutPage, AdvancedPage, ComingSoonPage, ConnectedAppsPage } from "@/features/settings/pages/other"
import { GeneralPage } from "@/features/settings/pages/general"
import { MenuBarPage } from "@/features/settings/pages/menu-bar"
import { pageOf, Sidebar } from "@/features/settings/sidebar"
import type { PageId } from "@/features/settings/pages"
import { useSettings } from "@/shared/stores/settings"

/**
 * The settings window (label `settings`, design: docs/design/settings-window.png): the sidebar on
 * the left, the page on the right. Settings apply at once (no Save button) and Rust owns them: every
 * control sends a command and the page redraws from `settingsChanged`.
 */
export default function SettingsWindow() {
  const settings = useSettings()
  const [page, setPage] = useState<PageId>("menu-bar")

  return (
    // `dark` gives the shadcn tokens their dark values for this window only. The window is opaque
    // (the bar's transparent page background does not apply here).
    <div className="dark fixed inset-0 flex bg-[#0a0a0a] font-sans text-foreground select-none">
      <Sidebar current={page} onSelect={setPage} />
      <main aria-label={pageOf(page).label} className="min-w-0 flex-1 overflow-y-auto px-8 py-7 [scrollbar-width:thin]">
        <div className="mx-auto max-w-[780px]">{settings ? <Page id={page} settings={settings} /> : null}</div>
      </main>
    </div>
  )
}

function Page({ id, settings }: { id: PageId; settings: NonNullable<ReturnType<typeof useSettings>> }) {
  switch (id) {
    case "general":
      return <GeneralPage settings={settings} />
    case "menu-bar":
      return <MenuBarPage settings={settings} />
    case "connected-apps":
      return <ConnectedAppsPage />
    case "advanced":
      return <AdvancedPage settings={settings} />
    case "about":
      return <AboutPage />
    default:
      return <ComingSoonPage id={id} />
  }
}
