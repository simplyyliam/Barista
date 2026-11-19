import { useEffect, useState } from "react"
import { Button } from "@barista/ui/components/ui/button"
import { commands } from "@/shared/bindings"
import { useScriptTrustRequests } from "@/shared/hooks/use-scripts"

/** A bit wider than the other panels so the first lines of a script read well. */
export const SCRIPT_TRUST_PANEL_WIDTH = 380

function formatSize(bytes: number) {
  return bytes < 1024 ? `${bytes} B` : `${(bytes / 1024).toFixed(1)} KB`
}

/**
 * "Run this script?" The bar asks once per script and again when the file changes. It shows what
 * Rust read from the file (name, interpreter, size, hash, first lines). The answer carries the
 * hash back: Rust only trusts the file if it still has it. Scripts are not sandboxed, and the
 * panel says so.
 */
export function ScriptTrustPanel({ active }: { active: boolean }) {
  const requests = useScriptTrustRequests()
  const current = requests?.[0]
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState("")

  // Nothing is waiting (answered, or the file was deleted): there is nothing left to show.
  useEffect(() => {
    if (active && requests && requests.length === 0) void commands.closePanel()
  }, [active, requests])
  useEffect(() => setError(""), [current?.id, current?.sha256])

  if (!current) return <div className="h-24" style={{ width: SCRIPT_TRUST_PANEL_WIDTH }} />

  const answer = async (allow: boolean) => {
    setBusy(true)
    const result = await commands.answerScriptTrust(current.id, current.sha256, allow)
    setBusy(false)
    if (result.status === "error") setError(result.error)
  }

  return (
    <div className="flex flex-col gap-3 p-4 text-[13px]" style={{ width: SCRIPT_TRUST_PANEL_WIDTH }}>
      <div className="flex items-baseline justify-between gap-3">
        <div className="min-w-0">
          <div className="truncate font-medium">Run {current.fileName}?</div>
          <div className="text-[12px] opacity-60">
            {current.interpreter} · {formatSize(current.size)} · SHA-256 {current.sha256.slice(0, 12)}…
          </div>
        </div>
        {requests && requests.length > 1 && (
          <span className="shrink-0 text-[12px] opacity-60">1 of {requests.length}</span>
        )}
      </div>
      <pre
        aria-label="The first lines of the script"
        className="max-h-44 overflow-auto rounded-md bg-black/25 p-2.5 font-mono text-[11px] leading-snug whitespace-pre select-text"
      >
        {current.preview.length ? current.preview.join("\n") : "(empty file)"}
      </pre>
      <p className="text-[12px] leading-snug opacity-60">
        Scripts run with your permissions. Barista can't sandbox them, so only allow ones you trust. If the file
        changes, you'll be asked again.
      </p>
      {error && (
        <p role="alert" className="text-[12px] text-red-400">
          {error}
        </p>
      )}
      <div className="flex justify-end gap-2">
        <Button variant="ghost" size="sm" className="h-7 text-xs" disabled={busy} onClick={() => void answer(false)}>
          Don't allow
        </Button>
        <Button size="sm" className="h-7 text-xs" disabled={busy} onClick={() => void answer(true)}>
          Allow
        </Button>
      </div>
    </div>
  )
}
