# 0006: Status items in the registry and what the UI receives

Status: accepted (2026-11-09, v0.2 First item)

## Context

The registry holds every item; the UI (Liam's React) renders them (rule 5). We need: a key that
can't collide between apps, a format for icons that the UI can use directly, and a way to tell the UI about changes
without flooding it.

## Decision

**Keys.** Items live under (session, item id), so two apps can both create `"main"`. The UI sees one
string, `"s3/counter"` (session + `/` + id). Item ids can't contain `/`, so it is unambiguous.
`itemClicked(id, button)` takes that same string.

**Icons** travel as `data:` URLs (`data:image/png;base64,...` or `data:image/svg+xml;base64,...`):

|                      | `data:` URL in the message          | Path to a file              | Raw bytes (binary frames) |
| -------------------- | ----------------------------------- | --------------------------- | ------------------------- |
| UI can use it        | directly as `<img src>`             | needs the asset protocol    | needs conversion          |
| Works for any client | yes                                 | no (sandboxed apps, remote) | needs a second frame type |
| Size                 | +33% (base64), capped at 256 KB raw | none                        | none                      |

The bar checks the prefix, the base64, the size and the file signature (PNG header, or `<svg`/`<?xml`
for SVG). SVG is only safe as an `<img>` (scripts don't run there); the UI must never inline it into the DOM.

**Changes.** Every registry change pings a `Notify`. One task waits for it, sleeps 16 ms, and sends
`items-changed` with the **full list of visible items**. A client updating 1000 times a second makes the UI
redraw at most about 60 times a second, and a snapshot can never be applied out of order or half. Setting a field to
the value it already has is not a change. The UI also calls `getItems()` once on start.

**Update semantics.** In `item.update` a missing field means "unchanged" and an empty string clears it.

**Debug messages.** `debug.listItems` and `debug.click` let `examples/inspect` and the bench check the bar without
a UI. They exist in debug builds, or in release when the bar starts with `BARISTA_DEBUG` set. `debug.items` replies carry
`hasIcon` instead of the icon, to stay under the frame limit.

## Consequences

- Sending every icon in every event repeats big strings. At 8 items per app this is fine, and the bench
  (`docs/perf/0.2.md`) shows no cost. If it ever does, send icons once and refer to them by id (noted for v0.8).
- Item order is arrival order; drag ordering is v0.7.
- Hidden items (`visible: false`) stay in the registry but not in the snapshot.
