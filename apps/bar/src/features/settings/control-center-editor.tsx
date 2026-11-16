import { useRef, useState, type ComponentProps, type KeyboardEvent, type ReactNode } from "react"
import { cn } from "cn"
import {
  AirplaneModeIcon,
  BatteryEcoChargingIcon,
  BluetoothIcon,
  HotspotIcon,
  Icon,
  LucideMinus,
  MusicNote01Icon,
  Moon02Icon,
  Sun03Icon,
  UserIcon,
  VolumeHighIcon,
  Wifi01Icon,
} from "@barista/ui/icons"
import { ACTIONS } from "@/features/panel/control-center/quick-actions"
import type { CcTile_Serialize } from "@/shared/bindings"
import type { Settings } from "@/shared/stores/settings"
import { actions } from "./actions"
import { Chip, Tray } from "./bar-editor"
import { PreviewFrame } from "./preview-frame"
import { SortableList, type SortableState } from "./sortable"
import { ActionButton, Row, Segmented } from "./ui"

type IconType = ComponentProps<typeof Icon>["icon"]

const TILES: Record<string, { label: string; icon: IconType; sub?: string }> = {
  wifi: { label: "Wi-Fi", icon: Wifi01Icon, sub: "Home" },
  airplane: { label: "Airplane mode", icon: AirplaneModeIcon },
  "night-light": { label: "Night light", icon: Moon02Icon },
  bluetooth: { label: "Bluetooth", icon: BluetoothIcon },
  hotspot: { label: "Mobile hotspot", icon: HotspotIcon },
  "battery-saver": { label: "Battery Saver", icon: BatteryEcoChargingIcon },
}

type ModuleKey = "userCard" | "brightness" | "volume" | "nowPlaying"
const MODULES: { key: ModuleKey; label: string }[] = [
  { key: "userCard", label: "User card" },
  { key: "brightness", label: "Brightness slider" },
  { key: "volume", label: "Volume slider" },
  { key: "nowPlaying", label: "Now playing" },
]

type Selection = { kind: "tile" | "module" | "action"; id: string } | null

const tileName = (id: string) => TILES[id]?.label ?? id
const actionName = (id: string) => ACTIONS.find((a) => a.id === id)?.label ?? id

/** The "remove" badge on the selected thing, like the minus on an iOS home screen in edit mode. */
function Minus({ label, onClick }: { label: string; onClick: () => void }) {
  return (
    <button
      type="button"
      aria-label={label}
      onClick={onClick}
      className="absolute -top-1.5 -left-1.5 z-20 grid size-4 place-items-center rounded-full bg-neutral-900 text-white ring-1 ring-white/40 outline-none focus-visible:ring-2 focus-visible:ring-[#0a64e8]"
    >
      <Icon icon={LucideMinus} size={10} />
    </button>
  )
}

/** A thing in the preview that can be selected (a module), with its badge. */
function Selectable({
  selected,
  onSelect,
  onRemove,
  label,
  className,
  children,
}: {
  selected: boolean
  onSelect: () => void
  onRemove: () => void
  label: string
  className?: string
  children: ReactNode
}) {
  return (
    <div className={cn("relative", className)}>
      <div
        role="button"
        tabIndex={0}
        aria-pressed={selected}
        aria-label={label}
        onClick={onSelect}
        onKeyDown={(e) => {
          if (e.key === "Enter" || e.key === " ") {
            e.preventDefault()
            onSelect()
          } else if (e.key === "Delete" || e.key === "Backspace") {
            e.preventDefault()
            onRemove()
          }
        }}
        className={cn(
          "rounded-panel-child outline-none focus-visible:ring-2 focus-visible:ring-[#0a64e8]",
          selected && "ring-2 ring-[#0a64e8]",
        )}
      >
        {children}
      </div>
      {selected && <Minus label={`Remove ${label}`} onClick={onRemove} />}
    </div>
  )
}

