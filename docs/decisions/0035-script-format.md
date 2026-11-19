# 0035: Script format

Status: **proposed** (2026-11-19, v0.9; becomes accepted when Liam merges the PR)

## Context

Script items need a small text format. It is inspired by xbar and SwiftBar but is our own, with our own
parser. We do not claim compatibility and reuse none of their code.

## Decision

**File name:** `name.INTERVAL.ext` (`cpu.10s.ps1`). Interval units `s`, `m`, `h`, `d`; minimum 1 s
(`500ms` is not an interval). The interval is optional. The item id is the name without the interval,
mapped to `[a-z0-9-_.]` (`clock.10s.mjs` becomes `clock.mjs`), so changing the interval keeps the item.

| Extension | Runs with |
|---|---|
| `.ps1` | `pwsh` if installed, else Windows PowerShell (`-NoProfile -NonInteractive`) |
| `.py` | the `py` launcher, else `python` |
| `.js`, `.mjs` | `node` |

A missing interpreter gives an error item ("Python not found"), never a crash.

**Output:** the first line is the bar label. A `---` line starts the menu, later `---` lines are
separators. Text between the label and the first `---` is ignored for now.

**Menu line params,** after the last ` | `, as `key=value` (quote values with spaces):

| Param | Does |
|---|---|
| `href=` | opens a URL in the default browser. Only `http`, `https`, `mailto`. |
| `shell=` (+ `param1=`, `param2=` ...) | runs a program with those arguments, no shell string |
| `refresh=true` | runs the script again after the click |
| `color=` | colors the line (CSS name or `#rgb` / `#rrggbb`) |
| `--` prefix | submenu (more dashes = deeper, max 3 levels) |

Unknown params are ignored. Bad quoting drops the params and keeps the text.

**Limits (all enforced in Rust):** stdout 64 KB, menu 200 lines, line 1000 characters, label 200
characters, href 2048 characters, up to 20 `paramN`. Output with NUL bytes or invalid UTF-8 is
"garbage" and gives the error state. Control characters are stripped from text.

**Timing:** run timeout 10 s, never longer than the interval. Interval order: file name, then the
script's entry in settings, then the default (60 s). Failures double the interval, max 5 min.

**Trust:** nothing runs until the user confirms. Stored per script id + SHA-256. An edit asks again.

## Consequences

- Text is data: `shell=` takes an argument list, so menu text can never inject a command.
- Later, not now: `image=`, `dropdown=false`, `terminal=`, other shells, a changeable folder, a gallery.
- The docs must say plainly: scripts run with your permissions; the prompt is not a sandbox.
