import { commands } from "@/shared/bindings"
import type { Settings } from "@/shared/stores/settings"
import { actions } from "../actions"
import { useBarInfo } from "../hooks/use-bar-info"
import { ActionButton, Group, PageTitle, Row, SelectRow, SwitchRow } from "../ui"
import { UpdatesGroup } from "../updates"

export function GeneralPage({ settings }: { settings: Settings }) {
  const info = useBarInfo()
  const monitors = info?.monitors ?? []
  const current = settings.general.monitor ?? info?.monitor.id ?? ""

  return (
    <div className="flex flex-col gap-5">
      <PageTitle title="General" description="How Barista starts and where the bar lives." />

      <Group title="Startup">
        <SwitchRow
          label="Start Barista when Windows starts"
          hint="Adds Barista to your startup apps. Windows' Task Manager can switch it off too."
          checked={settings.general.startWithWindows}
          onChange={actions.startWithWindows}
        />
      </Group>

      <Group title="Display" description="The bar lives on one monitor.">
        <SelectRow
          label="Monitor"
          hint={monitors.length < 2 ? "Only one monitor is connected." : undefined}
          value={current}
          disabled={monitors.length < 2}
          options={monitors.map((m) => ({ value: m.id, label: `${m.name}${m.isPrimary ? " (main)" : ""}` }))}
          onChange={(id) => void commands.setMonitor(id)}
        />
        <SwitchRow
          label="Hide the bar for full-screen apps"
          hint="Games and videos get the whole screen."
          checked={settings.behavior.hideOnFullscreen}
          onChange={(on) => actions.behavior(settings, { hideOnFullscreen: on })}
        />
      </Group>

      <UpdatesGroup settings={settings} />

      <Group title="Barista">
        <Row label="Log files" hint="Barista keeps a small log on this PC for bug reports. It never leaves your computer.">
          <ActionButton onClick={() => void commands.openLogsFolder()}>Open folder</ActionButton>
        </Row>
        <Row label="Quit Barista" hint="Takes the bar away and gives the screen space back.">
          <ActionButton tone="danger" onClick={() => void commands.quitBar()}>
            Quit
          </ActionButton>
        </Row>
      </Group>
    </div>
  )
}
