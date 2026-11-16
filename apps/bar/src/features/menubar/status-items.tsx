import { commands } from "@/shared/bindings";
import { useItems } from "@/shared/hooks/use-items";
import { Icon } from "@barista/ui/icons";
import { AnimatePresence } from "motion/react";
import { ITEM_ENTER, ITEM_EXIT, ITEM_HIDDEN, ITEM_SHOWN } from "@/shared/motion";
import { useArrivalDelays } from "./hooks/use-arrival-delays";
import { useItemBounds } from "./hooks/use-item-bounds";
import { MenubarItem } from "./menubar-item";

/** The items client apps made through the SDK. Windows tray icons are in `TrayItems`. */
export function StatusItems() {
  // Items Rust put in overflow (collapsed and unpinned, or no room) are in the tray panel, not here.
  const items = useItems().filter((item) => item.source === "app" && item.placement === "bar");
  // Items that arrive together (every app reconnecting after a bar start) come in as a small wave.
  const delayOf = useArrivalDelays(items.map((item) => item.id));

  // Menus open under their icon, so Rust needs to know where each one is drawn.
  useItemBounds(items.map((item) => `${item.id}:${item.label ?? ""}`).join("|"));

  // An app's item fades in with a touch of scale and blur, and leaves a little faster; the pills
  // around it glide into the gap (the layout spring in MenubarItem). The first list does not animate.
  return (
    <AnimatePresence initial={false} mode="popLayout">
      {items.map((item) => (
        <MenubarItem className="gap-1.5 tabular-nums"
          key={item.id}
          initial={ITEM_HIDDEN}
          animate={{ ...ITEM_SHOWN, transition: { ...ITEM_ENTER, delay: delayOf(item.id) } }}
          exit={{ ...ITEM_HIDDEN, transition: ITEM_EXIT }}
          data-item-id={item.id}
          // With a menu open, moving onto another item switches menus without a click.
          onMouseEnter={() => void commands.menuHover(item.id)}
          onClick={() => commands.itemClicked(item.id, "left")}
          onContextMenu={(e) => {
            e.preventDefault()
            commands.itemClicked(item.id, "right")
          }}
          onAuxClick={(e) => {
            if (e.button === 1) commands.itemClicked(item.id, "middle")
          }}
        >
          {item.icon && <Icon icon={item.icon} />}
          {item.label}
        </MenubarItem>
      ))}
    </AnimatePresence>
  )
}
