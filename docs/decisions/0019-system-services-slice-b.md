# 0019: Slice B services (display, network, radios, output, user) and what Windows won't let us do

Status: accepted (2026-11-12, v0.4b slice B). The output switching and airplane choices were made with Liam before the build.

## Context

The control center needs brightness, Wi-Fi, Bluetooth, airplane mode, hotspot, the output device list, a
user header and the bottom-row actions. Every service follows ADR 0016 (`getX`, `xChanged` once per frame
at most, `setX`, event-driven). This ADR records the Windows API behind each one, the limits we hit, and
what the UI shows where there is no public API.

## The services

| Service | Windows API | What wakes us | Commands |
|---|---|---|---|
| Brightness | WMI `WmiMonitorBrightness` (laptop panel), DDC/CI `dxva2` (external) | `WmiMonitorBrightnessEvent` (a thread blocks in `Next`); DDC: monitors changing (`WM_DISPLAYCHANGE`), our own set, the control center opening | `setBrightness(id, level)` |
| Wi-Fi | WLAN API | `WlanRegisterNotification` (ACM source) + radio `StateChanged` | `setWifiEnabled`, `scanWifi`, `connectWifi`, `disconnectWifi` |
| Bluetooth | `Windows.Devices.Radios` + `DeviceWatcher` (paired, classic and Low Energy) | `Radio.StateChanged`, watcher added/updated/removed | `setBluetoothEnabled` |
| Airplane mode | registry `RadioManagement\SystemRadioState` | `RegNotifyChangeKeyValue` | none (see below) |
| Hotspot | `NetworkOperatorTetheringManager` | our own start/stop, `NetworkStatusChanged`, the control center opening | `setHotspotEnabled` |
| Output devices | Core Audio list (in the audio thread of ADR 0016) + `IPolicyConfig` | `IMMNotificationClient` (added, removed, state, default) | `setOutputDevice(id)` |
| User | `Windows.System.User` (name, 208 px picture), `GetUserNameExW` fallback | start, the control center opening | none |
| Actions | `LockWorkStation`, `SetSuspendState`, `ExitWindowsEx`, `ms-screenclip:` | n/a | `runAction`, `openSettings` |

All services are verified live (see "What was measured"). The slice B work stays small: each service is a
`system/<x>.rs` (types, mapping, tests) over a `platform/windows/<x>.rs` (one thread, callbacks only send a
message to it, it re-reads the whole state).

## Decisions

- **Output switching uses `IPolicyConfig` (undocumented), with a fallback.** There is no public API.
  EarTrumpet and SoundSwitch have used this COM interface since Windows 7. We declare only the vtable (the
  method order is what matters) and ask for it by its exact IID. If Windows removes it, creating it fails,
  `canSwitch` is `false` and `setOutputDevice` opens `ms-settings:sound`. The id from the UI is checked against
  a fresh device list on the audio thread before it reaches COM; anything else opens the sound settings.
  Worked on this machine (Windows 11 build 26300): the default moved and the volume service followed.
- **Airplane mode is read, not switched.** The state is a documented registry value, watched without polling.
  Switching has no public API, and "turn every radio off" would leave Windows' own flag off so the two could
  disagree. The UI opens `openSettings("airplaneMode")`.
