import { useRef, useState } from "react"
import { commands } from "@/shared/bindings"
import { useItems } from "@/shared/hooks/use-items"
import type { Settings } from "@/shared/stores/settings"
import { actions } from "../actions"
import { PageIcon, pageOf } from "../sidebar"
import type { PageId } from "../pages"
import { ActionButton, Group, PageTitle, Row, SegmentedRow } from "../ui"

/** Apps that put items in the bar, with how many each one has. Tray icons are listed apart. */
export function ConnectedAppsPage() {
  const items = useItems()
  const apps = new Map<string, string[]>()
  for (const item of items) {
    if (item.source !== "app") continue
    const list = apps.get(item.appName) ?? []
    list.push(item.label || item.tooltip || "Item")
    apps.set(item.appName, list)
  }
  const tray = items.filter((item) => item.source === "tray").length

  return (
    <div className="flex flex-col gap-5">
      <PageTitle
        title="Connected Apps"
        description="Apps that added something to the bar with the Barista SDK. They leave when the app quits."
      />
      <Group title="Apps in the bar">
        {apps.size === 0 ? (
          <Row label="No apps connected" hint="Start an app that uses the SDK, like the system monitor or mic mute.">
            <span />
          </Row>
        ) : (
          [...apps.entries()].map(([name, labels]) => (
            <Row key={name} label={name} hint={labels.join(", ")}>
              <span className="text-xs text-muted-foreground">
                {labels.length} {labels.length === 1 ? "item" : "items"}
              </span>
            </Row>
          ))
        )}
      </Group>
      <Group title="Windows tray">
        <Row label="Tray icons in the bar" hint="Change how they show under Menu bar.">
          <span className="text-xs text-muted-foreground">{tray}</span>
        </Row>
      </Group>
    </div>
  )
}

/** The `barista | windows` switches (CLAUDE.md rule 11), and backup of the settings. */
export function AdvancedPage({ settings }: { settings: Settings }) {
  const adv = settings.advanced
  const choice = [
    { value: "barista" as const, label: "Barista" },
    { value: "windows" as const, label: "Windows" },
  ]
  const [message, setMessage] = useState("")
  const file = useRef<HTMLInputElement>(null)

  const exportFile = async () => {
    const result = await commands.exportSettings()
    if (result.status !== "ok") return setMessage(result.error)
    const url = URL.createObjectURL(new Blob([result.data], { type: "application/json" }))
    const link = document.createElement("a")
    link.href = url
    link.download = "barista-settings.json"
    link.click()
    URL.revokeObjectURL(url)
    setMessage("Settings saved to a file.")
  }
  const importFile = async (picked: File | undefined) => {
    if (!picked) return
    if (picked.size > 256 * 1024) return setMessage("That file is too big to be a settings backup.")
    const result = await commands.importSettings(await picked.text())
    setMessage(result.status === "ok" ? "Settings loaded." : result.error)
  }

  return (
    <div className="flex flex-col gap-5">
      <PageTitle
        title="Advanced"
        description="Let Windows do some things itself. Barista then stops its own work for that feature and opens the Windows version."
      />
      <Group title="Who draws it">
        <SegmentedRow
          label="Notifications and calendar"
          hint="Windows mode opens Windows' own notification center."
          value={adv.notifications}
          options={choice}
          onChange={(notifications) => actions.advanced(settings, { notifications })}
        />
        <SegmentedRow
          label="Control center"
          hint="Wi-Fi, Bluetooth, brightness and more. Windows mode opens Quick Settings."
          value={adv.controlCenter}
          options={choice}
          onChange={(controlCenter) => actions.advanced(settings, { controlCenter })}
        />
        <SegmentedRow
          label="Tray icons"
          hint="Windows mode leaves the tray with Explorer. No tray icons in the bar."
          value={adv.tray}
          options={choice}
          onChange={(tray) => actions.advanced(settings, { tray })}
        />
      </Group>
      <Group title="Back up and reset" description={message || undefined}>
        <Row label="Save settings to a file" hint="Your choices, without this PC's monitor.">
          <ActionButton onClick={() => void exportFile()}>Save</ActionButton>
        </Row>
        <Row label="Load settings from a file">
          <ActionButton onClick={() => file.current?.click()}>Load</ActionButton>
          <input
            ref={file}
            type="file"
            accept="application/json,.json"
            hidden
            onChange={(e) => {
              void importFile(e.target.files?.[0])
              e.target.value = ""
            }}
          />
        </Row>
        <Row label="Reset everything" hint="Back to how Barista came. Your pins and order stay.">
          <ActionButton tone="danger" onClick={() => actions.reset("all")}>
            Reset
          </ActionButton>
        </Row>
      </Group>
    </div>
  )
}

/** Pages with nothing to set yet say which release brings them. */
export function ComingSoonPage({ id }: { id: PageId }) {
  const page = pageOf(id)
  return (
    <div className="flex flex-col gap-5">
      <PageTitle title={page.label} />
      <div className="flex flex-col items-center gap-2 px-6 py-20 text-center text-muted-foreground/60">
        <PageIcon page={page} size={28} />
        <p className="text-[13px] text-foreground/80">Coming in {page.comingIn ?? "a later release"}</p>
        <p className="max-w-80 text-xs">Nothing to set here yet.</p>
      </div>
    </div>
  )
}
