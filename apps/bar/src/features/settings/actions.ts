import { commands } from "@/shared/bindings"
import type { Settings } from "@/shared/stores/settings"

type Bar = Settings["menuBar"]

/**
 * Small wrappers for the settings window: each one sends a whole section with one field changed,
 * and Rust answers with `settingsChanged` (the store then redraws everything). Rust validates and
 * clamps every value, so nothing here needs to.
 */
export const actions = {
  appearance: (s: Settings, patch: Partial<Bar["appearance"]>) =>
    void commands.setAppearance({ ...s.menuBar.appearance, ...patch }),
  clock: (s: Settings, patch: Partial<Bar["clock"]>) => void commands.setClockPrefs({ ...s.menuBar.clock, ...patch }),
  tray: (s: Settings, patch: Partial<Bar["tray"]>) => void commands.setTrayPrefs({ ...s.menuBar.tray, ...patch }),
  items: (items: Bar["items"]) => void commands.setMenuBarItems(items),
  controlCenter: (s: Settings, patch: Partial<Settings["controlCenter"]>) =>
    void commands.setControlCenterPrefs({ ...s.controlCenter, ...patch }),
  behavior: (s: Settings, patch: Partial<Settings["behavior"]>) =>
    void commands.setBehavior({ ...s.behavior, ...patch }),
  advanced: (s: Settings, patch: Partial<Settings["advanced"]>) =>
    void commands.setAdvanced({ ...s.advanced, ...patch }),
  height: (height: number) => void commands.setBarHeight(height),
  autoHideTaskbar: (on: boolean) => void commands.setTaskbarAutoHide(on),
  startWithWindows: (on: boolean) => void commands.setStartWithWindows(on),
  reset: (section: Parameters<typeof commands.resetSettings>[0]) => void commands.resetSettings(section),
}