/**
 * The control center, editable in place (the second preview of the design). Drag tiles and quick
 * actions to reorder them, click one to resize or remove it, and add what is missing from the tray
 * under the picture. Keyboard: focus a tile, Alt+Left / Alt+Right move it, Delete removes it.
 */
export function ControlCenterEditor({ settings }: { settings: Settings }) {
  const prefs = settings.controlCenter
  const [selected, setSelected] = useState<Selection>(null)
  const tileBox = useRef<HTMLDivElement>(null)
  const actionBox = useRef<HTMLDivElement>(null)

  const visibleTiles = prefs.tiles.filter((t) => t.visible)
  const hiddenTiles = prefs.tiles.filter((t) => !t.visible)
  const setTiles = (tiles: CcTile_Serialize[]) => actions.controlCenter(settings, { tiles })
  const patchTile = (id: string, patch: Partial<CcTile_Serialize>) =>
    setTiles(prefs.tiles.map((t) => (t.id === id ? { ...t, ...patch } : t)))
  const removeTile = (id: string) => {
    patchTile(id, { visible: false })
    setSelected(null)
  }

  const removeAction = (id: string) => {
    actions.controlCenter(settings, { quickActions: prefs.quickActions.filter((a) => a !== id) })
    setSelected(null)
  }

  const setModule = (key: ModuleKey, on: boolean) => {
    actions.controlCenter(settings, { [key]: on })
    if (!on) setSelected(null)
  }

  const is = (kind: NonNullable<Selection>["kind"], id: string) => selected?.kind === kind && selected.id === id
  const tileOf = (id: string) => visibleTiles.find((t) => t.id === id)!

  const hiddenModules = MODULES.filter((m) => !prefs[m.key])
  const missingActions = ACTIONS.filter((a) => !prefs.quickActions.includes(a.id))
  const nothingHidden = hiddenTiles.length + hiddenModules.length + missingActions.length === 0

  const selectedTile = selected?.kind === "tile" ? prefs.tiles.find((t) => t.id === selected.id) : undefined

  return (
    <div className="flex flex-col gap-3">
      <PreviewFrame settings={settings} height={560} className="grid place-items-center">
        <div
          className="flex w-panel-w flex-col gap-2"
          onClick={(e) => e.target === e.currentTarget && setSelected(null)}
        >
          <section className="flex flex-col gap-cc-gap rounded-panel bg-cc-card p-cc-pad text-cc-fg">
            {prefs.userCard && (
              <Selectable
                label="User card"
                selected={is("module", "userCard")}
                onSelect={() => setSelected({ kind: "module", id: "userCard" })}
                onRemove={() => setModule("userCard", false)}
              >
                <div className="flex h-12 items-center gap-2.5 rounded-panel-child bg-cc-tile px-2.5">
                  <span className="grid size-8 place-items-center rounded-full bg-cc-active">
                    <Icon icon={UserIcon} size={16} />
                  </span>
                  <span className="flex flex-col leading-tight">
                    <span className="text-[11px] font-medium">Your name</span>
                    <span className="text-[10px] text-cc-fg-muted">you@example.com</span>
                  </span>
                </div>
              </Selectable>
            )}

            <div ref={tileBox}>
              <SortableList
                ids={visibleTiles.map((t) => t.id)}
                layout="grid"
                className="grid grid-cols-4 content-start gap-cc-gap"
                boundsRef={tileBox}
                name={tileName}
                onRemove={removeTile}
                onReorder={(next) => {
                  const byId = new Map(prefs.tiles.map((t) => [t.id, t]))
                  setTiles([...next.flatMap((id) => (byId.get(id) ? [byId.get(id)!] : [])), ...hiddenTiles])
                }}
                renderItem={(id, state) => (
                  <Draggable
                    state={state}
                    className={cn(tileOf(id).size === "wide" && "col-span-2")}
                    selected={is("tile", id)}
                    label={tileName(id)}
                    onSelect={() => setSelected({ kind: "tile", id })}
                    onRemove={() => removeTile(id)}
                  >
                    <TileFace id={id} wide={tileOf(id).size === "wide"} selected={is("tile", id) && !state.dragging} />
                  </Draggable>
                )}
                renderOverlay={(id) => <TileFace id={id} wide={tileOf(id).size === "wide"} selected={false} overlay />}
              />
            </div>

            {prefs.brightness && (
              <Selectable
                label="Brightness slider"
                selected={is("module", "brightness")}
                onSelect={() => setSelected({ kind: "module", id: "brightness" })}
                onRemove={() => setModule("brightness", false)}
              >
                <FakeSlider icon={Sun03Icon} fill={62} />
              </Selectable>
            )}
            {prefs.volume && (
              <Selectable
                label="Volume slider"
                selected={is("module", "volume")}
                onSelect={() => setSelected({ kind: "module", id: "volume" })}
                onRemove={() => setModule("volume", false)}
              >
                <FakeSlider icon={VolumeHighIcon} fill={40} />
              </Selectable>
            )}
            {prefs.nowPlaying && (
              <Selectable
                label="Now playing"
                selected={is("module", "nowPlaying")}
                onSelect={() => setSelected({ kind: "module", id: "nowPlaying" })}
                onRemove={() => setModule("nowPlaying", false)}
              >
                <div className="flex h-12 items-center gap-2.5 rounded-panel-child bg-cc-tile px-2.5">
                  <span className="grid size-8 place-items-center rounded-md bg-cc-active">
                    <Icon icon={MusicNote01Icon} size={16} />
                  </span>
                  <span className="flex flex-col leading-tight">
                    <span className="text-[11px] font-medium">Not playing</span>
                    <span className="text-[10px] text-cc-fg-muted">Music, video, podcasts</span>
                  </span>
                </div>
              </Selectable>
            )}
          </section>

          {prefs.quickActions.length > 0 && (
            <section ref={actionBox} className="rounded-panel bg-cc-card p-cc-pad">
              <SortableList
                ids={prefs.quickActions}
                layout="grid"
                className="grid grid-cols-4 gap-cc-gap"
                boundsRef={actionBox}
                name={actionName}
                onRemove={removeAction}
                onReorder={(next) => actions.controlCenter(settings, { quickActions: next })}
                renderItem={(id, state) => (
                  <Draggable
                    state={state}
                    selected={is("action", id)}
                    label={actionName(id)}
                    onSelect={() => setSelected({ kind: "action", id })}
                    onRemove={() => removeAction(id)}
                  >
                    <ActionFace id={id} selected={is("action", id) && !state.dragging} />
                  </Draggable>
                )}
                renderOverlay={(id) => <ActionFace id={id} selected={false} overlay />}
              />
            </section>
          )}
        </div>
      </PreviewFrame>

      {!nothingHidden && (
        <Tray label="Add to the control center">
          {hiddenTiles.map((t) => (
            <Chip key={t.id} onClick={() => patchTile(t.id, { visible: true })}>
              {tileName(t.id)}
            </Chip>
          ))}
          {hiddenModules.map((m) => (
            <Chip key={m.key} onClick={() => setModule(m.key, true)}>
              {m.label}
            </Chip>
          ))}
          {missingActions.map((a) => (
            <Chip
              key={a.id}
              onClick={() =>
                prefs.quickActions.length < 8 &&
                actions.controlCenter(settings, { quickActions: [...prefs.quickActions, a.id] })
              }
            >
              {a.label}
            </Chip>
          ))}
        </Tray>
      )}

      {selectedTile && (
        <div className="flex flex-col border-y border-white/[0.07]">
          <Row label={tileName(selectedTile.id)} hint="Selected tile">
            <Segmented
              label="Tile size"
              value={selectedTile.size}
              options={[
                { value: "small", label: "Small" },
                { value: "wide", label: "Wide" },
              ]}
              onChange={(size) => patchTile(selectedTile.id, { size })}
            />
            <ActionButton tone="danger" onClick={() => removeTile(selectedTile.id)}>
              Remove
            </ActionButton>
          </Row>
        </div>
      )}
    </div>
  )
}

