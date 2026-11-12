# 0020: Real toggles for the control center, and now playing

Status: accepted (2026-11-12, v0.4b control center UI). Liam decided: no tile may only open a Windows settings page.

## Context

Liam's rule for the control center: a tile that just opens Windows Settings is bad UX, so every tile
does what it says. Windows has no public API for several of them, so this records what we use, what
could break, and what the tile does then. When a method stops working, the tile is **greyed out**
(`supported: false`); the bar never writes bytes it doesn't recognise and never falls back to a link.

## What each one uses

| Tile | How | Wakes us | If it breaks |
| --- | --- | --- | --- |
| Airplane mode | `IRadioManager::SetSystemRadioState` (undocumented COM, same one Quick Settings uses); state from the `SystemRadioState` registry value | registry watch | `radio_manager::set_airplane` returns false, logged, the state stays what Windows says |
| Night light | the CloudStore `bluelightreductionstate` binary value in HKCU: flip the state byte, add or remove two bytes, write both timestamps (Unix seconds, FILETIME). Windows ignores a change that does not look newer, so the stamps always increase (two clicks in one second would otherwise lose the second). Tested byte for byte against values Windows wrote itself | registry watch | the layout is checked before reading or writing; an unknown layout means `supported: false` |
| Battery saver | **Read only.** Windows keeps the state in `HKLM\SYSTEM\CurrentControlSet\Control\Power\EnergySaverState` (1 = on, 2 = off), and the bar reads and watches it, so the tile follows Quick Settings, also plugged in (Windows' own `PowerManager.EnergySaverStatus` and the power status flag stay off on AC, so they can't be used). **Writing that value does not switch anything**: tested with the helper service (admin write worked, value went 1 → 2) while Windows' own panel stayed on. The value is a record, not the switch. The charge-level setting (`ESBATTTHRESHOLD`) was tried first and judged by the wrong flag, so it is also unproven | registry watch | `saverControllable` is false, the tile shows the state and is greyed out. To find: the call Quick Settings makes (an undocumented power service call) |
| Dark mode | `AppsUseLightTheme` and `SystemUsesLightTheme` in HKCU, then `WM_SETTINGCHANGE "ImmersiveColorSet"` so apps pick it up | registry watch | plain registry, stable since Windows 10 |
| Wi-Fi (new network) | `WlanSetProfile` with a WPA / WPA2 / WPA3 / open profile built from the SSID and password, then `WlanConnect`. SSID must be in the current list, password 8 to 63 characters, XML escaped, never logged or kept. Enterprise (802.1X) opens Windows' own flyout | WLAN notification (already there) | Windows refuses the profile: logged, the list shows "Couldn't connect" |
| Now playing | `GlobalSystemMediaTransportControlsSessionManager` (SMTC, public): the same session as Windows' volume flyout, so Spotify and browsers work with no per-app code. Cover limited to 2 MB and checked to be JPEG or PNG; text clipped to 200 chars | `CurrentSessionChanged`, `MediaPropertiesChanged`, `PlaybackInfoChanged` | no session = `available: false`, the tile shows "Not playing" |

All of them sit behind the control center switch (`controlCenterMode`, CLAUDE.md rule 11): in `windows`
mode their snapshots are emptied and the bar opens Quick Settings.

## The helper

Each admin-only action gets its own fixed control code and its own test; nothing takes free-form input. It exists for admin-only actions (`scripts/helper/install.ps1`). Energy saver was meant to be the first one, but writing its registry value does not actuate it, so nothing uses the helper yet; night light and airplane mode work without admin. The installer (v0.8) should run the same steps as `scripts/helper/install.ps1`.

## Later

- `Media.source` is `"system"`. The Spotify web connection (sign in with a Spotify account in the v0.8
  settings, for music playing on another device) becomes a second source behind the same snapshot.
- If a new Windows build changes the night light layout, add the new layout to `night_light.rs` next to
  a real sample and a test; do not loosen the checks.
