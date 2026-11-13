import { useEffect, useState } from "react"
import { commands } from "@/shared/bindings"
import { WifiHeader, WifiList } from "./control-center/wifi-list"

/** Same width as the control center (--spacing-panel-w), so both look like one family. */
export const WIFI_PANEL_WIDTH = 300
/** A first guess: card padding + header tile + gap + 240px of list. Rust remembers the measured size. */
export const WIFI_PANEL_HEIGHT = 330

/**
 * The `wifi` route: the control center's Wi-Fi view (same header with the radio switch, same
 * network list, same password and join flow) as a card of its own, opened from the Wi-Fi item in
 * the bar. It is the very same `WifiHeader` and `WifiList`, so the two cannot drift apart.
 *
 * Always mounted like the other routes, so it opens fully loaded. While the window is hidden the
 * list is mounted again from scratch, so a half-typed password never survives a close, and a new
 * Wi-Fi scan starts on every open (the control center does the same when its list opens).
 */
export function WifiPanel({ active }: { active: boolean }) {
  const [session, setSession] = useState(0)

  useEffect(() => {
    if (active) void commands.scanWifi()
    else setSession((n) => n + 1)
  }, [active])

  return (
    <div className="flex w-panel-w flex-col">
      <section className="flex flex-col gap-cc-gap rounded-panel bg-cc-card p-cc-pad">
        <WifiHeader plain />
        <div className="h-60">
          <WifiList key={session} scanOnMount={false} />
        </div>
      </section>
    </div>
  )
}
