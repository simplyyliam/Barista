import { useRef, useState, type KeyboardEvent, type ReactNode } from "react"
import { arrayMove } from "@dnd-kit/sortable"
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
import { PreviewFrame } from "./preview-frame"
import { SortableList, type SortableState } from "./sortable"
import { ActionButton, Row, Segmented } from "./ui"

const nameOf = (id: string) => (isBuiltIn(id) ? BUILT_IN_NAMES[id] : id)

/** What an item looks like in the preview: the real icons and text, with the saved clock format. */
export function Glyph({ id, pref, settings }: { id: BuiltInId; pref: BarItemPref_Serialize; settings: Settings }) {
  switch (id) {
    case "brand":
      return <BrandLogo />
    case "app-name":
      return <span className="font-bar-strong">Barista</span>
    case "app-menus":
      return <span>File&nbsp;&nbsp;&nbsp;Edit&nbsp;&nbsp;&nbsp;View</span>
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

export const pillClass = (pref: BarItemPref_Serialize) => {
  const iconOnly = pref.id !== "app-name" && pref.id !== "app-menus" && pref.id !== "clock" && !(pref.id === "battery" && pref.display !== "icon")
  return cn(
    "flex h-[calc(var(--bar-h)-4px)] items-center gap-1.5 rounded-full text-bar-fg select-none",
    iconOnly ? "w-7 justify-center" : "px-[9px]",
  )
}

function Pill({
  pref,
  settings,
  selected,
  state,
  onSelect,
  onHide,
}: {
  pref: BarItemPref_Serialize
  settings: Settings
  selected: boolean
  state: SortableState
  onSelect: () => void
  onHide: () => void
}) {
  if (!isBuiltIn(pref.id)) return null
  const { onKeyDown: dragKeys, ...handle } = state.handle as { onKeyDown?: (e: KeyboardEvent) => void }
  return (
    <div ref={state.setNodeRef} style={state.style} className={cn("relative touch-none", state.dragging && "opacity-30")}>
      <button
        type="button"
        aria-pressed={selected}
        aria-label={nameOf(pref.id)}
        {...handle}
        onClick={onSelect}
        onKeyDown={(e) => {
          if (e.key === "Delete" || e.key === "Backspace") {
            e.preventDefault()
            onHide()
          } else dragKeys?.(e)
        }}
        className={cn(
          pillClass(pref),
          "cursor-grab outline-none transition-colors hover:bg-bar-hover focus-visible:ring-2 focus-visible:ring-[#0a64e8] active:cursor-grabbing",
          selected && !state.dragging && "bg-bar-hover ring-2 ring-[#0a64e8]",
          // No ring on what you are dragging, even if it has keyboard focus.
          state.dragging && "ring-0 focus-visible:ring-0",
        )}
      >
        <Glyph id={pref.id} pref={pref} settings={settings} />
      </button>
      {selected && !state.dragging && (
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
 * The menu bar, editable in place (the preview of the design), at the size it really has. Drag an
 * item to move it (the others glide aside), drag it off the bar to hide it, click it to select it and
 * change it below. Hidden items wait in a tray under the picture. Without a mouse: Tab to an item,
 * Space picks it up, arrows move it, Space drops it, Delete hides it.
 */
export function BarEditor({ settings, corner }: { settings: Settings; corner?: ReactNode }) {
  const bar = settings.menuBar
  const [selected, setSelected] = useState<string | null>(null)
  const strip = useRef<HTMLDivElement>(null)

  const prefs = bar.items
  const shown = prefs.filter((p) => p.visible)
  const hidden = prefs.filter((p) => !p.visible)
  const leftIds = shown.filter((p) => zoneOf(p) === "left").map((p) => p.id)
  const rightIds = shown.filter((p) => zoneOf(p) === "right").map((p) => p.id)

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
  const prefOf = (id: string) => prefs.find((p) => p.id === id)!

  const zone = (ids: string[], onReorder: (next: string[]) => void, gap: string) => (
    <SortableList
      ids={ids}
      layout="row"
      onReorder={onReorder}
      onRemove={hide}
      boundsRef={strip}
      name={nameOf}
      className={cn("flex h-[var(--bar-h)] items-center", gap)}
      renderItem={(id, state) => (
        <Pill
          pref={prefOf(id)}
          settings={settings}
          selected={selected === id}
          state={state}
          onSelect={() => setSelected(selected === id ? null : id)}
          onHide={() => hide(id)}
        />
      )}
      renderOverlay={(id) => (
        <div className={cn(pillClass(prefOf(id)), "cursor-grabbing bg-bar-hover shadow-lg")}>
          <Glyph id={prefOf(id).id as BuiltInId} pref={prefOf(id)} settings={settings} />
        </div>
      )}
    />
  )

  const current = selected ? prefs.find((p) => p.id === selected) : undefined
  const currentZone = current ? zoneOf(current) : undefined
  const moveSelected = (by: -1 | 1) => {
    if (!current) return
    const ids = currentZone === "left" ? leftIds : rightIds
    const from = ids.indexOf(current.id)
    const to = from + by
    if (from < 0 || to < 0 || to >= ids.length) return
    const next = arrayMove(ids, from, to)
    if (currentZone === "left") save(next, rightIds)
    else save(leftIds, next)
  }

  return (
    <div className="flex flex-col gap-4">
      <PreviewFrame settings={settings} height={148}>
        <div className="absolute inset-0 grid place-items-center" onClick={(e) => e.target === e.currentTarget && setSelected(null)}>
          <div
            ref={strip}
            style={{ "--bar-h": `${bar.height}px` } as React.CSSProperties}
            data-bar-root
            className="flex w-[calc(100%-24px)] items-center justify-between font-bar text-bar font-bar-item text-bar-fg"
          >
            {zone(leftIds, (next) => save(next, rightIds), "gap-0")}
            {zone(rightIds, (next) => save(leftIds, next), "gap-[2px]")}
          </div>
        </div>
        <p className="absolute right-3.5 bottom-2.5 text-[11px] text-white/80 mix-blend-difference">
          Drag to reorder. Drag off the bar to hide.
        </p>
        {corner}
      </PreviewFrame>

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
        <div className="flex flex-col">
          <Row label={nameOf(current.id)} hint="Selected in the bar">
            <ActionButton onClick={() => moveSelected(-1)}>Move left</ActionButton>
            <ActionButton onClick={() => moveSelected(1)}>Move right</ActionButton>
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
        </div>
      )}
    </div>
  )
}

export function Tray({ label, children }: { label: string; children: ReactNode }) {
  return (
    <div className="flex flex-wrap items-center gap-2">
      <span className="text-[13px] text-muted-foreground">{label}</span>
      {children}
    </div>
  )
}

export function Chip({ children, onClick }: { children: ReactNode; onClick: () => void }) {
  return (
    <button
      type="button"
      onClick={onClick}
      className="flex h-7 items-center gap-1 rounded-full bg-white/[0.08] pr-3 pl-2 text-[13px] text-foreground outline-none hover:bg-white/[0.14] focus-visible:ring-2 focus-visible:ring-ring/40"
    >
      <Icon icon={LucidePlus} size={12} />
      {children}
    </button>
  )
}
