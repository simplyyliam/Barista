import { useEffect, useState, type ReactNode } from "react"
import { motion, MotionConfig } from "motion/react"
import { commands } from "@/shared/bindings"
import { useBattery, useVolume } from "@/shared/hooks/use-system-state"
import { useSettings } from "@/shared/stores/settings"
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
import { DEFAULT_CONTROL_CENTER, gridRows } from "./lib/layout"
import { NowPlaying } from "./now-playing"
import { QuickActions } from "./quick-actions"
import { SystemSlider } from "./system-slider"
import { Tile } from "./tile"
import { UserHeader } from "./user-header"
import { WifiHeader, WifiList } from "./wifi-list"

/**
 * The size of the panel with the default layout, to the pixel (App.png): a card with the user row,
 * two rows of tiles, two sliders and the music row, then the quick action card. `openPanel` takes
 * it as its first guess so the window never resizes while it opens. A changed layout (settings
 * window) makes the panel report its real size once it is drawn.
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
 * The tiles, sliders, music row and quick actions follow the user's layout (settings window,
 * `settings.controlCenter`). The Wi-Fi tile opens its network list inside the card: the tile grows
 * to the full width, the other tiles, the sliders and the music row fade and shrink away, and the
 * list takes their place.
 */
export function ControlCenterPanel({ active }: { active: boolean }) {
  const [wifiOpen, setWifiOpen] = useState(false)
  useEffect(() => {
    if (!active) setWifiOpen(false)
  }, [active])

  const prefs = useSettings()?.controlCenter ?? DEFAULT_CONTROL_CENTER
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

  const tiles = prefs.tiles.filter((tile) => tile.visible)
  const rows = gridRows(tiles.map((tile) => (tile.size === "wide" ? 2 : 1)))
  // A tile shows its label only when it is wide (a small one is just the icon).
  const text = (wide: boolean, label: string, sub?: string) => (wide ? { label, sub } : { "aria-label": label })

  const tileFor = (id: string, wide: boolean): ReactNode => {
    switch (id) {
      case "airplane":
        return (
          <Tile
            icon={AirplaneModeIcon}
            {...text(wide, "Airplane mode")}
            active={airplane?.on === true}
            disabled={!airplane?.supported}
            onClick={() => void commands.setAirplaneMode(!airplane?.on)}
          />
        )
      case "night-light":
        return (
          <Tile
            icon={Moon02Icon}
            {...text(wide, "Night light")}
            active={nightOn}
            disabled={!nightLight?.supported}
            onClick={() => {
              setNightWanted(!nightOn)
              void commands.setNightLight(!nightOn)
            }}
          />
        )
      case "bluetooth":
        return (
          <Tile
            icon={BluetoothIcon}
            {...text(wide, "Bluetooth")}
            active={bluetooth?.enabled === true}
            disabled={!bluetooth?.available || bluetooth.accessDenied}
            onClick={() => void commands.setBluetoothEnabled(!bluetooth?.enabled)}
          />
        )
      case "hotspot":
        return (
          <Tile
            icon={HotspotIcon}
            {...text(wide, "Mobile hotspot")}
            active={hotspot?.state === "on"}
            className={hotspot?.state === "changing" ? "animate-pulse" : undefined}
            disabled={!hotspot?.supported || hotspot.state === "changing"}
            onClick={() => void commands.setHotspotEnabled(hotspot?.state !== "on")}
          />
        )
      case "battery-saver":
        return (
          <Tile
            icon={BatteryEcoChargingIcon}
            {...text(wide, "Battery Saver")}
            active={battery?.saver === true}
            disabled={!battery?.present || !battery.saverControllable}
            onClick={() => void commands.setBatterySaver(!battery?.saver)}
          />
        )
      default:
        return null
    }
  }

  return (
    <MotionConfig reducedMotion="user">
      <div className="flex w-panel-w flex-col gap-2">
        <section className="flex flex-col gap-cc-gap rounded-panel bg-cc-card p-cc-pad">
          {prefs.userCard && <UserHeader active={active} />}

          <div className="relative flex flex-col gap-cc-gap">
            {/* A fixed height for the rows the tiles need: while the Wi-Fi tile is wide the cells
                below it move down, and that must not push the sliders around. */}
            {tiles.length > 0 && (
              <div
                className="grid grid-cols-4 content-start gap-cc-gap"
                style={{ height: `calc(var(--spacing-cc-tile) * ${rows} + var(--spacing-cc-gap) * ${rows - 1})` }}
              >
                {tiles.map((tile) =>
                  tile.id === "wifi" ? (
                    <motion.div
                      key={tile.id}
                      layout
                      transition={{ layout: { duration: 0.28, ease: [0.2, 0, 0, 1] } }}
                      className={wifiOpen ? "col-span-4" : tile.size === "wide" ? "col-span-2" : undefined}
                    >
                      {wifiOpen ? (
                        <WifiHeader onBack={() => setWifiOpen(false)} />
                      ) : (
                        <Tile
                          icon={Wifi01Icon}
                          {...text(tile.size === "wide", "Wi-Fi", wifiSub)}
                          active={wifi?.enabled === true}
                          disabled={!wifi?.available}
                          onClick={() => setWifiOpen(true)}
                        />
                      )}
                    </motion.div>
                  ) : (
                    <Cell key={tile.id} span={tile.size === "wide" ? 2 : 1} hidden={wifiOpen}>
                      {tileFor(tile.id, tile.size === "wide")}
                    </Cell>
                  ),
                )}
              </div>
            )}

            {prefs.brightness && (
              <Cell hidden={wifiOpen}>
                <BrightnessSlider />
              </Cell>
            )}
            {prefs.volume && (
              <Cell hidden={wifiOpen}>
                <VolumeSlider />
              </Cell>
            )}
            {prefs.nowPlaying && (
              <Cell hidden={wifiOpen}>
                <NowPlaying />
              </Cell>
            )}

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
