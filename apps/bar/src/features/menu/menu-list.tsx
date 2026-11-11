import { Icon, LucideCheck, LucideChevronRight } from "@barista/ui/icons"
import type { MouseEvent, ReactNode } from "react"
import type { MenuEntry } from "@/shared/bindings"
import { formatAccelerator } from "./lib/accelerator"

type MenuListProps = {
  entries: MenuEntry[]
  /** Row painted as highlighted (-1: none). Mouse and keyboard both write this one value. */
  highlighted: number
  /** Row whose submenu is open; it stays lit while the pointer is in the submenu. */
  expanded?: number | null
  idPrefix: string
  onHover: (index: number, event: MouseEvent) => void
  onLeave: () => void
  onPick: (index: number) => void
}

function Indicator({ entry }: { entry: MenuEntry }): ReactNode {
  if (entry.kind === "checkbox") {
    return entry.checked ? <LucideCheck size={12} strokeWidth={2.75} aria-hidden /> : null
  }
  if (entry.kind === "radio") {
    return entry.checked ? <span aria-hidden className="size-[5px] rounded-full bg-current" /> : null
  }
  if (entry.icon) return <Icon icon={entry.icon} size={16} />
  return null
}

/** Rows and separators of one panel. Plain divs: focus stays on the panel (aria-activedescendant). */
export function MenuList({ entries, highlighted, expanded = null, idPrefix, onHover, onLeave, onPick }: MenuListProps) {
  // macOS only keeps a column for check marks when the menu has some.
  const hasIndicator = entries.some(
    (entry) => entry.kind === "checkbox" || entry.kind === "radio" || Boolean(entry.icon),
  )

  return (
    <div onMouseLeave={onLeave}>
      {entries.map((entry, index) => {
        if (entry.kind === "separator") {
          return (
            <div
              key={`separator-${index}`}
              role="separator"
              className="mx-[10px] my-[5px] h-px bg-menu-separator"
            />
          )
        }

        const lit = index === highlighted || index === expanded
        const submenu = entry.kind === "submenu"
        const role =
          entry.kind === "checkbox" ? "menuitemcheckbox" : entry.kind === "radio" ? "menuitemradio" : "menuitem"

        return (
          <div
            key={entry.id ?? `row-${index}`}
            id={`${idPrefix}-${index}`}
            data-row={index}
            role={role}
            aria-checked={entry.kind === "checkbox" || entry.kind === "radio" ? entry.checked : undefined}
            aria-disabled={!entry.enabled}
            aria-haspopup={submenu ? "menu" : undefined}
            aria-expanded={submenu ? expanded === index : undefined}
            data-highlighted={lit && entry.enabled}
            data-disabled={!entry.enabled}
            onMouseMove={(event) => onHover(index, event)}
            onClick={() => onPick(index)}
            // No transition on purpose: the highlight must snap, like a native menu.
            className="group/row flex h-menu-row items-center rounded-menu-row px-[10px] data-[disabled=true]:text-menu-fg-disabled data-[highlighted=true]:bg-menu-highlight data-[highlighted=true]:text-menu-highlight-fg"
          >
            {hasIndicator && (
              <span className="mr-[5px] -ml-[2px] grid w-4 shrink-0 place-items-center">
                <Indicator entry={entry} />
              </span>
            )}
            <span className="min-w-0 flex-1 truncate whitespace-nowrap">{entry.label}</span>
            {entry.accelerator && (
              <span className="ml-6 shrink-0 whitespace-nowrap text-menu-fg-muted tabular-nums group-data-[disabled=true]/row:text-menu-fg-disabled group-data-[highlighted=true]/row:text-menu-highlight-fg/75">
                {formatAccelerator(entry.accelerator)}
              </span>
            )}
            {submenu && (
              <LucideChevronRight
                size={13}
                strokeWidth={2.25}
                aria-hidden
                className="-mr-[3px] ml-4 shrink-0 text-menu-fg-muted group-data-[disabled=true]/row:text-menu-fg-disabled group-data-[highlighted=true]/row:text-menu-highlight-fg"
              />
            )}
          </div>
        )
      })}
    </div>
  )
}