- **Wi-Fi joins only networks Windows already knows** (`WlanConnect` with the saved profile). For a new network,
  an unknown name or an over-long name, `connectWifi` opens `ms-availablenetworks:` (Windows' own flyout):
  creating profiles and asking for passwords is out of scope.
- **Windows 11 24H2+ hides network names from apps without location access.** The WLAN call then returns
  access denied; the snapshot has `locationAllowed: false` and the UI shows "allow location" which calls
  `openSettings("location")`. (Names were readable on this machine.)
- **Radio access** (`RequestAccessAsync`) is asked once. A desktop app got `Allowed` without a prompt here. If
  it is ever denied, `accessDenied` is true and the UI opens the settings page instead of toggling.
- **Bluetooth devices** are listed with a `connected` flag. Connecting or disconnecting one device has no
  public API: the UI opens `openSettings("bluetooth")`.
- **Hotspot has no state event.** We read at start, after our own start/stop, when the network connection
  changes and when the control center opens (an event, not a poll). A change made in Windows' settings while
  the panel stays open shows at the next open. `supported` is false when Windows says this connection can't
  be shared. Starting takes seconds: the thread publishes `changing` first.
- **Brightness `supported: false`** means the display did not answer or took more than 1.5 s. The laptop
  panel is matched to its WMI entry by PnP id so it is never listed twice through DDC. An external monitor's
  own buttons are not seen until the next read (no DDC event exists). A set burst (a slider) keeps only the
  last value per display, and after a DDC write we trust the monitor instead of reading back over the cable.
  The level is never set below 1%: 0 can leave a screen black.
- **Destructive actions need `confirm: true`.** `decide(action, confirm)` is a pure function and the only
  tested part; the executor (which really locks or shuts down) is never run in a test. Restart and shut down
  ask apps to close and let them cancel (no `EWX_FORCE`), so unsaved work can stop them.
- **`openSettings` takes an enum**, not a string, so the UI can't make the bar launch an arbitrary URI.
- **User picture** is passed on only if it really is a JPEG or PNG under 512 KB.
- **The control center is switchable** (rule 11): `controlCenterMode` (`barista` | `windows`, default `barista`).
  In `windows` mode the slice B services publish nothing (snapshots go empty), their threads stay parked in
  `recv()`, and `openWindowsControlCenter` opens Windows' Quick Settings. No UI until v0.8
  (`releases/v0.8.md` has the entry).
- **One dependency added:** `windows-collections` 0.3 (the WinRT list types the device watcher needs). It was
  already in the tree through `windows`, so Cargo still builds one copy.

## No public API: what the UI shows instead

| Wanted | Why not | UI shows |
|---|---|---|
| Airplane mode switch | no API (see above) | real state, click opens the airplane mode page |
| Focus / Do Not Disturb toggle | no API | pill opens `openSettings("focus")` (already so in ADR 0018) |
| Night light toggle | no API | tile opens `openSettings("nightLight")` |
| Battery Saver toggle | no API (state is in `getBattery`) | tile shows the state, click opens `openSettings("batterySaver")` |
| Join a new secured Wi-Fi network | passwords, profiles: out of scope | Windows' flyout (`ms-availablenetworks:`) |
| Connect one Bluetooth device | no API | `openSettings("bluetooth")` |
| Per-app volume mixer | out of scope | later |
| Seeing a monitor's own buttons | DDC has no event | refreshed when the panel opens |

## What was measured (this machine)

Wi-Fi, Bluetooth and brightness switched live (and restored) on the laptop panel and one external monitor
over DDC/CI; the output device switched and came back; the hotspot went off, changing, on, off; the settings
page and the screenshot opened; lock showed the lock screen; restart, shut down and sign out refused without `confirm`.
Sleep was not run (wake timers are off on this PC, so it would have stopped the session). Airplane mode was flipped by hand in Windows settings while `pnpm inspect --system --watch` ran: `airplane` followed within a second and Wi-Fi and Bluetooth went off and back on with it. Paired devices on
this machine are Low Energy only, which is why the watcher asks for both protocols. Idle CPU and memory are in
`docs/perf/0.4b.md`.

## Consequences

- Several small threads (radios, WLAN, brightness plus its WMI event thread, hotspot, user reads on demand),
  all blocked in a wait while nothing happens.
- `IPolicyConfig` is the one undocumented interface in the bar. It is isolated in
  `platform/windows/audio_policy.rs`, with the fallback above.
- Protocol stays 0.4: nothing here is a client-facing message. `debug.system` reports the new services.
