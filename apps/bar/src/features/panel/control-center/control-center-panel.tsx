import { useEffect, useState, type ReactNode } from "react"
import { motion, MotionConfig } from "motion/react"
import { commands } from "@/shared/bindings"
import { useBattery, useVolume } from "@/shared/hooks/use-system-state"
import {
  AirplaneModeIcon,
  BatteryEcoChargingIcon,
  BluetoothIcon,
  HotspotIcon,
  Moon02Icon,
  Sun03Icon,
  VolumeHighIcon,
  VolumeMute01Icon,
  Wifi01Icon,
} from "@barista/ui/icons"
import {
  useAirplane,
  useBluetooth,
  useBrightness,
  useDragValue,
  useHotspot,
  useNightLight,
  useWifi,
} from "./hooks/use-control-center"
import { NowPlaying } from "./now-playing"
import { QuickActions } from "./quick-actions"
import { SystemSlider } from "./system-slider"
import { Tile } from "./tile"
import { UserHeader } from "./user-header"
import { WifiHeader, WifiList } from "./wifi-list"

/**
 * The size of the panel, to the pixel (App.png): a card with the user row, two rows of tiles, two
 * sliders and the music row, then the quick action card. `openPanel` takes it as its first guess so
 * the window never resizes while it opens.
 */
export const CONTROL_CENTER_WIDTH = 300
export const CONTROL_CENTER_HEIGHT = 508

const FADE = { duration: 0.18, ease: [0.2, 0, 0, 1] as const }

/** A tile that fades and shrinks out of the way while the Wi-Fi list is open. */
function Cell({ span = 1, hidden, children }: { span?: 1 | 2; hidden: boolean; children: ReactNode }) {
  return (
    <motion.div
      className={span === 2 ? "col-span-2" : undefined}
      animate={hidden ? { opacity: 0, scale: 0.9 } : { opacity: 1, scale: 1 }}
      transition={FADE}
      style={{ pointerEvents: hidden ? "none" : undefined }}
      aria-hidden={hidden}
      inert={hidden}
    >
      {children}
    </motion.div>
  )
}

/**
 * The `control-center` route (App.png, top right). `active` is false while the window is hidden:
 * everything stays mounted and loaded, but the Wi-Fi list closes so the next open starts clean.
 *
 * The Wi-Fi tile opens its network list inside the card: the tile grows to the full width, the
 * other tiles, the sliders and the music row fade and shrink away, and the list takes their place.
 */
