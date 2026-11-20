# 0038: Distribution

Status: **proposed** (2026-11-19, v0.10; becomes accepted when Liam merges the PR)

## Context

Barista is free and open source (Liam, 2026-11-20; it was closed source and freemium on 2026-11-18). A stranger
has to download the installer, and an installed copy has to find updates, without needing a GitHub login. The Tauri updater only needs a URL it can
reach without credentials that returns a small `latest.json` and the signed update files.

## Decision

- **Releases live in the code repo, `simplyyliam/Barista`.** The release Action builds on a tag and writes a
  **draft** release on the same repo with the built-in `GITHUB_TOKEN` (no extra token). Liam still reviews and
  publishes the draft by hand. (The first plan was a separate releases-only repo so the code could stay private;
  dropped on 2026-11-20 when Barista became open source, before any release shipped.)
- **The updater endpoint is baked into every install,** so it must not change:
  `https://github.com/simplyyliam/Barista/releases/latest/download/latest.json`. Moving to another
  host later means shipping one more release that points at the new URL, from the old one.
- **Update signing is separate from code signing.** `pnpm updater:keygen` makes the key (`.private/updater/`),
  puts the public half in `tauri.conf.json`, and prints the two Action secrets. Until it has run, the updater
  is not started (it would stop the app launching) and the commands answer "not set up".
- **Updates never install on their own.** One check shortly after start, then every 6 hours (a sleeping timer,
  rule 9), a setting to switch it off, "Update now" in Settings > General. Installing stops the bar's services
  first (`shutdown_services`), because the Windows installer ends the process without a normal quit, then
  runs the NSIS installer in passive mode and starts the new bar.
- **Per-user install.** NSIS `currentUser`: no admin, nothing outside the user's folders. WebView2 comes from
  the Microsoft bootstrapper. The uninstaller stops the bar, runs `barista.exe --uninstall-cleanup` (gives the
  Windows taskbar back from the saved settings, ADR 0031) and removes the startup entry.
- **No code-signing certificate yet.** SmartScreen shows its warning on first run; the docs say how to continue.
  A `signCommand` slot is noted in `release.yml` for Azure Trusted Signing later.
- **Privacy.** No telemetry and no accounts. Logs are rotating local files (5 x 1 MB) in the app log folder,
  opened from Settings > General. The only network call is the update check.
- The npm SDK keeps its `repository` field (npm provenance checks it against the repo that builds the
  package); its `homepage` and `bugs` point at this repo.

## Consequences

- The Action needs three secrets: `TAURI_SIGNING_PRIVATE_KEY` (+ password) and `NPM_TOKEN`.
- Renaming or moving the repo keeps old links working (GitHub redirects), but plan the updater URL before any rename.
- A GitHub outage means no updates, never a broken bar.
