import { commands, type StatusItemView } from "@/shared/bindings";
import { useItems } from "@/shared/hooks/use-items";
import { useTrayStyle } from "@/shared/hooks/use-tray-style";
import { itemClickHandlers } from "@/shared/lib/item-click";
import { TRAY_PANEL_HEIGHT, TRAY_PANEL_WIDTH } from "@/features/panel/tray-panel";
import { useOpenPanel } from "@/features/panel";
import { Swap, WAVE_EXIT, WAVE_HIDDEN, WAVE_SHOWN, staggerDelay, waveEnter } from "@/shared/motion";
import { AnimatePresence } from "motion/react";
import { useCallback, useState } from "react";
import { useAutoCollapse } from "./hooks/use-auto-collapse";
import { useItemBounds } from "./hooks/use-item-bounds";
import { ChevronIcon } from "./icons";
import { ITEM_APPEAR, MenubarItem } from "./menubar-item";

const ARROW_ID = "tray/arrow";

/**
 * The Windows tray in the bar, as the `trayStyle` setting says (default `panel`; the v0.8 settings
 * window flips it): an arrow that opens a grid under it, or an arrow that unfolds the icons inline.
 * Pinned tray icons are always in the bar, as pills with the same clicks as in the Windows tray.
 */
export function TrayItems() {
  return useTrayStyle() === "inline" ? <InlineTray /> : <PanelTray />;
}

/** What the tray needs from the item list: the icons Rust placed in the bar, and whether any item is in overflow. */
function useTray() {
  const items = useItems();
  const tray = items.filter((item) => item.source === "tray");
  return {
    // Pinned icons (or all of them while the bar is expanded) live in the bar.
    inBar: tray.filter((item) => item.placement === "bar"),
    // The rest wait behind the arrow.
    folded: tray.filter((item) => item.placement === "overflow"),
    // The arrow is there while there is anything behind it, and while any tray icon exists (the panel holds the pin and collapse controls).
    arrow: tray.length > 0 || items.some((item) => item.placement === "overflow"),
  };
}

/**
 * The arrow and the pinned icons; the rest are in the `tray` panel (`features/panel/tray-panel.tsx`).
 * Nothing in the bar moves when the panel opens, whatever the screen width.
 */
function PanelTray() {
  const { inBar, arrow } = useTray();
  const open = useOpenPanel();
  const isOpen = open?.anchor === ARROW_ID;
  useItemBounds(`${arrow ? ARROW_ID : ""}|${inBar.map((item) => item.id).join("|")}`);

  const click = () =>
    void commands.openPanel("tray", ARROW_ID, TRAY_PANEL_WIDTH, TRAY_PANEL_HEIGHT, "none");
  return (
    <>
      <AnimatePresence initial={false} mode="popLayout">
        {inBar.map((item) => (
          <TrayItem key={item.id} item={item} order={0} />
        ))}
      </AnimatePresence>
      {arrow && (
        <MenubarItem
          variant="icon"
          role="button"
          tabIndex={0}
          aria-label="Tray icons"
          aria-expanded={isOpen}
          title="Tray icons"
          open={isOpen}
          data-item-id={ARROW_ID}
          className="cursor-default outline-none focus-visible:bg-bar-hover"
          {...ITEM_APPEAR}
          onClick={click}
          onKeyDown={(e) => {
            if (e.key === "Enter" || e.key === " ") {
              e.preventDefault();
              click();
            }
          }}
        >
          <ChevronIcon direction="down" />
        </MenubarItem>
      )}
    </>
  );
}

/**
 * The Windows tray, folded away, unfolding inside the bar. It needs room: on a narrow bar a long
 * tray can push the other items off, which is why `panel` is the default. Tray icons are many and mostly not needed at a glance, so the
 * group starts collapsed and shows only an arrow (and the pinned icons); open it to reach one. It folds itself again after
 * the pointer has been away for a few seconds. No arrow when the tray is empty.
 *
 * The arrow never moves: the icons open out to its left (the bar's right side is right-aligned, so
 * nothing on the right shifts). Motion (ADR 0023, docs/research/ux-motion-and-taste.md): the icons
 * come in one after another starting next to the arrow, and leave all together, quicker. An icon's own picture changes
 * instantly, because apps flip it all day (unread badges, spinners).
 */
function InlineTray() {
  const { inBar, folded, arrow } = useTray();
  const [open, setOpen] = useState(false);
  const collapse = useCallback(() => setOpen(false), []);
  useAutoCollapse(open, collapse);

  // Rust needs each pill's place (it opens tray menus under it).
  useItemBounds(
    `${arrow ? ARROW_ID : ""}|${inBar.map((item) => item.id).join("|")}|${open ? folded.map((item) => item.id).join("|") : ""}`,
  );

  if (!arrow) return null;

  const label = open ? "Hide tray icons" : "Show tray icons";
  return (
    <>
      <AnimatePresence initial={false} mode="popLayout">
        {open &&
          folded.map((item, index) => (
            // The wave starts next to the arrow (the last one in the row) and moves outwards.
            <TrayItem key={item.id} item={item} order={folded.length - 1 - index} />
          ))}
      </AnimatePresence>
      <AnimatePresence initial={false} mode="popLayout">
        {inBar.map((item) => (
          <TrayItem key={item.id} item={item} order={0} />
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
      // Left, right and middle go to the app like in the Windows tray (a double click is found in Rust).
      {...itemClickHandlers(item.id)}
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
