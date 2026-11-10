# 0011: Menu state lives in the bar; only ids go over the wire

Status: accepted (2026-11-10, v0.3 Menus)

## Context

Apps send an Electron-style template (`item.setMenu`). Someone has to own checkbox and radio state,
decide where standard entries go, and map clicks back to code.

## Decision

- **Wire:** the template minus callbacks (`MenuTemplateItem`). Every entry but separators needs an id,
  unique in the whole tree. The SDK makes up ids (`menu-1`, ...) for entries without one. Limits:
  depth 3, 50 entries, accelerator 32 chars, icons 16 KB (`invalid_menu` otherwise).
- **Clicks:** the UI calls `menuItemClicked(itemId, menuItemId)`. The bar applies it, then sends
  `menu.click { id, menuItemId, checked? }`. The SDK finds the `click` callback by id.
- **Checked state in the bar.** A checkbox flips, a radio checks itself and unchecks its group (Electron's
  rule: adjacent radio entries). The bar redraws at once (`menu-changed`) without waiting for the app,
  and the app learns the new state in `checked`. The SDK mirrors it so `menu.getItemById(id).checked`
  stays true to what the user sees.
- **Roles:** `about` and `settings` move to the top, `quit` to the bottom, with separators, and get
  default labels ("About <app>", "Settings…", "Quit <app>") if the app gave none. Only top-level roles
  move. `quit` without a `click` makes the SDK remove its items and exit (or call `bar.on("quit")`).
- **Clicks on an item with a menu** open the menu (left or right button); the app gets no `item.click`.
  Middle click still does, so apps keep one free gesture.
- **Accelerators** are text only: the SDK shows `CmdOrCtrl` as `Ctrl`. No global hotkeys.

## Consequences

- Every app gets the same radio and role behaviour; a slow app can't make the menu feel laggy.
- If an app changes `checked` itself, it must `setMenu` again (noted in "Fix next").
- Protocol 0.3 is backward compatible: 0.2 clients never send the new messages.
