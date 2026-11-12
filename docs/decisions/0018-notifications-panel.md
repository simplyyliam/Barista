# 0018: The notifications panel (listener + database + folder watch)

Status: accepted (2026-11-12, v0.4b slice C). Builds on ADR 0017 (the spike). Liam chose the database
option and the settings switch.

## Context

ADR 0017 showed that `UserNotificationListener` works for an unpackaged app but gives text only and no
events. Liam wants the panel from `docs/design/App.png` (notifications with pictures, calendar, Focus)
to replace Windows' notification center, with Windows' own Do Not Disturb turned on by him so banners
stop. He accepted reading Windows' own notification database for the parts the API hides.

## Decision

```text
Windows writes in %LOCALAPPDATA%\Microsoft\Windows\Notifications
   └► folder watcher thread (FindFirstChangeNotification, sleeps until Windows says so)
        └► fingerprint: ids + arrival times from wpndatabase.db (read-only, a few ms)
             same as last time ─► ignore            different ─► worker refresh (debounced 250 ms)
worker: listener list (ids, app, text) + database XML (picture, attribution, buttons) ─► snapshot
        ─► notificationsChanged (at most once per frame, like the other system services)
```

- **Two sources merged by toast id.** The listener gives the truth about which toasts exist, who sent
  them and their text. The database XML adds the picture path, the attribution line and button labels.
  If the database can't be read or parsed, the panel shows text only. It is never written to.
- **The fingerprint is not optional.** Reading through the listener writes to the same database (the
  WAL grew about 1 MB per read), so "refresh on every folder write" fed itself at about 3% CPU. The
  watcher now asks the database whether the set of toasts really changed and only then reads.
- **Windows signals removals late.** A removal by another program may never raise a folder event
  within 30 s (arrivals do, in about 1.2 s). Opening the panel therefore always reads fresh, and our
  own dismissals update the snapshot immediately.
- **Images stay out of the snapshot.** `hasImage` says there is one; `getNotificationImage(id)` returns
  a data URL on demand. App icons come from the shell (`shell:AppsFolder\<AUMID>`, works for packaged and
  unpackaged apps) through `getNotificationAppIcon(appId)`, cached.
- **Nothing an app wrote is trusted.** Text is clipped (200 / 500 chars, 8 lines, 100 toasts). Picture
  paths must be `file:///` or a local drive path, or `ms-appdata:///` of a packaged app; web addresses,
  `\\server\share` paths (they would send the user's login to that server), relative paths and `..` are
  refused. Files are read only when they are 4 MB or less and start like PNG, JPEG, GIF or WebP. The
  app id handed to the shell has to be printable and short, and is only taken from a toast we listed.
- **A click opens the sender, then removes the toast.** `openNotification(id)` starts or focuses the app by
  its AUMID. Reply and Snooze buttons are not supported (we can't send an activation back to the app).
- **Dismiss and clear remove the toast from Windows too**, so its own list stays empty.
- **Unread dot** next to the date: something arrived after `notificationsSeenAt` (saved in
  `settings.json`). Opening the panel marks everything seen, and so does something arriving while it is open.

## The panel window: chrome `none`

The design's cards float with gaps, so the panel window can now be a bare transparent window (no
backdrop, no rounding, no shadow): `openPanel(name, anchor, w, h, chrome)` with `chrome: "glass" | "none"`.
`glass` is the menu look from ADR 0016. A bare panel is centred under its item and 14 px below the
bar (`--spacing-panel-offset`). The cards use the window radius (`--radius-panel`), and the containers
inset by 8 px inside them (calendar, picture) use the formula `--radius-panel-child` (ADR 0008).

## Every replacement is switchable (CLAUDE.md rule 11)

`settings.json` has `notificationsMode: "barista" | "windows"`. In `windows` mode the reader ignores
events and reads nothing, the snapshot is empty, there is no dot, and clicking the date opens Windows'
own notification center (`ms-actioncenter:`). Commands: `getNotificationMode`, `setNotificationMode`,
event `notificationModeChanged`. No UI for it until the v0.8 settings window.

## Edge cases (Liam's list)

| Case | What happens |
|---|---|
| Do Not Disturb on | Toasts still reach Windows' list, so we read them. Apps that skip non-urgent alerts when they see DND send nothing, and we can't see what was never sent. |
| Alarms, calls, priority apps | Windows still draws their banner. We draw no banners, so there is no double banner: they also appear in our list. Nothing to filter. |
| Access off or never granted | `access` is `denied` or `unknown`: the card says so and "Open settings" opens `ms-settings:privacy-notifications`. |
| Windows changes the database | Pictures and attribution disappear, text keeps working (tested: unreadable database = text only). Fix the query. |
| Banners | Still drawn by Windows. DND is the user's switch, we never toggle it. |

## Consequences

- New dependencies: `rusqlite` (SQLite built in, read-only use) and `roxmltree` (toast XML). Both credited.
- The reader costs 0.00 s CPU over 12 s at idle (debug build), and the same as Windows mode.
- The "Focus" pill only opens Windows' Focus settings. A Barista focus timer is a later release.
- Undocumented database: this is the one part that may break with a Windows update.

## Layout, prebaking and motion (2026-11-12)

- **One notification per block, each with its own app header** (like WhatsApp and Discord in App.png),
  newest first. The picture is always 16:9, so picture notifications are all about the same height. The list is
  at most as tall as the tallest one (measured, `data-picture`), so one is in view and the next is a
  scroll away (`scroll-snap`, proximity); text-only lists are capped at 240 px and packed with as many as
  fit. The list never has a minimum height: one notification makes the card shrink to fit it, with no
  empty space under it. Never taller than the screen allows.
- **Prebaked.** The panel window is hidden, but its page keeps the notifications route mounted (out of
  the layout, invisible). The snapshot, app icons and decoded pictures (`Image.decode()` before they are
  shown) are already there when you click, so the panel opens with nothing to load. A hidden panel never
  marks anything as read: only an open one does (`active`).
- **One window, no motion in the page.** The SDK popovers (quick-note, Spotify) are opaque, frameless
  windows that the app shows, so Windows gives them its own open animation. The panel has to be
  transparent for the floating cards, and measured with a screen burst capture Windows does not animate
  such a window: it appears in one frame. A first version faked motion in the page (fade, slide, scale
  over 260 ms); it felt staggered, so it was removed. Now the window appears whole, at its final size and
  place, already painted: 73 to 115 ms from the click to fully shown in a debug build, in a single frame.
  Nothing changes after it shows: the calendar is reset while the window is hidden, not on open, and the
  size is read with `offsetWidth`/`offsetHeight`. Closing keeps Windows' fade.
- **One beat, not "top then bottom".** A real-click screen capture showed every open as two frames:
  the top (down to the 400 px *guess* height) and, 50 ms later, the rest. The cause was resizing the hidden
  window around the open: WebView2 resizes asynchronously. Now `sizes` remembers each route's last
  measured size and the open places the window at that size (no resize), the startup warm-up lets the page
  measure itself so even the first open needs no resize, and if the size really changed Rust waits 40 ms
  before showing. The safety-net reveal is 150 ms (it was 20 ms, sooner than the page can answer).
- **Warm-up.** The first show of a window is slower (about 200 ms against 70 ms). At startup Rust shows
  the panel cloaked (invisible, no focus) for about 0.5 s while the page paints its content (`panelWarm`).
  Measured on real clicks: a single frame that already covers the whole window, from the first open on.