export function ControlCenterPanel({ active }: { active: boolean }) {
  const [wifiOpen, setWifiOpen] = useState(false)
  useEffect(() => {
    if (!active) setWifiOpen(false)
  }, [active])

  const wifi = useWifi()
  const bluetooth = useBluetooth()
  const airplane = useAirplane()
  const hotspot = useHotspot()
  const nightLight = useNightLight()
  // What you asked for, until Windows' answer comes back: the tile flips on the click, and a fast
  // second click toggles what you see (not the answer that is still on its way).
  const [nightWanted, setNightWanted] = useState<boolean | null>(null)
  useEffect(() => {
    if (nightWanted === null) return
    if (nightLight?.on === nightWanted) {
      setNightWanted(null)
      return
    }
    const timer = setTimeout(() => setNightWanted(null), 1500)
    return () => clearTimeout(timer)
  }, [nightWanted, nightLight?.on])
  const nightOn = nightWanted ?? nightLight?.on === true
  const battery = useBattery()

  const wifiSub = !wifi?.available ? "No adapter" : !wifi.enabled ? "Off" : (wifi.current ?? "Not connected")

  return (
    <MotionConfig reducedMotion="user">
      <div className="flex w-panel-w flex-col gap-2">
        <section className="flex flex-col gap-cc-gap rounded-panel bg-cc-card p-cc-pad">
          <UserHeader active={active} />

          <div className="relative flex flex-col gap-cc-gap">
            {/* Two rows of tiles. A fixed height: while the Wi-Fi tile is wide the cells below it
                move down, and that must not push the sliders around. */}
            <div className="grid h-[calc(var(--spacing-cc-tile)*2+var(--spacing-cc-gap))] grid-cols-4 content-start gap-cc-gap">
              <motion.div
                layout
                transition={{ layout: { duration: 0.28, ease: [0.2, 0, 0, 1] } }}
                className={wifiOpen ? "col-span-4" : "col-span-2"}
              >
                {wifiOpen ? (
                  <WifiHeader onBack={() => setWifiOpen(false)} />
                ) : (
                  <Tile
                    icon={Wifi01Icon}
                    label="Wi-Fi"
                    sub={wifiSub}
                    active={wifi?.enabled === true}
                    disabled={!wifi?.available}
                    onClick={() => setWifiOpen(true)}
                  />
                )}
              </motion.div>
              <Cell hidden={wifiOpen}>
                <Tile
                  icon={AirplaneModeIcon}
                  aria-label="Airplane mode"
                  active={airplane?.on === true}
                  disabled={!airplane?.supported}
                  onClick={() => void commands.setAirplaneMode(!airplane?.on)}
                />
              </Cell>
              <Cell hidden={wifiOpen}>
                <Tile
                  icon={Moon02Icon}
                  aria-label="Night light"
                  active={nightOn}
                  disabled={!nightLight?.supported}
                  onClick={() => {
                    setNightWanted(!nightOn)
                    void commands.setNightLight(!nightOn)
                  }}
                />
              </Cell>
              <Cell hidden={wifiOpen}>
                <Tile
                  icon={BluetoothIcon}
                  aria-label="Bluetooth"
                  active={bluetooth?.enabled === true}
                  disabled={!bluetooth?.available || bluetooth.accessDenied}
                  onClick={() => void commands.setBluetoothEnabled(!bluetooth?.enabled)}
                />
              </Cell>
              <Cell hidden={wifiOpen}>
                <Tile
                  icon={HotspotIcon}
                  aria-label="Mobile hotspot"
                  active={hotspot?.state === "on"}
                  className={hotspot?.state === "changing" ? "animate-pulse" : undefined}
                  disabled={!hotspot?.supported || hotspot.state === "changing"}
                  onClick={() => void commands.setHotspotEnabled(hotspot?.state !== "on")}
                />
              </Cell>
              <Cell span={2} hidden={wifiOpen}>
                <Tile
                  icon={BatteryEcoChargingIcon}
                  label="Battery Saver"
                  active={battery?.saver === true}
                  disabled={!battery?.present || !battery.saverControllable}
                  onClick={() => void commands.setBatterySaver(!battery?.saver)}
                />
              </Cell>
            </div>

            <Cell hidden={wifiOpen}>
              <BrightnessSlider />
            </Cell>
            <Cell hidden={wifiOpen}>
              <VolumeSlider />
            </Cell>
            <Cell hidden={wifiOpen}>
              <NowPlaying />
            </Cell>

            {/* The list sits under the grown Wi-Fi tile, over the faded-out controls. */}
            {wifiOpen && (
              <motion.div
                className="absolute inset-x-0 top-[calc(var(--spacing-cc-tile)+var(--spacing-cc-gap))] bottom-0"
                initial={{ opacity: 0, y: 6 }}
                animate={{ opacity: 1, y: 0 }}
                transition={{ ...FADE, delay: 0.08 }}
              >
                <WifiList />
              </motion.div>
            )}
          </div>
        </section>

        <QuickActions />
      </div>
    </MotionConfig>
  )
}

function BrightnessSlider() {
  const brightness = useBrightness()
  const display = brightness?.displays.find((d) => d.supported && d.level !== null)
  const drag = useDragValue(display?.level ?? null, (level) => {
    if (display) void commands.setBrightness(display.id, level)
  })
  return (
    <SystemSlider
      label="Brightness"
      icon={Sun03Icon}
      value={drag.value}
      disabled={!display}
      onChange={drag.change}
      onCommit={drag.end}
    />
  )
}

export function VolumeSlider() {
  const volume = useVolume()
  const drag = useDragValue(volume?.level ?? null, (level) => void commands.setVolume(level))
  return (
    <SystemSlider
      label="Volume"
      icon={volume?.muted ? VolumeMute01Icon : VolumeHighIcon}
      value={volume?.muted ? 0 : drag.value}
      disabled={!volume?.available}
      // Moving a muted volume up unmutes it, like the volume keys do.
      onChange={(level) => {
        if (volume?.muted && level > 0) void commands.setMuted(false)
        drag.change(level)
      }}
      onCommit={drag.end}
      onIconClick={() => void commands.setMuted(!volume?.muted)}
    />
  )
}
