# 0017: Reading Windows notifications (spike result)

Status: accepted (2026-11-12). Liam chose to build the panel; the plan became ADR 0018, which also reads the database for pictures.

## Context

The design has a notifications panel. The only public way to read other apps' toasts is WinRT
`UserNotificationListener`. It was made for packaged apps, and Barista is unpackaged, so the spike
(`examples/notifications-spike`, not part of the workspace) tested it for real.

Tested on this machine: Windows 11 Home build 26300, unpackaged `.exe` (no package identity), 24 real
toasts in the Action Center, plus my own test toasts sent as Windows PowerShell.

## What we saw

| Question | Result |
|---|---|
| Works without package identity? | **Yes.** `GetAccessStatus` = `Allowed` straight away, `RequestAccessAsync` = `Allowed`, `GetNotificationsAsync(Toast)` returned all toasts. |
| Consent prompt? | **None seen.** The global switch (`CapabilityAccessManager\ConsentStore\userNotificationListener`) is `Allow`, and there is no per-app entry for desktop apps. Not known whether it was on by default or Liam enabled it once ("not sure"), so a fresh machine may say `Unspecified` or `Denied`. The bar must handle all three states (see below). |
| Id | Yes (`u32`, stable while the toast stays in the Action Center). |
| Creation time, expiry | Yes. |
| App name and app id (AUMID) | Yes, for packaged **and** unpackaged senders (`GitHub Copilot`, `Notion`, `WhatsApp`, `Settings`, `Windows PowerShell`). |
| App icon | **Only for packaged apps** (3 of 24 gave a PNG). For unpackaged senders (21 of 24) `AppInfo.DisplayInfo.GetLogo` returns null. |
| Title and body | Yes: `ToastGeneric` text elements (`text[0]` title, `text[1]` body, more lines if the sender has them). |
| Attribution line | No (my `placement='attribution'` text was not returned). |
| Hero / inline images, app logo override | **No.** The API only exposes text elements. |
| Action buttons, reply boxes, click activation | **No.** Buttons are not exposed and a toast can't be activated from the listener. |
| Dismiss one | **Yes.** `RemoveNotification(id)`: count went 26 -> 24 after two removals (it also removes it from Windows' Action Center). |
| Clear all | `ClearNotifications()` exists. Not run on purpose: it would delete Liam's real notifications. Same API family as the working dismiss. |
| Live changes | **No.** `NotificationChanged` fails to register with `0x80070490 Element not found` for an unpackaged app. Only polling `GetNotificationsAsync` works. |
| Windows' own banners | **Still shown.** A test toast popped up as normal. Reading and removing doesn't hide banners, and no API hides them for an app that isn't the shell. (I observed the banner without the listener running, and there is no API call that could suppress it, so the result is the same either way.) |

## The problem this leaves

1. **No events, so polling.** Rule 9 says no polling where a Windows event exists. Here none exists
   for us. Options inside the limit: poll only while the notifications panel is open (cheap, one
   call) plus a slow poll (for example every 30 s) for the bell badge count, or no badge.
2. **No icons for most senders.** The UI needs a fallback (the app's initial letter), or we resolve
   the exe icon ourselves from the AUMID (not reliable).
3. **Text only, no buttons.** A notification in our panel can be read and dismissed, not answered.
   Clicking it can only open the sender app (we can't).
4. **Banners stay.** A 1:1 macOS replacement (our panel replaces banners) isn't possible.

## Options

| | What | Cost | Verdict |
|---|---|---|---|
| **A. Use the API as it is** | Poll while the panel is open, slow poll for the badge, text only, letter icons | Small, no setup | **Recommended for v0.4b** |
| B. Sparse package identity | Sign a sparse MSIX and register it at install: likely fixes `NotificationChanged` and icons | Needs a code-signing certificate users trust, installer changes, `ms-appx` identity quirks (touches the updater, ADR needed) | Later, only if polling proves too weak |
| C. Hide banners with Focus assist | Turn on Do Not Disturb so only our panel shows | No public API to toggle it (only Settings, or undocumented registry/WNF), and it hides banners for every app, so we would be changing the user's focus state for them | Don't |
| D. Notification API in the SDK | `BA.Notification` for our own clients (v0.8+): full control, buttons, images, events | New protocol messages, only covers apps that use our SDK | Add later as an extra, doesn't replace A |

## Decision (proposed)

Go with **A**: a Rust `notifications` module that holds a snapshot, `getNotifications()` (reads
now), `notificationsChanged` (emitted when a poll finds a difference), `dismissNotification(id)`
and `clearNotifications()`, grouped by app id in the UI. Poll only while the panel is open and on
a slow timer otherwise (measure in `docs/perf/` before choosing the interval). Report access as
`allowed | denied | unknown` so the UI can show "turn on notification access" and open
`ms-settings:privacy-notifications`. Windows' banners stay, and the UI shows our panel as "all
notifications", not as a banner replacement.

## Consequences

- No sparse package, no signing, no new dependency (the `windows` crate gets four more features).
- The panel is a read-and-dismiss list. B or D can add richness later without redoing A.
- Open for Liam: is a read-only, banner-less panel still wanted, or is the notifications panel
  better left until D (SDK notifications) exists?
