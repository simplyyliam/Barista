import { useRef, useState, type ReactNode } from "react"
import { cn } from "cn"
import { Icon, LucideMinus, LucidePlus } from "@barista/ui/icons"
import {
  BatteryIcon,
  BrandLogo,
  ControlCenterIcon,
  SearchIcon,
  VolumeIcon,
  WifiIcon,
} from "@/features/menubar/icons"
import { BUILT_IN_NAMES, isBuiltIn, zoneOf, type BuiltInId } from "@/features/menubar/built-in"
import { formatClock } from "@/features/menubar/lib/format-clock"
import type { BarItemPref_Serialize } from "@/shared/bindings"
import type { Settings } from "@/shared/stores/settings"
import { actions } from "./actions"
import { useReorder } from "./hooks/use-reorder"
import { PreviewFrame } from "./preview-frame"
import { ActionButton, Row, Segmented } from "./ui"

/** The bar in the preview is drawn this much bigger than life, so the pills are easy to grab. */
const ZOOM = 1.35

const nameOf = (id: string) => (isBuiltIn(id) ? BUILT_IN_NAMES[id] : id)

/** What an item looks like in the preview: the real icons and text, with the saved clock format. */
function Glyph({ id, pref, settings }: { id: BuiltInId; pref: BarItemPref_Serialize; settings: Settings }) {
  switch (id) {
    case "brand":
      return <BrandLogo />
    case "app-name":
      return <span className="font-bar-strong">Barista</span>
    case "battery":
      return (
        <>
          {pref.display !== "icon" && <span>82%</span>}
          <BatteryIcon percent={82} charging={false} />
        </>
      )
    case "wifi":
      return <WifiIcon />
    case "volume":
      return <VolumeIcon />
    case "search":
      return <SearchIcon />
    case "control-center":
      return <ControlCenterIcon />
    case "clock": {
      const { date, time } = formatClock(new Date(), false, settings.menuBar.clock)
      const center = settings.menuBar.clock.datePosition === "center"
      return (
        <span className="tabular-nums">
          {!center && date && `${date}  `}
          {time}
        </span>
      )
    }
  }
}

function Pill({
  pref,
  settings,
  selected,
  dragging,
  onSelect,
  onHide,
  handlers,
}: {
  pref: BarItemPref_Serialize
  settings: Settings
  selected: boolean
  dragging: boolean
  onSelect: () => void
  onHide: () => void
  handlers: ReturnType<ReturnType<typeof useReorder>["handlers"]>
}) {
  if (!isBuiltIn(pref.id)) return null
  const iconOnly = pref.id !== "app-name" && pref.id !== "clock" && !(pref.id === "battery" && pref.display !== "icon")
  return (
    <div className="group/pill relative">
      <button
        type="button"
        aria-pressed={selected}
        aria-label={`${nameOf(pref.id)}. Alt plus arrow keys moves it, Delete hides it.`}
        {...handlers}
        onClick={onSelect}
        className={cn(
          "flex h-[calc(var(--bar-h)-4px)] cursor-grab touch-none items-center gap-1.5 rounded-full text-bar-fg outline-none transition-[background-color,box-shadow,opacity] select-none",
          iconOnly ? "w-7 justify-center" : "px-[9px]",
          "hover:bg-bar-hover focus-visible:ring-2 focus-visible:ring-[#0a64e8]",
          selected && "bg-bar-hover ring-2 ring-[#0a64e8]",
          dragging && "z-10 cursor-grabbing opacity-70 shadow-lg",
        )}
      >
        <Glyph id={pref.id} pref={pref} settings={settings} />
      </button>
      {selected && (
        <button
          type="button"
          aria-label={`Hide ${nameOf(pref.id)}`}
          onClick={onHide}
          className="absolute -top-1.5 -left-1.5 z-20 grid size-4 place-items-center rounded-full bg-neutral-900 text-white ring-1 ring-white/40 outline-none focus-visible:ring-2 focus-visible:ring-[#0a64e8]"
        >
          <Icon icon={LucideMinus} size={10} />
        </button>
      )}
    </div>
  )
}

/**
 * The menu bar, editable in place (the preview of the design). Press and drag an item to move it,
 * drag it off the bar to hide it, click it to select it and change it below. Hidden items wait in a
 * tray under the picture. Without a mouse: focus an item, Alt+Left / Alt+Right move it, Delete hides it.
 */
