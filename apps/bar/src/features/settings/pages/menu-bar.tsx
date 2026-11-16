import { useState } from "react"
import { commands } from "@/shared/bindings"
import { Input } from "@barista/ui/components/ui/input"
import { Tabs, TabsList, TabsTrigger } from "@barista/ui/components/ui/tabs"
import type { Settings } from "@/shared/stores/settings"
import { actions } from "../actions"
import { BarEditor } from "../bar-editor"
import { ControlCenterEditor } from "../control-center-editor"
import { Group, PageTitle, Row, SegmentedRow, SliderRow, SwitchRow } from "../ui"

type View = "bar" | "control-center"

const FONTS = ["Segoe UI", "Segoe UI Variable", "Bahnschrift", "Calibri", "Cascadia Code", "Consolas", "Inter"]

/** The two tabs under the picture in the design: which thing the preview shows and edits (shadcn Tabs). */
function ViewTabs({ view, onChange }: { view: View; onChange: (view: View) => void }) {
  return (
    <Tabs value={view} onValueChange={(next) => onChange(next as View)}>
      <TabsList aria-label="What to edit">
        <TabsTrigger value="bar">Menu bar</TabsTrigger>
        <TabsTrigger value="control-center">Control center</TabsTrigger>
      </TabsList>
    </Tabs>
  )
}

function FontRow({ settings }: { settings: Settings }) {
  const saved = settings.menuBar.appearance.font
  const [draft, setDraft] = useState<string | null>(null)
  const commit = () => {
    if (draft !== null && draft.trim() !== saved) actions.appearance(settings, { font: draft.trim() })
    setDraft(null)
  }
  return (
    <Row label="Font" hint="Any font installed on this PC. Empty uses Segoe UI Variable.">
      <Input
        aria-label="Font"
        list="barista-fonts"
        placeholder="Segoe UI Variable"
        maxLength={64}
        value={draft ?? saved}
        onChange={(e) => setDraft(e.target.value)}
        onBlur={commit}
        onKeyDown={(e) => e.key === "Enter" && commit()}
        className="w-60"
      />
      <datalist id="barista-fonts">
        {FONTS.map((f) => (
          <option key={f} value={f} />
        ))}
      </datalist>
    </Row>
  )
}

export function MenuBarPage({ settings }: { settings: Settings }) {
  const [view, setView] = useState<View>("bar")
  const bar = settings.menuBar
  const look = bar.appearance
  const clock = bar.clock

  return (
    <div className="flex flex-col gap-6">
      <PageTitle
        title="Menu bar"
        description="Drag things around in the picture. Everything you change shows up in your bar at once."
      />

      <div className="flex flex-col gap-3">
        {view === "bar" ? <BarEditor settings={settings} /> : <ControlCenterEditor settings={settings} />}
        <ViewTabs view={view} onChange={setView} />
      </div>

      <h2 className="-mb-2 pt-4 text-lg font-medium tracking-tight text-foreground">Appearance</h2>

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
