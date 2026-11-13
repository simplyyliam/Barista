# 0022: Per-app volume (audio sessions)

Status: accepted (2026-11-13). Liam asked for a volume popover with a slider per app, next to the master volume and the output devices.

## Context

The volume item in the bar opens a panel with the master slider, one slider per app that is using
audio (what Windows' volume mixer shows) and the output devices. Windows has a public, documented
API for this: Core Audio *sessions* on the default output device. Nothing here is undocumented
(unlike night light or output switching).

## Decision

- **Where.** `platform/windows/audio_sessions.rs`, on the audio thread that already owns the volume
  and device COM objects (`audio.rs`). Windows' callbacks (`IAudioSessionNotification` for new
  sessions, `IAudioSessionEvents` for volume, state and disconnect) only send a message to that
  thread, which re-reads (calling COM from inside a callback can deadlock). No polling (rule 9).
- **One app = one slider.** A browser has many sessions, so sessions are grouped by program file
  (its full path, so two programs with the same file name stay apart); the loudest session sets the shown level, the app is muted when all are, and a
  change is applied to every session of the app. Playing apps come first. Expired sessions and the
  system sounds session are left out, and so is the bar itself.
- **Name and icon.** The name is the file's description ("Google Chrome"), else the file name; the
  icon comes from the shell for the program file (`app_icon.rs`), cached per file as a `data:` PNG.
  A process we cannot open (a protected one) shows as "Unknown app", told apart by its pid.
- **IDs and validation (rule 3).** The id is the lower-case path of the program file. The UI sends it back; the
  audio thread only acts on an id that is in the list it read itself, never on the string as it is.
  Levels are clamped and NaN is refused; ids must be short and printable.
- **Surface.** `getAppVolumes`, `setAppVolume`, `setAppMuted` and the `appVolumesChanged` event
  (full snapshot, at most once per frame), like the other system services (ADR 0016). The list is
  also read when the `volume` panel opens.
- **Switchable (rule 11).** The service publishes only while the control center is ours
  (`controlCenterMode`); in `windows` mode the bar's volume item opens Quick Settings instead.

## Consequences

- The names of apps that use audio reach the bar's UI only; nothing leaves the machine.
- If a Windows build stops reporting sessions, the list is empty and the panel says "No app is using
  audio"; the master slider and the devices keep working.
- Not measured yet: memory and CPU with the session registrations (add to `docs/perf/`). The
  registrations are one per live session, so the cost grows with the number of apps using audio.
