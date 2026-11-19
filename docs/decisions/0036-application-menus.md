# 0036: Application menus in the bar

Status: **proposed** (2026-11-19, after v0.9; becomes accepted when Liam merges the PR)

## Context

macOS shows the menus of the app in front (File, Edit, View ...) in the menu bar. Liam asked for the same,
and it must really work for apps. Windows has no single API for another app's menus, so this was "hard,
research first" in the backlog.

The research (a probe of real windows on a Windows 11 machine, 2026-11-19):

| App style | What it exposes | Example found |
|---|---|---|
| Classic Win32 / MFC / WinForms menu | an `HMENU` (`GetMenu`), readable from another process without side effects | msinfo32, regedit, Notepad++, 7-Zip |
| Modern XAML / WPF / Electron with a UIA menu bar | a UI Automation `MenuBar` with `MenuItem`s. Only the titles exist until a menu is expanded (`ExpandCollapse` pattern); expanding takes about 200 ms and makes the app draw its own popup | Windows 11 Notepad: File, Edit, View; expanding Edit gave 16 items with names, enabled state and accelerators |
| Chromium / Electron with native-less UI, GPU-drawn apps, browsers | neither | Figma, Spotify, Zed, Zen: nothing to show |

So no approach reaches every app. Two providers cover the apps that do have menus.

## Decision

- A new module `src-tauri/src/appmenu/` follows the foreground window (the existing
  `EVENT_SYSTEM_FOREGROUND` hook: no polling, rule 9) and keeps a snapshot: the app's display name and its
  menu titles. Barista's own windows never count as "the app in front" (the bar's menu windows take focus
  when they open).
- **Provider 1, classic menus (`platform/windows/win32_menu.rs`):** `GetMenu` and `GetMenuItemInfoW` read
  the whole tree (titles, separators, check marks, disabled, accelerators after the tab, submenus) when the
  app comes to the front and again right before a menu opens. Before reading a submenu the app is sent
  `WM_INITMENUPOPUP` with a short timeout, which is what the app does itself when its menu opens, so its
  enabled and checked states are current. A click is `WM_COMMAND` posted to the app's window.
- **Provider 2, UI Automation (`platform/windows/uia_menu.rs`):** the `MenuBar`'s children give the titles
  cheaply. When the user opens a title, the menu is expanded through UIA, its items are read, and it is
  collapsed again (a brief flash of the app's own popup, accepted). A pick expands the same path again and
  invokes the item. All UIA calls run on one worker thread with timeouts, never on the bar's threads, so a
  slow or hung app cannot freeze the bar (rule 3).
- Provider 1 is tried first; if the window has no `HMENU` the UIA bar is tried; if neither, **a fallback**: an
  Edit menu (Undo, Cut, Copy, Paste, Select All, sent as the standard shortcuts to the app) and a Window menu
  (Minimize, Zoom, Close Window, via `WM_SYSCOMMAND`). The fallback is offered only for normal app windows (top-level, visible, no tool
  or no-activate style, not the shell's own windows), so Close Window can never be aimed at the Start menu or
  the taskbar.
- The menu is drawn by the bar's existing menu window (`MenuEntry`), keys `appmenu/<n>`, like script items.
  Menu content from another process is untrusted text, so it goes through the same limits as script menus
  (entry and depth caps, control characters stripped) and is only ever rendered as text.
- A click on a menu entry never runs anything the bar chose: it posts the command id (provider 1) or
  invokes the UIA element (provider 2) in the app the user was looking at, and only if that window is
  still the one the menu was read from.
- This replaces nothing in Windows (apps keep their own menus), so it has no `barista | windows` switch
  (rule 11). A setting turns the feature off (`appMenus.enabled`, default on).
- Commands and events for the UI are internal (`getAppMenus`, `appMenusChanged`), not SDK or protocol.

## Consequences

- Works for classic Win32 apps and for modern apps with a UIA menu bar. Does not work for apps that draw
  their own menus without exposing them (most browsers and Electron apps): the bar shows their name only.
- UIA menus flash the app's own popup for about 200 ms when opened, and cost that long to appear.
  Hover-switching between titles only works for classic menus (they are already read).
- Owner-drawn items (no text) are skipped; item icons are not shown.
- Not done: keyboard access (Alt, mnemonic keys), a "Help" search field, the system menu, menus of apps
  running elevated while the bar is not (Windows blocks the messages: those apps simply show nothing).

## Verified (2026-11-19, live on a Windows 11 machine)

- Classic: System Information (File, Edit, View, Help). "Close" showed greyed (the app's real state after
  `WM_INITMENUPOPUP`), and "Exit" closed the app (`WM_COMMAND`).
- UI Automation: Windows 11 Notepad. Edit gave 16 items with shortcuts in about 0.8 s, and "Select all"
  selected all the text in Notepad.
- Found on the way: a **minimized window shows no popup**, so it has no menu items for UIA (an app in front
  is never minimized); some browsers put the window's **system menu** (Restore, Move, Close) in the menu bar
  slot, which is filtered out (its commands are all `SC_*`, 0xF000 and up); the first UIA read must wait for
  the popup to appear (polling, up to 0.6 s).
- Found after Liam asked why no menus showed (2026-11-19): his everyday apps (Zed, Zen, Figma, Spotify) are
  Chromium or GPU-drawn, so providers 1 and 2 give nothing; that is why the fallback was added. Also: Windows
  exposes the title bar's system menu as a UIA `MenuBar` named "System" on every window, which is skipped.

## Update: UI Automation reading removed

Reading a modern app's menu (Notepad, WinUI/XAML) needs the app's own menu to be opened, because the items
only exist while it is expanded. In practice the app's popup opened next to the bar's menu and the two fought
over focus, so the bar's menu closed right after opening. Timing fixes, reading ahead and hiding the popup
(making it transparent) were all workarounds, so they were removed. It does not feel native.

Now: apps with a classic menu show their real menus (provider 1). Every other app gets the standard Edit and
Window menu (provider 3). Reading modern menus can come back when there is a clean way (an app-side API, or
a reliable non-visible read). Notes above about provider 2 describe the removed code.
