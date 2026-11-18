import { useEffect, useState } from "react"
import { getCurrentWindow } from "@tauri-apps/api/window"
import { Button } from "@barista/ui/components/ui/button"
import { Icon, LucideX } from "@barista/ui/icons"
import { cn } from "cn"

/**
 * The settings window's own titlebar (the window has no frame, so the page can be black all the way
 * to the edge). The empty part drags the window, a double click maximizes it, and the three buttons
 * are the usual minimize / maximize / close. The permissions are in `capabilities/settings.json`.
 */
export function TitleBar() {
  const [maximized, setMaximized] = useState(false)

  useEffect(() => {
    const win = getCurrentWindow()
    let stop: (() => void) | undefined
    const sync = () => void win.isMaximized().then(setMaximized)
    sync()
    void win.onResized(sync).then((unlisten) => (stop = unlisten))
    return () => stop?.()
  }, [])

  const win = getCurrentWindow()
  const control = "h-full w-12 rounded-none text-foreground/80 hover:bg-white/10 hover:text-foreground"
  return (
    <header
      data-tauri-drag-region
      onDoubleClick={() => void win.toggleMaximize()}
      className="flex h-8 shrink-0 items-center justify-between bg-black pl-4 select-none"
    >
      <span data-tauri-drag-region className="text-xs text-muted-foreground/70">
        Barista Settings
      </span>
      <div className="flex h-full">
        <Button variant="ghost" aria-label="Minimize" className={control} onClick={() => void win.minimize()}>
          <svg width="10" height="10" viewBox="0 0 10 10" aria-hidden="true">
            <path d="M0 5h10" stroke="currentColor" strokeWidth="1" />
          </svg>
        </Button>
        <Button
          variant="ghost"
          aria-label={maximized ? "Restore" : "Maximize"}
          className={control}
          onClick={() => void win.toggleMaximize()}
        >
          <svg width="10" height="10" viewBox="0 0 10 10" aria-hidden="true" fill="none" stroke="currentColor">
            {maximized ? (
              <>
                <rect x="0.5" y="2.5" width="7" height="7" />
                <path d="M2.5 2.5v-2h7v7h-2" />
              </>
            ) : (
              <rect x="0.5" y="0.5" width="9" height="9" />
            )}
          </svg>
        </Button>
        <Button
          variant="ghost"
          aria-label="Close"
          className={cn(control, "hover:bg-[#e81123] hover:text-white")}
          onClick={() => void win.close()}
        >
          <Icon icon={LucideX} size={14} />
        </Button>
      </div>
    </header>
  )
}
