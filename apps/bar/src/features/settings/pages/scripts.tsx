import { commands, type ScriptInfo } from "@/shared/bindings"
import { useScripts } from "@/shared/hooks/use-scripts"
import type { Settings } from "@/shared/stores/settings"
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@barista/ui/components/ui/select"
import { Switch } from "@barista/ui/components/ui/switch"
import { ActionButton, Group, PageTitle, Row, SelectRow, SwitchRow } from "../ui"

const INTERVALS = [5, 10, 30, 60, 300, 900, 3600]

/** 5 -> "5 s", 60 -> "1 min", 3600 -> "1 hour". */
function formatInterval(secs: number) {
  if (secs < 60) return `${secs} s`
  if (secs < 3600) return `${Math.round(secs / 60)} min`
  if (secs < 86400) return `${Math.round(secs / 3600)} hour`
  return `${Math.round(secs / 86400)} day`
}

/** The choices, plus the current value when it is not one of them (a hand-edited setting or a file name). */
function intervalOptions(current?: number) {
  const secs = current && !INTERVALS.includes(current) ? [...INTERVALS, current].sort((a, b) => a - b) : INTERVALS
  return secs.map((s) => ({ value: String(s), label: formatInterval(s) }))
}

function stateText(script: ScriptInfo) {
  switch (script.state) {
    case "error":
      return script.message ?? "The last run failed"
    case "needsTrust":
      return "Waiting for your OK before it runs"
    case "invalid":
      return script.message ?? "Can't be used"
    case "disabled":
      return "Off"
    default:
      return "Running"
  }
}

function ScriptRow({ script, scriptsOn }: { script: ScriptInfo; scriptsOn: boolean }) {
  if (script.state === "invalid") {
    return (
      <Row label={script.fileName} hint={stateText(script)}>
        <span />
      </Row>
    )
  }
  const every = `every ${formatInterval(script.intervalSecs)}${script.intervalFromName ? " (from the file name)" : ""}`
  const options = intervalOptions(script.intervalSecs)
  return (
    <Row label={script.title} hint={`${script.interpreter} · ${every} · ${stateText(script)}`} disabled={!scriptsOn}>
      {script.state === "needsTrust" && script.enabled && (
        // The review panel hangs from the bar item, so this clicks that item.
        <ActionButton onClick={() => void commands.itemClicked(`script/${script.id}`, "left")}>Review</ActionButton>
      )}
      <Select
        items={options}
        value={String(script.intervalSecs)}
        disabled={script.intervalFromName || !scriptsOn}
        onValueChange={(next) => next && void commands.setScriptInterval(script.id, Number(next))}
      >
        <SelectTrigger aria-label={`How often ${script.title} runs`} className="h-7 w-24 border-0 bg-white/[0.06] text-xs">
          <SelectValue />
        </SelectTrigger>
        <SelectContent alignItemWithTrigger={false} align="end">
          {options.map((option) => (
            <SelectItem key={option.value} value={option.value}>
              {option.label}
            </SelectItem>
          ))}
        </SelectContent>
      </Select>
      <Switch
        checked={script.enabled}
        disabled={!scriptsOn}
        onCheckedChange={(on) => void commands.setScriptEnabled(script.id, on)}
        aria-label={`Run ${script.title}`}
      />
    </Row>
  )
}

/**
 * Script items (v0.9): scripts in the scripts folder become bar items. They run with your own
 * permissions, which is why each one asks first and again when the file changes.
 */
export function ScriptsPage({ settings }: { settings: Settings }) {
  const scripts = useScripts()
  const prefs = settings.scripts

  return (
    <div className="flex flex-col gap-5">
      <PageTitle
        title="Script Items"
        description="Drop a script into the folder and it shows in the bar. The file name sets how often it runs, like cpu.10s.ps1."
      />
      <Group title="Scripts">
        <SwitchRow
          label="Run scripts"
          hint="Off stops every script and removes their items."
          checked={prefs.enabled}
          onChange={(on) => void commands.setScriptsEnabled(on)}
        />
        <SelectRow
          label="Default interval"
          hint="For scripts whose file name has no interval."
          value={String(prefs.defaultIntervalSecs)}
          options={intervalOptions(prefs.defaultIntervalSecs)}
          disabled={!prefs.enabled}
          onChange={(value) => void commands.setScriptsDefaultInterval(Number(value))}
        />
        <Row label="Scripts folder" hint="%APPDATA%\Barista\scripts">
          <ActionButton onClick={() => void commands.openScriptsFolder()}>Open scripts folder</ActionButton>
        </Row>
      </Group>
      <Group title="Your scripts">
        {!scripts || scripts.length === 0 ? (
          <Row label="No scripts yet" hint="Put a .ps1, .py, .js or .mjs file in the folder, like system-monitor.5s.ps1.">
            <span />
          </Row>
        ) : (
          scripts.map((script) => <ScriptRow key={script.id} script={script} scriptsOn={prefs.enabled} />)
        )}
      </Group>
      <p className="max-w-md text-xs text-muted-foreground/70">
        Scripts run with your permissions. Barista asks before the first run and again after any edit, but it is not a
        sandbox: only add scripts you trust.
      </p>
    </div>
  )
}
