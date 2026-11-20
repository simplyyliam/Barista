# 0037: The website runs the real bar UI

Status: **proposed** (2026-11-19, v0.10; becomes accepted when Liam merges the PR)

## Context

The landing page showed the bar, notifications, control center and tray as hand-made CSS copies. They drifted
from the product, and Liam wants the site to sell "the look and feel of what the user will get", so everything
on it should be the real thing (also noted for v1.0 in `docs/release/releases/v1.0.md`, moved into v0.10).

CLAUDE.md says nothing imports from `apps/`. The usual answer is to move the presentational components into a
shared package. That edits the bar's frontend wholesale, and the bar's components are written against
`@/shared/bindings` (generated Tauri calls), so they would need extracting and re-wiring before the site
could use them.

## Decision

- **Build the real UI a second time, for the browser.** `apps/bar/vite.demo.config.ts` builds
  `apps/bar/demo/` into `apps/web/public/demo/` (git-ignored, built by `pnpm web:build` and `pnpm web:dev`).
  The page imports the same widgets and features as the app (`Menubar`, `PanelBody`, `RootMenu`), so the site
  always shows the current bar. The bar's code is not edited for this.
- **Mock the IPC underneath, not the bindings.** `@tauri-apps/api/mocks` (`mockIPC`, `mockWindows`) answers
  the real generated `commands` and `events` (`demo/backend.ts`). Because it sits below `bindings.ts`, a changed
  command shape fails `pnpm typecheck` in the demo instead of breaking the site quietly. The backend keeps
  real state (toggles, sliders, media, notifications, a focus timer, tray pins) and emits the same events Rust
  would, so the real components redraw as they do in the app.
- **Fixtures come from Rust.** The default settings are written to `demo/settings.json` by a Rust test
  (`export_demo_settings`, run by `pnpm gen`); CI's `git diff` check catches a stale file.
- **Frame, don't embed.** The landing page shows the demo in an `<iframe>` (`landing/bar-demo.tsx`): the bar's
  global Tailwind and tokens cannot clash with the site's CSS, and the page scales to the frame, so phones get
  the same picture. A panel or menu is a window in the app, so the demo gives each a box of the size it
  reported and places it from the clicked item's position, like Rust does.
- **Toys stay outside the product.** The counter's rolling digits, press scale and confetti
  (`demo/counter-fx.ts`, `demo.css`) are demo-only DOM effects. The bar's own motion rules
  (ADR 0023) are not changed.

## Consequences

- The site is only as good as the mocks: a new command the UI calls gets a `console.debug("demo: no answer
  for ...")` and does nothing until `backend.ts` learns it.
- The demo bundle adds about 360 kB (117 kB gzipped) to the site, loaded lazily per frame and cached across frames.
- CLAUDE.md's "nothing imports from `apps/`" still holds for packages; the demo is a build target of the
  bar app, not an import.
- Still hand-made on the landing page: the Monitors tile, the Settings sliders and the Windows-mode switches
  (v0.10.x: the real settings window components).
