import { type StatusItemView } from "@/shared/bindings";
import { useItems } from "@/shared/hooks/use-items";
import { Swap, WAVE_EXIT, WAVE_HIDDEN, WAVE_SHOWN, staggerDelay, waveEnter } from "@/shared/motion";
import { AnimatePresence } from "motion/react";
import { useCallback, useState } from "react";
import { useAutoCollapse } from "./hooks/use-auto-collapse";
import { useItemBounds } from "./hooks/use-item-bounds";
import { ChevronIcon } from "./icons";
import { ITEM_APPEAR, MenubarItem } from "./menubar-item";

const ARROW_ID = "tray/arrow";

/**
 * The Windows tray, folded away. Tray icons are many and mostly not needed at a glance, so the
 * group starts collapsed and shows only an arrow; open it to reach one. It folds itself again after
 * the pointer has been away for a few seconds. No arrow when the tray is empty.
 *
 * The arrow never moves: the icons open out to its left (the bar's right side is right-aligned, so
 * nothing on the right shifts). Motion (ADR 0023, docs/research/ux-motion-and-taste.md): the icons
 * come in one after another starting next to the arrow, and leave all together, quicker. An icon's own picture changes
 * instantly, because apps flip it all day (unread badges, spinners).
 */
export function TrayItems() {
  const items = useItems().filter((item) => item.source === "tray");
  const [open, setOpen] = useState(false);
  const collapse = useCallback(() => setOpen(false), []);
  useAutoCollapse(open, collapse);

  // Rust needs each pill's place (v0.7 opens tray menus under it).
  useItemBounds(`${items.length ? ARROW_ID : ""}|${open ? items.map((item) => item.id).join("|") : ""}`);

  if (items.length === 0) return null;

  const label = open ? "Hide tray icons" : "Show tray icons";
  return (
    <>
      <AnimatePresence initial={false} mode="popLayout">
        {open &&
          items.map((item, index) => (
            // The wave starts next to the arrow (the last one in the row) and moves outwards.
            <TrayItem key={item.id} item={item} order={items.length - 1 - index} />
          ))}
      </AnimatePresence>
      <MenubarItem
        variant="icon"
        role="button"
        tabIndex={0}
        aria-label={label}
        aria-expanded={open}
        title={label}
        data-item-id={ARROW_ID}
        className="cursor-default outline-none focus-visible:bg-bar-hover"
        {...ITEM_APPEAR}
        onClick={() => setOpen((value) => !value)}
        onKeyDown={(e) => {
          if (e.key === "Enter" || e.key === " ") {
            e.preventDefault();
            setOpen((value) => !value);
          }
        }}
      >
        <Swap id={open}>
          <ChevronIcon direction={open ? "right" : "left"} />
        </Swap>
      </MenubarItem>
    </>
  );
}

function TrayItem({ item, order }: { item: StatusItemView; order: number }) {
  const name = item.tooltip?.split("\n")[0] || item.appName;
  return (
    <MenubarItem
      variant="icon"
      title={item.tooltip ?? item.appName}
      aria-label={name}
      data-item-id={item.id}
      className="cursor-default"
      initial={WAVE_HIDDEN}
      animate={{ ...WAVE_SHOWN, transition: waveEnter(staggerDelay(order)) }}
      exit={{ ...WAVE_HIDDEN, x: 0, scale: 0.9, transition: WAVE_EXIT }}
      // Clicks arrive with v0.7: no press dip until the icon does something.
      whileTap={undefined}
    >
      {item.icon ? (
        <img src={item.icon} alt="" draggable={false} className="pointer-events-none size-4 object-contain" />
      ) : (
        // An icon the app gave no picture for (or one we could not read): its first letter.
        <span className="pointer-events-none grid size-4 place-items-center rounded-full bg-bar-hover text-[10px] font-semibold">
          {item.appName.slice(0, 1).toUpperCase()}
        </span>
      )}
    </MenubarItem>
  );
}
