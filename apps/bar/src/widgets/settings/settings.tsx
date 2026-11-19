import { useEffect, useState } from "react"
import { ScrollFade } from "@/features/settings/scroll-fade"
import { AboutPage } from "@/features/settings/pages/about"
import { AdvancedPage, ComingSoonPage, ConnectedAppsPage } from "@/features/settings/pages/other"
import { GeneralPage } from "@/features/settings/pages/general"
import { MenuBarPage } from "@/features/settings/pages/menu-bar"
import { ScriptsPage } from "@/features/settings/pages/scripts"
import { pageOf, Sidebar } from "@/features/settings/sidebar"
import { TitleBar } from "@/features/settings/titlebar"
import type { PageId } from "@/features/settings/pages"
import { useSettings } from "@/shared/stores/settings"

/**
 * The settings window (label `settings`, design: docs/design/settings-window.png): a titlebar of its
 * own on top, the sidebar on the left, the page on the right. Settings apply at once (no Save button)
 * and Rust owns them: every control sends a command and the page redraws from `settingsChanged`.
 */
export default function SettingsWindow() {
  const settings = useSettings()
  const [page, setPage] = useState<PageId>("menu-bar")

  // Dropdowns render in a portal on <body>, outside this tree: the dark tokens must be on the root.
  useEffect(() => {
    document.documentElement.classList.add("dark")
  }, [])

  return (
    // The window is opaque black (the bar's transparent page background does not apply here).
    <div className="fixed inset-0 flex flex-col bg-black font-sans text-foreground select-none">
      <TitleBar />
      <div className="flex min-h-0 flex-1">
        <Sidebar current={page} onSelect={setPage} />
        <ScrollFade
          label={pageOf(page).label}
          className="min-w-0 flex-1"
          innerClassName="px-8 pt-2 pb-10"
          fade="h-12"
        >
          <div key={page} className="mx-auto max-w-[600px]">
            {settings ? <Page id={page} settings={settings} /> : null}
          </div>
        </ScrollFade>
      </div>
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
    case "script-items":
      return <ScriptsPage settings={settings} />
    case "advanced":
      return <AdvancedPage settings={settings} />
    case "about":
      return <AboutPage />
    default:
      return <ComingSoonPage id={id} />
  }
}
