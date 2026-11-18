import { useState } from "react"
import { commands } from "@/shared/bindings"
import { AnimatePresence, motion } from "motion/react"
import { ArrowDown01Icon, Icon, ViewIcon, ViewOffIcon } from "@barista/ui/icons"
import { cn } from "cn"
import { EASE_OUT, ITEM_EXIT } from "@/shared/motion"
import type { Settings } from "@/shared/stores/settings"
import { actions } from "../actions"
import { BarEditor } from "../bar-editor"
import { ControlCenterEditor } from "../control-center-editor"
import { Group, PageTitle, SegmentedRow, SelectRow, SliderRow, SwitchRow } from "../ui"

type View = "bar" | "control-center"

const FONTS = ["Segoe UI", "Segoe UI Variable", "Bahnschrift", "Calibri", "Cascadia Code", "Consolas", "Inter"]

/** Where the preview was left (open or closed) is remembered; a blocked store just means closed. */
const OPEN_KEY = "barista.settings.preview"
const readOpen = () => {
  try {
    return localStorage.getItem(OPEN_KEY) === "1"
  } catch {
    return false
  }
}

/** The two pills in the picture's corner (the design): which thing the preview shows and edits. */
function ViewPills({ view, onChange }: { view: View; onChange: (view: View) => void }) {
  const pill = (value: View, label: string) => (
    <button
      type="button"
      role="radio"
      aria-checked={view === value}
      onClick={() => onChange(value)}
      className={cn(
        "rounded-2xl px-[8.25px] py-[6.6px] text-[11px] leading-none outline-none backdrop-blur-sm transition-colors focus-visible:ring-2 focus-visible:ring-white/60",
        view === value ? "bg-white text-black" : "bg-black/35 text-white hover:bg-black/50",
      )}
    >
      {label}
    </button>
  )
  return (
    <div role="radiogroup" aria-label="What to edit" className="absolute bottom-3 left-3 z-10 flex gap-1">
      {pill("bar", "Menu bar")}
      {pill("control-center", "Control center")}
    </div>
  )
}

/** Opens and closes the picture: it is big, and most visits are just for the options below. */
function PreviewToggle({ open, onToggle }: { open: boolean; onToggle: () => void }) {
  return (
    <button
      type="button"
      aria-expanded={open}
      onClick={onToggle}
      className="flex h-8 w-fit items-center gap-1.5 rounded-lg px-2.5 text-xs text-muted-foreground outline-none transition-colors hover:bg-white/[0.04] hover:text-foreground focus-visible:ring-2 focus-visible:ring-ring/40"
    >
      <Icon icon={open ? ViewOffIcon : ViewIcon} size={14} strokeWidth={1.5} />
      {open ? "Hide preview" : "Show preview"}
      <Icon icon={ArrowDown01Icon} size={12} className={cn("transition-transform duration-200", open && "rotate-180")} />
    </button>
  )
}

const DEFAULT_FONT = "default"

function FontRow({ settings }: { settings: Settings }) {
  const saved = settings.menuBar.appearance.font
  // A font typed in an older version (or imported) stays in the list so it is not lost.
  const names = saved && !FONTS.includes(saved) ? [saved, ...FONTS] : FONTS
  return (
    <SelectRow
      label="Font"
      hint="Default is Segoe UI Variable."
      value={saved || DEFAULT_FONT}
      options={[{ value: DEFAULT_FONT, label: "Default" }, ...names.map((f) => ({ value: f, label: f }))]}
      onChange={(font) => actions.appearance(settings, { font: font === DEFAULT_FONT ? "" : font })}
    />
  )
}

