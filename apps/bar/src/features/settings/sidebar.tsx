import { Icon } from "@barista/ui/icons"
import { cn } from "cn"
import { PAGES, SIDEBAR, type Page, type PageId } from "./pages"

/** The coloured block with the page's icon in it, like the icons in Apple's System Settings. */
export function PageBadge({ page, size = 24 }: { page: Page; size?: number }) {
  return (
    <span
      aria-hidden="true"
      style={{ backgroundColor: page.color, width: size, height: size }}
      className="grid shrink-0 place-items-center rounded-[7px] text-white"
    >
      <Icon icon={page.icon} size={Math.round(size * 0.62)} strokeWidth={2} />
    </span>
  )
}

export function Sidebar({ current, onSelect }: { current: PageId; onSelect: (id: PageId) => void }) {
  return (
    <nav aria-label="Settings" className="flex w-[220px] shrink-0 flex-col gap-3 overflow-y-auto px-3 py-5">
      {SIDEBAR.map((group, index) => (
        <div key={group.title ?? index} className="flex flex-col gap-0.5">
          {group.title && <div className="px-2 pb-1 text-[11px] text-muted-foreground">{group.title}</div>}
          {group.pages.map((page) => (
            <button
              key={page.id}
              type="button"
              aria-current={page.id === current ? "page" : undefined}
              onClick={() => onSelect(page.id)}
              className={cn(
                "flex items-center gap-2.5 rounded-lg px-2 py-1.5 text-left text-[13px] outline-none transition-colors",
                "focus-visible:ring-2 focus-visible:ring-ring/40",
                page.id === current ? "bg-white/[0.09] text-foreground" : "text-foreground/85 hover:bg-white/[0.05]",
              )}
            >
              <PageBadge page={page} />
              {page.label}
            </button>
          ))}
        </div>
      ))}
    </nav>
  )
}

export const pageOf = (id: PageId): Page => PAGES.find((p) => p.id === id) ?? PAGES[0]!