/** A draggable thing in the preview: the item itself, plus its remove badge while selected. */
function Draggable({
  state,
  className,
  selected,
  label,
  onSelect,
  onRemove,
  children,
}: {
  state: SortableState
  className?: string
  selected: boolean
  label: string
  onSelect: () => void
  onRemove: () => void
  children: ReactNode
}) {
  const { onKeyDown: dragKeys, ...handle } = state.handle as { onKeyDown?: (e: KeyboardEvent) => void }
  return (
    <div ref={state.setNodeRef} style={state.style} className={cn("relative touch-none", state.dragging && "opacity-30", className)}>
      <div
        {...handle}
        role="button"
        aria-pressed={selected}
        aria-label={label}
        onClick={onSelect}
        onKeyDown={(e) => {
          if (e.key === "Enter") onSelect()
          else if (e.key === "Delete" || e.key === "Backspace") {
            e.preventDefault()
            onRemove()
          } else dragKeys?.(e)
        }}
        className={cn(
          "cursor-grab rounded-panel-child outline-none focus-visible:ring-2 focus-visible:ring-[#0a64e8] active:cursor-grabbing",
          // No ring on what you are dragging, even if it has keyboard focus.
          state.dragging && "focus-visible:ring-0",
        )}
      >
        {children}
      </div>
      {selected && !state.dragging && <Minus label={`Remove ${label}`} onClick={onRemove} />}
    </div>
  )
}