export function MenuBarPage({ settings }: { settings: Settings }) {
  const [view, setView] = useState<View>("bar")
  const [open, setOpen] = useState(readOpen)
  const toggle = () =>
    setOpen((was) => {
      try {
        localStorage.setItem(OPEN_KEY, was ? "0" : "1")
      } catch {
        // storage blocked: the choice just isn't remembered
      }
      return !was
    })
  const bar = settings.menuBar
  const look = bar.appearance
  const clock = bar.clock

  return (
    <div className="flex flex-col gap-5">
      <PageTitle
        title="Menu bar"
        description="Everything you change shows up in your bar at once."
      />

      <div className="flex flex-col">
        <PreviewToggle open={open} onToggle={toggle} />
        <AnimatePresence initial={false}>
          {open && (
            <motion.div
              key="preview"
              initial={{ height: 0, opacity: 0 }}
              animate={{ height: "auto", opacity: 1, transition: { duration: 0.28, ease: EASE_OUT } }}
              exit={{ height: 0, opacity: 0, transition: ITEM_EXIT }}
              className="overflow-hidden"
            >
              <div className="pt-2 pb-1">
                {view === "bar" ? (
                  <BarEditor settings={settings} corner={<ViewPills view={view} onChange={setView} />} />
                ) : (
                  <ControlCenterEditor settings={settings} corner={<ViewPills view={view} onChange={setView} />} />
                )}
              </div>
            </motion.div>
          )}
        </AnimatePresence>
      </div>

      <Group title="Look" onReset={() => actions.reset("appearance")}>
        <SegmentedRow
          label="Tone"
          hint="Auto reads the wallpaper behind the bar to pick dark or light text."
          value={look.tone}
          options={[
            { value: "auto", label: "Auto" },
            { value: "light", label: "Dark text" },
            { value: "dark", label: "Light text" },
          ]}
          onChange={(tone) => actions.appearance(settings, { tone })}
        />
        <SegmentedRow
          label="Background"
          hint="Transparent shows the wallpaper through the bar."
          value={look.background}
          options={[
            { value: "translucent", label: "Transparent" },
            { value: "solid", label: "Solid" },
          ]}
          onChange={(background) => actions.appearance(settings, { background })}
        />
        <SliderRow
          label="Background strength"
          value={look.opacity}
          min={30}
          max={100}
          step={5}
          unit="%"
          disabled={look.background !== "solid"}
          onChange={(opacity) => actions.appearance(settings, { opacity })}
        />
        <SegmentedRow
          label="Menus and panels"
          hint="Blur needs Windows transparency effects on and battery saver off."
          value={look.blur}
          options={[
            { value: "auto", label: "Blur" },
            { value: "off", label: "Solid colours" },
          ]}
          onChange={(blur) => actions.appearance(settings, { blur })}
        />
        <SegmentedRow
          label="App icons"
          hint="Monochrome turns every app and tray icon into a silhouette in the bar's colour."
          value={look.iconStyle}
          options={[
            { value: "colour", label: "Colour" },
            { value: "mono", label: "Monochrome" },
          ]}
          onChange={(iconStyle) => actions.appearance(settings, { iconStyle })}
        />
      </Group>

      <Group title="Size and text">
        <SliderRow
          label="Bar height"
          hint="Menus and the space the bar reserves follow it."
          value={bar.height}
          min={20}
          max={64}
          unit=" px"
          onChange={actions.height}
        />
        <SliderRow
          label="Text size"
          value={look.fontSize}
          min={11}
          max={16}
          unit=" px"
          onChange={(fontSize) => actions.appearance(settings, { fontSize })}
        />
        <FontRow settings={settings} />
      </Group>

      <Group title="Clock and date" onReset={() => actions.reset("clock")}>
        <SegmentedRow
          label="Hour format"
          hint="Auto uses what Windows uses."
          value={clock.hourFormat}
          options={[
            { value: "auto", label: "Auto" },
            { value: "h12", label: "12 hour" },
            { value: "h24", label: "24 hour" },
          ]}
          onChange={(hourFormat) => actions.clock(settings, { hourFormat })}
        />
        <SwitchRow label="Show seconds" checked={clock.seconds} onChange={(seconds) => actions.clock(settings, { seconds })} />
        <SwitchRow label="Show the weekday" checked={clock.weekday} onChange={(weekday) => actions.clock(settings, { weekday })} />
        <SwitchRow label="Show the date" checked={clock.date} onChange={(date) => actions.clock(settings, { date })} />
        <SegmentedRow
          label="Date position"
          hint="In the middle, the date sits in the centre of the bar."
          value={clock.datePosition}
          options={[
            { value: "right", label: "Next to the time" },
            { value: "center", label: "Centre" },
          ]}
          onChange={(datePosition) => actions.clock(settings, { datePosition })}
        />
      </Group>

      <Group title="Tray icons" description="The icons apps used to put in the corner of the taskbar.">
        <SegmentedRow
          label="Show them"
          hint="In a panel under an arrow, or unfolded in the bar (needs room)."
          value={bar.tray.style}
          options={[
            { value: "panel", label: "In a panel" },
            { value: "inline", label: "In the bar" },
          ]}
          onChange={(style) => actions.tray(settings, { style })}
        />
        <SwitchRow
          label="Keep only pinned icons in the bar"
          hint="The rest wait in the tray panel."
          checked={bar.layout.collapsed}
          onChange={(on) => void commands.setCollapsed(on)}
        />
        <SwitchRow
          label="Hide Windows' own tray icons"
          hint="Safely Remove, Bluetooth and others Explorer owns. Some are the only way to reach that feature."
          checked={bar.tray.hideWindowsIcons}
          onChange={(hideWindowsIcons) => actions.tray(settings, { hideWindowsIcons })}
        />
      </Group>

      <Group title="Windows taskbar">
        <SwitchRow
          label="Auto-hide the taskbar"
          hint="Windows' own setting. Barista puts it back the way it was when you quit."
          checked={bar.taskbar.autoHide}
          onChange={actions.autoHideTaskbar}
        />
      </Group>
    </div>
  )
}
