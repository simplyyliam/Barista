import { useEffect, useState } from "react"
import { commands, type WifiNetwork } from "@/shared/bindings"
import { Switch } from "@barista/ui/components/ui/switch"
import { Input } from "@barista/ui/components/ui/input"
import { ArrowLeft01Icon, Icon, LockIcon, Tick02Icon, Wifi01Icon } from "@barista/ui/icons"
import { cn } from "cn"
import { useWifi } from "./hooks/use-control-center"

const JOIN_WAIT_MS = 10_000

/**
 * The Wi-Fi tile after it has grown to the full width: back, name and the radio switch. Without
 * `onBack` (the Wi-Fi panel opened from the bar) there is no back arrow.
 */
export function WifiHeader({ onBack }: { onBack?: () => void }) {
  const wifi = useWifi()
  const enabled = wifi?.enabled === true
  return (
    <div
      className={cn(
        "flex h-cc-tile w-full items-center gap-2.5 rounded-panel-child px-cc-label-pad text-cc-fg",
        enabled ? "bg-cc-active" : "bg-cc-tile",
      )}
    >
      {onBack && (
        <button
          type="button"
          aria-label="Back"
          onClick={onBack}
          className="-ml-1 flex size-8 items-center justify-center rounded-full outline-none hover:bg-cc-fg/15 focus-visible:ring-2 focus-visible:ring-cc-fg/60"
        >
          <Icon icon={ArrowLeft01Icon} size={18} />
        </button>
      )}
      <span className="flex-1 text-[13px] font-medium">Wi-Fi</span>
      <Switch
        aria-label="Wi-Fi"
        checked={enabled}
        disabled={!wifi?.available || wifi.accessDenied}
        onCheckedChange={(on) => void commands.setWifiEnabled(on)}
        className="data-checked:bg-cc-fg data-unchecked:bg-cc-fg/30 [&_[data-slot=switch-thumb]]:bg-cc-fg [&_[data-slot=switch-thumb][data-checked]]:bg-cc-active"
      />
    </div>
  )
}

/** The networks in range, filling the space under the header. Scrolls when there are many. */
export function WifiList() {
  const wifi = useWifi()
  // The network that asked for a password, and the one we just tried to join.
  const [asking, setAsking] = useState<string | null>(null)
  const [password, setPassword] = useState("")
  const [trying, setTrying] = useState<string | null>(null)
  const [failed, setFailed] = useState<string | null>(null)

  useEffect(() => {
    void commands.scanWifi()
  }, [])

  // Connected: nothing more to wait for. Still not connected after a while: say so.
  useEffect(() => {
    if (!trying) return
    if (wifi?.current === trying) {
      setTrying(null)
      return
    }
    const timer = setTimeout(() => {
      setFailed(trying)
      setTrying(null)
    }, JOIN_WAIT_MS)
    return () => clearTimeout(timer)
  }, [trying, wifi?.current])

  const join = (ssid: string, key: string | null) => {
    setAsking(null)
    setPassword("")
    setFailed(null)
    setTrying(ssid)
    void commands.joinWifi(ssid, key)
  }

  const pick = (network: WifiNetwork) => {
    if (network.connected) return void commands.disconnectWifi()
    // A join that just failed left a profile with the wrong password: delete it and ask again.
    if (failed === network.ssid && network.secured) {
      void commands.forgetWifi(network.ssid)
      setFailed(null)
      setAsking(network.ssid)
      setPassword("")
      return
    }
    setFailed(null)
    if (network.known) {
      setTrying(network.ssid)
      return void commands.connectWifi(network.ssid)
    }
    if (!network.secured) return join(network.ssid, null)
    setAsking(asking === network.ssid ? null : network.ssid)
    setPassword("")
  }

  let body
  if (!wifi?.available) {
    body = <Notice>No Wi-Fi adapter found</Notice>
  } else if (!wifi.enabled) {
    body = <Notice>Wi-Fi is off</Notice>
  } else if (!wifi.locationAllowed) {
    body = (
      <Notice>
        Windows hides network names until location access is allowed.
        <button
          type="button"
          onClick={() => void commands.openSettings("location")}
          className="mt-2 rounded-full bg-cc-active px-3 py-1 text-[12px] outline-none focus-visible:ring-2 focus-visible:ring-cc-fg/60"
        >
          Allow location
        </button>
      </Notice>
    )
  } else if (wifi.networks.length === 0) {
    body = <Notice>Looking for networks…</Notice>
  } else {
    body = wifi.networks.map((network) => (
      <div key={network.ssid} className="flex flex-col">
        <button
          type="button"
          onClick={() => pick(network)}
          className="flex h-10 shrink-0 items-center gap-2.5 rounded-[max(0px,calc(var(--radius-panel-child)-var(--spacing-cc-pad)))] px-3 text-left text-cc-fg outline-none hover:bg-cc-tile-hover focus-visible:ring-2 focus-visible:ring-cc-fg/60"
        >
          <Icon icon={Wifi01Icon} size={16} className={cn("shrink-0", network.signal < 35 && "opacity-50")} />
          <span className="flex min-w-0 flex-1 flex-col leading-tight">
            <span className="truncate text-[13px]">{network.ssid}</span>
            {(trying === network.ssid || failed === network.ssid) && (
              <span className="text-[11px] text-cc-fg-muted">
                {trying === network.ssid ? "Connecting…" : "Couldn't connect"}
              </span>
            )}
          </span>
          {network.secured && !network.connected && <Icon icon={LockIcon} size={13} className="shrink-0 opacity-70" />}
          {network.connected && <Icon icon={Tick02Icon} size={16} className="shrink-0" />}
        </button>
        {asking === network.ssid && (
          <form
            className="flex gap-2 px-3 pb-2"
            onSubmit={(event) => {
              event.preventDefault()
              if (password.length >= 8) join(network.ssid, password)
            }}
          >
            <Input
              type="password"
              autoFocus
              value={password}
              maxLength={63}
              placeholder="Password"
              aria-label={`Password for ${network.ssid}`}
              onChange={(event) => setPassword(event.target.value)}
              className="h-8 flex-1 border-0 bg-cc-tile text-[13px] text-cc-fg placeholder:text-cc-fg-muted"
            />
            <button
              type="submit"
              disabled={password.length < 8}
              className="rounded-full bg-cc-active px-3 text-[12px] text-cc-fg outline-none focus-visible:ring-2 focus-visible:ring-cc-fg/60 disabled:opacity-40"
            >
              Join
            </button>
          </form>
        )}
      </div>
    ))
  }

  return <div className="flex h-full flex-col overflow-y-auto rounded-panel-child bg-cc-tile p-1">{body}</div>
}

function Notice({ children }: { children: React.ReactNode }) {
  return (
    <div className="flex h-full flex-col items-center justify-center px-6 text-center text-[12px] text-cc-fg-muted">
      {children}
    </div>
  )
}