function TileFace({ id, wide, selected, overlay }: { id: string; wide: boolean; selected: boolean; overlay?: boolean }) {
  const def = TILES[id]
  if (!def) return null
  return (
    <div
      className={cn(
        "flex h-cc-tile items-center rounded-panel-child bg-cc-tile text-cc-fg select-none",
        wide ? "justify-start gap-2.5 px-cc-label-pad text-left" : "justify-center",
        selected && "ring-2 ring-[#0a64e8]",
        overlay && "cursor-grabbing bg-neutral-700/90 shadow-xl",
      )}
      style={overlay ? { width: wide ? 140 : 66 } : undefined}
    >
      <Icon icon={def.icon} size={wide ? 20 : 24} className="shrink-0" />
      {wide && (
        <span className="flex min-w-0 flex-col leading-tight">
          <span className="truncate text-[11px] font-medium">{def.label}</span>
          {def.sub && <span className="truncate text-[10px] text-cc-fg-muted">{def.sub}</span>}
        </span>
      )}
    </div>
  )
}

function ActionFace({ id, selected, overlay }: { id: string; selected: boolean; overlay?: boolean }) {
  const action = ACTIONS.find((a) => a.id === id)
  if (!action) return null
  return (
    <div
      className={cn(
        "flex h-cc-tile items-center justify-center rounded-panel-child bg-cc-tile text-cc-fg select-none",
        selected && "ring-2 ring-[#0a64e8]",
        overlay && "w-[66px] cursor-grabbing bg-neutral-700/90 shadow-xl",
      )}
    >
      <Icon icon={action.icon} size={24} />
    </div>
  )
}

function FakeSlider({ icon, fill }: { icon: IconType; fill: number }) {
  return (
    <div className="relative h-8 overflow-hidden rounded-full bg-cc-tile">
      <div className="h-full rounded-full bg-white" style={{ width: `${fill}%` }} />
      <span className="absolute inset-y-0 right-2.5 grid place-items-center text-neutral-800">
        <Icon icon={icon} size={14} />
      </span>
    </div>
  )
}
