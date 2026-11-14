import { motion } from "motion/react";
import { useItems } from "@/shared/hooks/use-items";
import { GRID_HIDDEN, GRID_SHOWN, gridEnter, staggerDelay } from "@/shared/motion";

/** Five 36 px tiles, four 4 px gaps and 8 px padding either side: the Windows tray flyout has five per row. */
export const TRAY_PANEL_WIDTH = 5 * 36 + 4 * 4 + 2 * 8;
/** A first guess (two rows); Rust remembers the measured size after the first open. */
export const TRAY_PANEL_HEIGHT = 2 * 36 + 4 + 2 * 8;

/**
 * The `tray` route: every Windows tray icon in a wrapping grid, under the arrow in the bar. Nothing
 * in the bar moves when it opens, and any number of icons fits. The tiles come in as a small wave
 * each time the panel opens (small elements inside a card may move, ADR 0023 rule 15); a tile's
 * picture changes instantly. Clicks arrive with v0.7.
 */
export function TrayPanel({ active }: { active: boolean }) {
  const items = useItems().filter((item) => item.source === "tray");
  return (
    <div className="flex flex-col" style={{ width: TRAY_PANEL_WIDTH }}>
      <section className="rounded-panel bg-cc-card p-2 text-cc-fg">
        {items.length === 0 ? (
          <p className="px-2 py-2 text-[13px] text-cc-fg-muted">No tray icons</p>
        ) : (
          // A new key on every open replays the wave; while hidden nothing runs.
          <div key={active ? "open" : "closed"} className="grid grid-cols-5 gap-1">
            {items.map((item, index) => (
              <motion.div
                key={item.id}
                title={item.tooltip ?? item.appName}
                aria-label={item.tooltip?.split("\n")[0] || item.appName}
                data-item-id={item.id}
                className="grid size-9 cursor-default place-items-center rounded-panel-child bg-cc-tile"
                initial={active ? GRID_HIDDEN : false}
                animate={{ ...GRID_SHOWN, transition: gridEnter(staggerDelay(index)) }}
              >
                {item.icon ? (
                  <img src={item.icon} alt="" draggable={false} className="pointer-events-none size-[18px] object-contain" />
                ) : (
                  <span className="pointer-events-none text-[12px] font-semibold">
                    {item.appName.slice(0, 1).toUpperCase()}
                  </span>
                )}
              </motion.div>
            ))}
          </div>
        )}
      </section>
    </div>
  );
}
