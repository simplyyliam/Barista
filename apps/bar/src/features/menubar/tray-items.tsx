import { commands, type StatusItemView } from "@/shared/bindings";
import { useItems } from "@/shared/hooks/use-items";
import { useTrayStyle } from "@/shared/hooks/use-tray-style";
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
 */
export function TrayItems() {
  return useTrayStyle() === "inline" ? <InlineTray /> : <PanelTray />;
}

/**
 * The arrow only; the icons are in the `tray` panel (`features/panel/tray-panel.tsx`). Nothing in
 * the bar moves when it opens, whatever the screen width. No arrow when the tray is empty.
 */
function PanelTray() {
  const items = useItems().filter((item) => item.source === "tray");
  const open = useOpenPanel();
  const isOpen = open?.anchor === ARROW_ID;
  useItemBounds(items.length ? ARROW_ID : "");

  if (items.length === 0) return null;

  const click = () =>
    void commands.openPanel("tray", ARROW_ID, TRAY_PANEL_WIDTH, TRAY_PANEL_HEIGHT, "none");
  return (
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
  );
}

/**
 * The Windows tray, folded away, unfolding inside the bar. It needs room: on a narrow bar a long
 * tray can push the other items off, which is why `panel` is the default. Tray icons are many and mostly not needed at a glance, so the
 * group starts collapsed and shows only an arrow; open it to reach one. It folds itself again after
 * the pointer has been away for a few seconds. No arrow when the tray is empty.
 *
 * The arrow never moves: the icons open out to its left (the bar's right side is right-aligned, so
 * nothing on the right shifts). Motion (ADR 0023, docs/research/ux-motion-and-taste.md): the icons
 * come in one after another starting next to the arrow, and leave all together, quicker. An icon's own picture changes
 * instantly, because apps flip it all day (unread badges, spinners).
 */
function InlineTray() {
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
