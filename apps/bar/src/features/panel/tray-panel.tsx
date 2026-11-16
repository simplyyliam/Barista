import { motion } from "motion/react";
import { commands } from "@/shared/bindings";
import { useCollapsed } from "@/shared/hooks/use-collapsed";
import { useItems } from "@/shared/hooks/use-items";
import { itemClickHandlers } from "@/shared/lib/item-click";
import { GRID_HIDDEN, GRID_SHOWN, gridEnter, staggerDelay } from "@/shared/motion";
import { useTileReorder } from "./hooks/use-tile-reorder";
import { PinIcon } from "./pin-icon";

/** Five 36 px tiles, four 4 px gaps and 8 px padding either side: the Windows tray flyout has five per row. */
export const TRAY_PANEL_WIDTH = 5 * 36 + 4 * 4 + 2 * 8;
/** A first guess (two rows and the collapse button); Rust remembers the measured size after the first open. */
export const TRAY_PANEL_HEIGHT = 2 * 36 + 4 + 2 * 8 + 30;

/**
 * The `tray` route: every Windows tray icon (and any other item Rust put in overflow) in a wrapping
 * grid, under the arrow in the bar. Nothing in the bar moves when it opens, and any number of icons
 * fits. The tiles come in as a small wave each time the panel opens (small elements inside a card may
 * move, ADR 0023 rule 15); a tile's picture changes instantly.
 *
 * Each tile does what the icon does in the Windows tray: click, right-click (the app's own menu) and
 * double-click go to the app. On top of that: a pin in the corner keeps the icon in the bar, a press and
 * drag reorders, and the button at the bottom collapses the bar to its pinned items or shows them all.
 */
export function TrayPanel({ active }: { active: boolean }) {
  const all = useItems();
  const collapsed = useCollapsed();
  const items = all.filter((item) => item.source === "tray" || item.placement === "overflow");
  const { shown, handlers } = useTileReorder(items.map((item) => item.id));
  const byId = new Map(items.map((item) => [item.id, item]));

  return (
    <div className="flex flex-col" style={{ width: TRAY_PANEL_WIDTH }}>
      <section className="rounded-panel bg-cc-card p-2 text-cc-fg">
        {items.length === 0 ? (
          <p className="px-2 py-2 text-[13px] text-cc-fg-muted">No tray icons</p>
        ) : (
          <>
            {/* A new key on every open replays the wave; while hidden nothing runs. */}
            <div key={active ? "open" : "closed"} className="grid grid-cols-5 gap-1">
              {shown.map((id, index) => {
                const item = byId.get(id);
                if (!item) return null;
                return (
                  <motion.div
                    key={item.id}
                    title={item.tooltip ?? item.appName}
                    aria-label={item.tooltip?.split("\n")[0] || item.appName}
                    data-item-id={item.id}
                    className="group relative grid size-9 cursor-default touch-none select-none place-items-center rounded-panel-child bg-cc-tile hover:bg-cc-tile-hover"
                    initial={active ? GRID_HIDDEN : false}
                    animate={{ ...GRID_SHOWN, transition: gridEnter(staggerDelay(index)) }}
                    {...itemClickHandlers(item.id)}
                    {...handlers(item.id)}
                  >
                    {item.icon ? (
                      <img src={item.icon} alt="" draggable={false} className="pointer-events-none size-[18px] object-contain" />
                    ) : (
                      <span className="pointer-events-none text-[12px] font-semibold">
                        {item.appName.slice(0, 1).toUpperCase()}
                      </span>
                    )}
                    <button
                      type="button"
                      aria-label={item.pinned ? "Unpin from the bar" : "Pin to the bar"}
                      aria-pressed={item.pinned}
                      title={item.pinned ? "Unpin from the bar" : "Pin to the bar"}
                      className={
                        "absolute -top-1 -right-1 size-4 place-items-center rounded-full bg-cc-card text-cc-fg outline-none " +
                        (item.pinned ? "grid" : "hidden group-hover:grid")
                      }
                      // A press on the pin is not the start of a drag, and not a click on the icon.
                      onPointerDown={(e) => e.stopPropagation()}
                      onClick={(e) => {
                        e.stopPropagation();
                        void commands.setPinned(item.id, !item.pinned);
                      }}
                      onContextMenu={(e) => e.stopPropagation()}
                    >
                      <PinIcon pinned={item.pinned} />
                    </button>
                  </motion.div>
                );
              })}
            </div>
            <button
              type="button"
              className="mt-1 w-full rounded-panel-child px-2 py-1 text-left text-[12px] text-cc-fg-muted outline-none hover:bg-cc-tile-hover hover:text-cc-fg"
              onClick={() => void commands.setCollapsed(!collapsed)}
            >
              {collapsed ? "Show all in the bar" : "Collapse to pinned"}
            </button>
          </>
        )}
      </section>
    </div>
  );
}