export function BarEditor({ settings }: { settings: Settings }) {
  const bar = settings.menuBar
  const [selected, setSelected] = useState<string | null>(null)
  const strip = useRef<HTMLDivElement>(null)

  const prefs = bar.items
  const shown = prefs.filter((p) => p.visible)
  const hidden = prefs.filter((p) => !p.visible)
  const left = shown.filter((p) => zoneOf(p) === "left")
  const right = shown.filter((p) => zoneOf(p) === "right")

  const save = (visibleLeft: string[], visibleRight: string[]) => {
    const byId = new Map(prefs.map((p) => [p.id, p]))
    const pick = (ids: string[]) => ids.flatMap((id) => (byId.get(id) ? [byId.get(id)!] : []))
    actions.items([...pick(visibleLeft), ...pick(visibleRight), ...hidden])
  }
  const setVisible = (id: string, visible: boolean) =>
    actions.items(prefs.map((p) => (p.id === id ? { ...p, visible } : p)))
  const hide = (id: string) => {
    setVisible(id, false)
    setSelected(null)
  }

  const leftIds = left.map((p) => p.id)
  const rightIds = right.map((p) => p.id)
  const leftOrder = useReorder({
    ids: leftIds,
    onReorder: (next) => save(next, rightIds),
    onRemove: hide,
    boundsRef: strip,
    name: nameOf,
  })
  const rightOrder = useReorder({
    ids: rightIds,
    onReorder: (next) => save(leftIds, next),
    onRemove: hide,
    boundsRef: strip,
    name: nameOf,
  })

  const prefOf = (id: string) => prefs.find((p) => p.id === id)!
  const zone = (order: typeof leftOrder, align: "start" | "end") => (
    <div className={cn("flex items-center", align === "end" ? "gap-[2px]" : "gap-0")}>
      {order.shown.map((id) => (
        <Pill
          key={id}
          pref={prefOf(id)}
          settings={settings}
          selected={selected === id}
          dragging={order.dragging === id}
          onSelect={() => setSelected(selected === id ? null : id)}
          onHide={() => hide(id)}
          handlers={order.handlers(id)}
        />
      ))}
    </div>
  )

  const current = selected ? prefs.find((p) => p.id === selected) : undefined
  const selectedZone = current ? zoneOf(current) : undefined
  const selectedOrder = selectedZone === "left" ? leftOrder : rightOrder

  return (
    <div className="flex flex-col gap-3">
      <PreviewFrame settings={settings} height={224}>
        <div className="absolute inset-0 grid place-items-center" onClick={(e) => e.target === e.currentTarget && setSelected(null)}>
          <div
            ref={strip}
            style={{ "--bar-h": `${bar.height}px`, zoom: ZOOM } as React.CSSProperties}
            data-bar-root
            className="flex w-[calc((100%-24px)/var(--z,1))] items-center justify-between font-bar text-bar font-bar-item text-bar-fg"
          >
            <div className="flex h-[var(--bar-h)] items-center">{zone(leftOrder, "start")}</div>
            <div className="flex h-[var(--bar-h)] items-center">{zone(rightOrder, "end")}</div>
          </div>
        </div>
        <p className="absolute bottom-3 left-4 rounded-full bg-black/45 px-2.5 py-1 text-[11px] text-white/85 backdrop-blur-sm">
          Drag to reorder. Drag off the bar to hide. Tray and app items sit between, in the order of the bar.
        </p>
      </PreviewFrame>

      <div aria-live="polite" className="sr-only">
        {leftOrder.announcement || rightOrder.announcement}
      </div>

      {hidden.length > 0 && (
        <Tray label="Not in the bar">
          {hidden.map((p) => (
            <Chip key={p.id} onClick={() => setVisible(p.id, true)}>
              {nameOf(p.id)}
            </Chip>
          ))}
        </Tray>
      )}

      {current && isBuiltIn(current.id) && (
        <div className="divide-y divide-border/60 rounded-xl bg-card/60 ring-1 ring-border/60">
          <Row label={nameOf(current.id)} hint="Selected in the bar">
            <ActionButton onClick={() => selectedOrder.move(current.id, -1)}>Move left</ActionButton>
            <ActionButton onClick={() => selectedOrder.move(current.id, 1)}>Move right</ActionButton>
            <ActionButton tone="danger" onClick={() => hide(current.id)}>
              Hide
            </ActionButton>
          </Row>
          {current.id === "battery" && (
            <Row label="Show" hint="The percentage next to the icon">
              <Segmented
                label="Battery display"
                value={current.display === "icon" ? "icon" : "both"}
                options={[
                  { value: "icon", label: "Icon" },
                  { value: "both", label: "Icon and percent" },
                ]}
                onChange={(display) =>
                  actions.items(prefs.map((p) => (p.id === current.id ? { ...p, display } : p)))
                }
              />
            </Row>
          )}
          {current.id === "clock" && (
            <Row label="Format" hint="Hour format, seconds and date are under Clock and date">
              <span />
            </Row>
          )}
        </div>
      )}
    </div>
  )
}

export function Tray({ label, children }: { label: string; children: ReactNode }) {
  return (
    <div className="flex flex-wrap items-center gap-2 rounded-xl bg-card/40 px-3 py-2 ring-1 ring-border/50">
      <span className="text-xs text-muted-foreground">{label}</span>
      {children}
    </div>
  )
}

export function Chip({ children, onClick }: { children: ReactNode; onClick: () => void }) {
  return (
    <button
      type="button"
      onClick={onClick}
      className="flex h-6 items-center gap-1 rounded-full bg-muted pr-2.5 pl-2 text-xs text-foreground outline-none hover:bg-muted/70 focus-visible:ring-2 focus-visible:ring-ring/40"
    >
      <Icon icon={LucidePlus} size={11} />
      {children}
    </button>
  )
}
