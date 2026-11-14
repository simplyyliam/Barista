# 0015: Icons as objects, cropped to their glyph by the SDK

Status: accepted (2026-11-11, icon sizing fix)

## Context

Icons looked small in the bar. Lucide and Hugeicons draw in a 24-unit box with about 2 units of empty
border, so a 16 px box shows a glyph about 13 px tall; a wide shape such as a battery in a square box shows
about 9 px. The target (macOS): a bar 24 high, an icon area of 22, square glyphs about 16 px, wide ones sized by
height (a battery about 24 x 12). Apps also had to hand-write SVG strings, because `setIcon` only took bytes.

## Decision

- **Icon objects.** `setIcon` and `StatusItem.create({ icon })` accept an icon object, `{ icon, color, strokeWidth,
  height }`, SVG text, bytes or a `data:` URL. An icon object is the `[tag, attributes][]` list that the `lucide`
  package and `@hugeicons/core-free-icons` already export, so `import { Clock01Icon } from "@hugeicons/core-free-icons"`
  works with no SVG. The SDK imports no icon library.
- **Safe conversion.** `iconToSvg` only emits plain shapes (`path`, `circle`, `rect`, `line`, `polyline`, `polygon`,
  `ellipse`, `g`) and plain attributes (no `on*`), escapes values and rejects a bad colour, so an icon object cannot put
  markup into the bar.
- **Fit in the SDK, not in the bar.** `fitToGlyph(svg, height)` measures the real bounds of the drawing (stroke included)
  with resvg, makes them the `viewBox`, and sets `height` and an aspect-correct `width`. Icon objects go through it
  automatically at 16 px (`ICON_SIZE`); wide icons use `WIDE_ICON_HEIGHT` (12). The size constants live in
  `@baristabar/sdk/icons` (one place to tune); `ICON_AREA` (22) caps anything taller.
- **resvg is an optional peer dependency**, loaded lazily. Apps that never import an icon object need nothing. Without
  it, icon objects still work but keep their border (and a warning says how to fix that, once). The core SDK stays
  light; the native module is only for apps that opt in.
- **Sharpness.** SVG stays vector, so the bar's webview draws it sharp at any scale. `toPng(svg, { height, scale })`
  makes a PNG at `height * scale` pixels for apps that want one; the SDK does not know the monitor scale, so the
  caller passes it.

## Consequences

- The bar's own icons (`apps/bar/src/`, the user's frontend: the `--spacing-bar-icon` token, `battery.tsx`, `volume.tsx`)
  have the same padding problem and are not touched here. The UI should size item images by height, not in a square box.
- Considered and not done: cropping in Rust when the bar receives an icon (works for every client, but adds a
  rendering crate and image work to the bar, and needs the UI to agree on sizes). Revisit if non-JS clients matter.
