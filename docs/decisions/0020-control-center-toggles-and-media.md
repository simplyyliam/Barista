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
| Battery saver | Windows keeps the switch in `HKLM\SYSTEM\CurrentControlSet\Control\Power\EnergySaverState` (1 = on, 2 = off); flipping it in Quick Settings changes nothing else (captured on this PC, also plugged in). Only administrators may write it, so a **helper service** (`crates/saver-helper`, `BaristaSaver`, LocalSystem, installed once with `pnpm helper:install`) writes it. The bar sends a *service control code* (128 on, 129 off), no data and no pipe; the service's access list lets signed-in users send codes and nothing else, and any other code is refused. The exe lives in Program Files (admin-only), never in a user-writable folder. The bar reads and watches the same value, so the tile follows Windows | registry watch | helper not installed or stopped: `saverControllable: false`, the tile shows the state but is greyed out |
| Dark mode | `AppsUseLightTheme` and `SystemUsesLightTheme` in HKCU, then `WM_SETTINGCHANGE "ImmersiveColorSet"` so apps pick it up | registry watch | plain registry, stable since Windows 10 |
| Wi-Fi (new network) | `WlanSetProfile` with a WPA / WPA2 / WPA3 / open profile built from the SSID and password, then `WlanConnect`. SSID must be in the current list, password 8 to 63 characters, XML escaped, never logged or kept. Enterprise (802.1X) opens Windows' own flyout | WLAN notification (already there) | Windows refuses the profile: logged, the list shows "Couldn't connect" |
| Now playing | `GlobalSystemMediaTransportControlsSessionManager` (SMTC, public): the same session as Windows' volume flyout, so Spotify and browsers work with no per-app code. Cover limited to 2 MB and checked to be JPEG or PNG; text clipped to 200 chars | `CurrentSessionChanged`, `MediaPropertiesChanged`, `PlaybackInfoChanged` | no session = `available: false`, the tile shows "Not playing" |

All of them sit behind the control center switch (`controlCenterMode`, CLAUDE.md rule 11): in `windows`
mode their snapshots are emptied and the bar opens Quick Settings.

## The helper

Each admin-only action gets its own fixed control code and its own test; nothing takes free-form input. Today only energy saver needs it (night light and airplane mode work without admin). The installer (v0.8) should run the same steps as `scripts/helper/install.ps1`.

## Later

- `Media.source` is `"system"`. The Spotify web connection (sign in with a Spotify account in the v0.8
  settings, for music playing on another device) becomes a second source behind the same snapshot.
- If a new Windows build changes the night light layout, add the new layout to `night_light.rs` next to
  a real sample and a test; do not loosen the checks.
