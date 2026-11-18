# 0033: WebView2 memory flags

Status: **proposed** (2026-11-18, v0.8; becomes accepted when Liam merges the PR)

## Context

The release bar measured 156 MB private (bench). Budget is 120 MB
(130 MB accepted in v0.4b). The bar already ran with `--disable-gpu` and `--renderer-process-limit=1`, yet it still had 8
processes: browser, GPU, two utilities, crashpad and two renderers.

## Decision

Add to `additionalBrowserArgs` (`tauri.conf.json`, repeated by every extra window as WebView2 requires):

- `--in-process-gpu`: the (software) GPU work runs inside the browser process, so the GPU process is gone.
- `--disable-site-isolation-trials` and `IsolateOrigins,site-per-process` in `--disable-features`: the app only loads its own
  local pages, so per-site renderer processes buy nothing here.

## Consequences

- Measured on the release build (private working set, same number as `pnpm bench`): bar alone 156 to 143 MB (bench, two runs
  143 and 144), working set sum 541 to 456 MB, processes 8 to 6. The settings window costs about 28 MB on the Menu bar page and about 6 MB on About, and gives it back on close (148.5 MB).
- Still over the 120 MB budget; v0.4b accepted 130 MB. The next step would be fewer or lazier hidden windows, not more flags.
- Menu click to window visible moved from 43 to 48 ms median in one run and 50 ms in another: inside the run-to-run noise
  (43 to 50 ms across builds), but worth watching.
- Not tested: a crash of the shared process now takes every window with it (it already did with the renderer limit).
- `scripts/settings-memory.ps1` measures the bar alone and with the settings window (`BARISTA_OPEN_SETTINGS=1`, read once at
  startup, only to open the window for the measurement).
