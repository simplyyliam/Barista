# 0023: Motion language

Status: proposed (2026-11-13). Liam asked for "premium but subtle" motion in the bar, using `motion/react`, and then said not to put it on everything.

## Context

The bar and its panels were static. Small, well-chosen motion (a press that answers, icons that
turn into each other, items that arrive without a jump) makes the bar feel good, not just look
good. Too much of it makes people drowsy and dizzy, and the bar is used hundreds of times a day.
Reasoning and sources: `docs/research/ux-motion-and-taste.md` (local, not tracked).

## Decision

- **Motion is the exception.** A new element gets none by default. It gets motion only if it is
  infrequent, or a real change the eye would miss. Always-visible text (clock, battery percent
  in the bar) and anything used many times a day stays instant. Panels do not animate open
  (ADR 0018).
- **One place for the values:** `apps/bar/src/shared/motion.tsx` (press, icon swap, text drop,
  item enter/exit, slide). Components import them, they never write their own numbers.
- **Values:** press `scale 0.96` (0.985 on full-width rows, where 0.96 looks like a lurch),
  spring 0.2 s, no bounce; icon swap fade + scale 0.25 to 1 + blur 4px, spring 0.3 s, no bounce;
  items fade + scale + blur, exit faster than enter; ease-out `[0.23, 1, 0.32, 1]`.
- **Switching panels** (a panel is open and another item is clicked): Rust moves the one window at once, so
  the page animates the incoming content from where the old panel was (`usePanelSwap`, at most 48px,
  fade + scale + blur, 240 ms). Opening from closed stays instant. The old content cannot fade out: the
  window is already at its new place and size.
- **Hover is instant in, 150 ms out** (CSS, not motion). The hover capsule is fully rounded
  (`--menubar-pill-radius: 999px`, like macOS; set 8px for a softer rectangle).
- **Only transform, opacity and a small blur.** No loops. Idle = zero running animations.
- **Reduced motion:** `<Motion>` (MotionConfig `reducedMotion="user"`) wraps the bar and the panel window.
- **Anchors ignore transforms:** `useItemBounds` sends the settled rect, so a pressed or sliding
  item never shifts where a panel opens.

## Consequences

- `motion` is a new dependency of the bar (needs its credit entry).
- Adding motion to something new means adding a row to the table in the research file with the
  reason; "it looks nice" is not a reason for something seen all day.
- The 0.985 press on wide rows is a deliberate exception to the 0.96 rule.
