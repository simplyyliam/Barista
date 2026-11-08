# 0002: Reserve screen space with an AppBar

Status: accepted (2026-11-08, v0.1 Bar shell)

## Context

The bar must reserve the top of the screen like the macOS menu bar: maximized windows stop under
it, and the space comes back when the bar is gone, even if it crashes.

|                                               | AppBar (`SHAppBarMessage`)  | Change the work area (`SPI_SETWORKAREA`) | Topmost window only          |
| --------------------------------------------- | --------------------------- | ---------------------------------------- | ---------------------------- |
| Maximized windows stop under it               | yes                         | yes                                      | no, they slide under the bar |
| Cooperates with the taskbar and other appbars | yes (Windows arranges them) | no, fights the taskbar                   | n/a                          |
| Space returned after a kill                   | yes (tested, see below)     | no, the work area stays changed          | n/a                          |
| Per monitor                                   | yes                         | primary monitor only                     | yes                          |

## Decision

Use an **AppBar** on the top edge.

- **Register once, set position on every change.** `ABM_NEW`, then `ABM_QUERYPOS` + `ABM_SETPOS`
  with our own height restored after the query (the query only moves the top edge). The window is
  then placed in the rectangle Windows granted.
- **Messages through a window subclass** (`SetWindowSubclass`), in front of Tauri's own handler.
  We handle `ABN_POSCHANGED`, `ABN_FULLSCREENAPP`, `WM_DISPLAYCHANGE`, `WM_DPICHANGED` and
  Explorer's `TaskbarCreated`, and forward `WM_ACTIVATE` and `WM_WINDOWPOSCHANGED` as Windows asks.
  For `WM_DPICHANGED` Tauri runs first, then we put the window back to our rectangle.
- **One conversion point for DPI.** Height is logical in settings, converted to physical pixels
  once in `bar_rect`. Everything at the Win32 boundary is physical.
- **Monitor choice.** The saved monitor if plugged in, otherwise the primary. The saved id is never
  overwritten by a fallback, so the bar moves back when the monitor returns. Ids are the display's
  device path, which survives replugging better than `\\.\DISPLAY2`.
- **Full-screen hide, without polling.** `ABN_FULLSCREENAPP` (the shell's own signal, covers F11 in
  a browser) plus a foreground-change hook (`SetWinEventHook`) that checks if the front window covers
  the whole monitor. Desktop and taskbar windows are ignored. The reservation stays while hidden.
- **Re-entrancy.** Win32 calls from `apply` can call back into our window procedure. The state
  lock is dropped before any Win32 call, and a flag turns nested applies into one more loop.

## Cleanup after exit, panic and kill

- Normal quit: `ABM_REMOVE` on `RunEvent::Exit`.
- Panic: a panic hook sends `ABM_REMOVE` first (atomic hwnd, no locks).
- **Kill (`Stop-Process -Force`):** tested on Windows 11. The work area top went from 0 to 48 px
  (32 px at 150%) while running and back to 0 within 3 seconds after the kill. Explorer notices the
  dead window by itself, so **no watchdog process is needed**. Recheck on Windows 10 (risk file).

## Consequences

- If Explorer restarts, we register again (`TaskbarCreated`).
- Another top appbar pushes the bar down; we accept the rectangle Windows grants.
- The bar's window never takes focus (`WS_EX_NOACTIVATE`) and stays out of Alt+Tab (`WS_EX_TOOLWINDOW`).
- One bar per process and one monitor in v0.1; a bar per monitor is v1.2+.
