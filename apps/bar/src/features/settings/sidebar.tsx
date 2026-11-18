import { Icon } from "@barista/ui/icons"
import { cn } from "cn"
import { PAGES, SIDEBAR, type Page, type PageId } from "./pages"
import { ScrollFade } from "./scroll-fade"

/** The page's icon on its own: mono, no coloured block. */
export function PageIcon({ page, size = 16 }: { page: Page; size?: number }) {
  return <Icon icon={page.icon} size={size} strokeWidth={1.5} className="shrink-0" />
}

export function Sidebar({ current, onSelect }: { current: PageId; onSelect: (id: PageId) => void }) {
  return (
    <nav aria-label="Settings" className="flex w-[196px] shrink-0 flex-col">
      <ScrollFade className="flex-1" innerClassName="px-3 pt-2 pb-6" fade="h-6">
        <div className="flex flex-col gap-4">
          {SIDEBAR.map((group, index) => (
            <div key={group.title ?? index} className="flex flex-col gap-px">
              {group.title && <div className="px-2.5 pb-1 text-[11px] text-muted-foreground/70">{group.title}</div>}
              {group.pages.map((page) => (
                <button
                  key={page.id}
                  type="button"
                  aria-current={page.id === current ? "page" : undefined}
                  onClick={() => onSelect(page.id)}
                  className={cn(
                    "flex h-8 items-center gap-2.5 rounded-lg px-2.5 text-left text-[13px] font-medium outline-none transition-colors",
                    "focus-visible:ring-2 focus-visible:ring-ring/40",
                    page.id === current
                      ? "bg-white/[0.08] text-foreground"
                      : "text-foreground/60 hover:bg-white/[0.04] hover:text-foreground/90",
                  )}
                >
                  <PageIcon page={page} />
                  {page.label}
                </button>
              ))}
            </div>
          ))}
        </div>
      </ScrollFade>
    </nav>
  )
}

export const pageOf = (id: PageId): Page => PAGES.find((p) => p.id === id) ?? PAGES[0]!
