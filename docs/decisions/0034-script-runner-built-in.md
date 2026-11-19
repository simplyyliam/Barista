# 0034: The script runner is built into the bar

Status: **proposed** (2026-11-19, v0.9; becomes accepted when Liam merges the PR)

## Context

v0.9 runs user scripts on a timer and shows their output as bar items. Scripts are untrusted input: one
that hangs, floods stdout or spawns children must never freeze or slow the bar (CLAUDE.md rule 3).
The first-party items of v0.8 are an SDK client in a separate process. Doing the same here would add
a process and memory for no gain, since the runner has no UI of its own.

## Decision

- The runner is a Rust module in the bar (`src-tauri/src/scripts/`), started in `setup`. **No separate
  process.** Script items go straight into the item registry (`script/<id>` keys), with the same menu
  types SDK items use. They do not go through the pipe, and **no SDK or protocol type changes**.
- One tokio task per script. A run that is still going makes the next tick a skip. Failures double the
  interval (max 5 min).
- Each run starts inside a Windows **Job Object** with kill-on-close. Dropping the job (timeout, script
  removed, scripts turned off, bar exit) kills the whole process tree. The process starts suspended and
  is resumed only after it is in the job, so no child can slip out first.
- Windows-only code (job object, suspended start) is in `platform/windows/job.rs` (rule 7).
- **Watching the folder: the existing `platform/windows/folder_watch.rs`** (`FindFirstChangeNotification`,
  one thread asleep in `WaitForSingleObject`, no polling), plus a 300 ms debounce and a **rescan** of the
  folder. The release file named the `notify` crate; we did not need it:
  - the folder is small, so a rescan is cheap and cannot miss, double or mis-pair a rename;
  - it is already in the bar and proven, and it adds no dependency (`notify` brings a dozen crates).
  If a user ever has thousands of scripts, switch to `ReadDirectoryChangesW` directly.
- **Trust** is per script: the id plus the SHA-256 of the file. It lives in its own `script-trust.json`
  beside `settings.json`, and never in settings export/import, so importing settings cannot grant trust.
  The file is hashed again before every run, so an edit between trust and run is caught.
- The trust prompt is an internal command and event pair for the bar's UI, not public API.

## Consequences

- No extra process or memory. Task Manager shows only the script's own short-lived children.
- A bug in the runner is a bug in the bar process, so the runner never `unwrap`s on script data and every
  limit (64 KB, 200 lines, timeout) is checked in Rust.
- Scripts run with the user's permissions. This is a trust prompt, **not a sandbox**.
- **Known and accepted:** the hash is checked just before each run and the interpreter opens the file a
  moment later, so a write in between runs unreviewed content once (the watcher then asks again).
  Closing that gap means keeping the file locked against writes during every run, which would make editors
  fail to save a script while it runs (a PowerShell script runs about a third of the time at 5 s). Only
  someone who can already write to the folder can use it, and that person can already run code as the user.
- A menu line with `shell=` starts a program. Settings has a switch for that (default on), because a
  script that prints outside data (a window title, a branch name) could otherwise let that data start one.

