# Checkpoint

Keep this file short: where we are, the next step, and facts a new session needs.
When a release ships, move its notes to `docs/release/history/` and start fresh.

## Where we are (2026-11-19)
- **v0.9.0 shipped** (tag `v0.9.0` on the merge of PR #51, not pushed yet: Liam runs `pnpm release`, then publishes the draft).
  Plan releases v0.0 to v0.9 are done. Full notes (script items, application menus, UI wiring lists, dogfooding gaps, test
  recipes) are in `docs/release/history/v0.9-checkpoint.md` (local only).
- **Next:** v0.10 "Ship it" (`docs/release/releases/v0.10.md`). Open SDK gaps and "fix next" items are in the v0.9 history file.
- **Spec departures Liam confirmed (2026-11-13), do not "fix" them:** icon-only pills are 28px wide, the battery is
  `examples/battery`'s drawing, the hover is a 999px capsule.

## Useful facts

- Write scripts with the Write tool; Bash heredocs with apostrophes break here.
  Python on Windows: open files with `encoding='utf-8', newline=''`.
