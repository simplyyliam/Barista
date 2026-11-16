# 0028: Item layout and overflow

Status: **proposed** (2026-11-16, v0.7; becomes accepted when Liam merges the PR)

## Context

The bar needs a user-controlled arrangement: which items stay visible (pinned), in what order, whether the
rest are collapsed, and what happens to items that do not fit. It has to work for client items and tray
icons alike, and survive a restart. Rule 5 says Rust holds the logic and the UI only renders.

## Decision

- **Rust decides, the UI draws.** `items-changed` stays one list, now in **display order**, and every item
  gets two fields: `pinned: boolean` and `placement: "bar" | "overflow"`. Nothing in the old payload moves,
  so the current UI keeps working; it switches to the new fields when it is ready.
- **Saved in settings** as `itemLayout { order: string[], pinned: { id: bool }, collapsed: bool }`, cleaned
  on load (512 entries, 300 characters per id, duplicates dropped).
- **Stable ids**, because the runtime keys change every run (session numbers, window handles):
  client items `app:<app name>/<item id>`; tray icons `tray:<guid>` when the app registered one, else
  `tray:<program name, lower case>/<icon id>`. Limit: two windows of one program that use the same icon id
  share a stable id; a program whose icon id is a pointer or counter that changes each run (G HUB's was
  `532175291`) loses its saved place at the next start. A friendlier id would need the executable path.
- **Defaults.** Client items start pinned (an app made them on purpose); tray icons start unpinned (there can
  be dozens). The bar starts collapsed. So by default the bar shows the client items and an arrow, and every
  tray icon is in overflow until the user pins it. Windows' own pins (`NotifyIconSettings`, `IsPromoted`) are
  not read: Liam chose "none pinned".
- **Collapsed** = one saved flag: every unpinned item (client or tray) is in overflow.
- **Order.** Sorted by position in `order`; items not in it go after, in arrival order (client items first).
  `setOrder(ids)` takes the ids the user can see and moves only those, within the slots they already held
  (`layout::merge_order`), so items that are not running keep their place. Unknown or repeated ids refuse
  the whole call.
- **Overflow by room.** The UI tells Rust how much room it has (`setItemSpace(width, gap)`, logical px, not
  saved) and Rust measures items from `setItemBounds` widths (remembered, so an item moved to overflow does
  not flip back and forth; a guess until measured). While the items do not fit, the left-most unpinned
  item goes to overflow, then the left-most pinned one. With no space set nothing overflows for lack of room.
- **Commands:** `setPinned(id, pinned)`, `setOrder(ids)`, `setCollapsed(collapsed)`, `getCollapsed()`,
  `setItemSpace(width, gap)`; event `collapsedChanged`. Each saves through `update_settings` only when the
  layout really changed, and wakes the UI only when order, pins or placements changed.
- **Pure core.** `registry/layout.rs` has no registry or Windows in it (sort, defaults, merge, split), tested
  with plain values; the registry only gathers entries and applies the result.

## Consequences

- One `items-changed` carries everything the UI needs; no second event for the split.
- A drag in the UI is one `setOrder` call with the full visible list.
- The layout file grows slowly with apps that come and go (capped at 512 ids; the oldest at the end drop).
- Moving where the date or other first-party items sit is a v0.8 setting, built on the same ids.
