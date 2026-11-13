# 0021: Native feel

Status: **proposed** (2026-11-13). Needs Liam's decision before N3 starts. Nothing here is built yet.

## Context

Barista is a Tauri app: the bar, menus and panels are WebView2 pages. Liam wants it to feel like a
native product that stands next to SwiftUI's `MenuBarExtra`, not like a web page in a window, for
public users. Research and tiers: `docs/research/native-feel.md` (local).

## Options

| | A. Webview, remove the tells | B. A + native menus as a switch | C. Native bar strip, webview panels |
|---|---|---|---|
| Effort | days | A + about a week | weeks, plus a new workflow for the bar |
| Perceived native feel | most of the gap | more (menus are the OS's) | closest possible |
| Idle memory / start time | unchanged | unchanged | expected better, **to be measured** |
| Who writes the bar's look | Liam (React) | Liam | Rust (Claude) or a Rust UI layer |
| Risk | low | dark popup menus need undocumented calls | rewrite of working UI, slower look iteration |

## Decision (proposed)

1. **Do A now** (N1, N2 in `releases/native-feel.md`). No new architecture, so no further approval needed for the individual tells.
2. **Measure C** with a time-boxed spike (N3): a native strip with the same pills, clock and two icons, compared to the webview bar on cold start, hover latency, idle memory and CPU, written to `docs/perf/native-strip.md`.
3. **B** becomes a `menuStyle` switch (`barista` | `windows`, default `barista`, rule 11) in v0.8 if wanted.
4. Liam decides C from the measurements, before v0.10. This ADR then becomes accepted or rejected.

## Consequences

- If C is chosen: `apps/bar/src/` keeps the panels and popovers only; the strip moves to `src-tauri/src/platform/windows/` behind a small module, tokens stay in one file, and the design in `docs/design/` becomes the spec. Rule 5 still holds.
- If C is rejected: A stays, and the measured numbers go into `docs/perf/` as the baseline for the budget.
- Either way the tier 1 list becomes a checklist every new window must pass (N2 audit).
