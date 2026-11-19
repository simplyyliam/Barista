import { useEffect, useState } from "react"
import { commands, events, type UpdateInfo } from "@/shared/bindings"
import type { Settings } from "@/shared/stores/settings"
import { actions } from "./actions"
import { ActionButton, Group, Row, SwitchRow } from "./ui"

type Status = "idle" | "checking" | "installing"

/** Check for a new version, install it, and the switch for the automatic check. */
export function UpdatesGroup({ settings }: { settings: Settings }) {
  const [update, setUpdate] = useState<UpdateInfo | null>(null)
  const [status, setStatus] = useState<Status>("idle")
  const [message, setMessage] = useState("")

  // The bar checks by itself every few hours and tells us when it finds one.
  useEffect(() => {
    const off = events.updateAvailable.listen((e) => setUpdate(e.payload))
    return () => void off.then((f) => f())
  }, [])

  async function check() {
    setStatus("checking")
    setMessage("")
    const result = await commands.checkForUpdate()
    setStatus("idle")
    if (result.status === "error") return setMessage(result.error)
    setUpdate(result.data)
    if (!result.data) setMessage("You have the latest version.")
  }

  async function install() {
    setStatus("installing")
    const result = await commands.installUpdate()
    // On success the bar restarts and this window goes with it.
    if (result.status === "error") {
      setStatus("idle")
      setMessage(result.error)
    }
  }

  return (
    <Group title="Updates">
      <SwitchRow
        label="Check for updates automatically"
        hint="Looks every few hours. Nothing installs until you say so."
        checked={settings.behavior.autoUpdate}
        onChange={(on) => actions.behavior(settings, { autoUpdate: on })}
      />
      {update ? (
        <Row label={`Barista ${update.version} is ready`} hint="Installing restarts the bar for a moment.">
          <ActionButton disabled={status === "installing"} onClick={() => void install()}>
            {status === "installing" ? "Installing..." : "Update now"}
          </ActionButton>
        </Row>
      ) : (
        <Row label="Check now" hint={message || undefined}>
          <ActionButton disabled={status === "checking"} onClick={() => void check()}>
            {status === "checking" ? "Checking..." : "Check for updates"}
          </ActionButton>
        </Row>
      )}
    </Group>
  )
}
