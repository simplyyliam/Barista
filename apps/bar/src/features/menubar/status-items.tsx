import { commands, events, type StatusItemView } from "@/shared/bindings";
import { Icon } from "@barista/ui/icons";
import { AnimatePresence } from "motion/react";
import { useEffect, useState } from "react";
import { ITEM_EXIT, ITEM_HIDDEN } from "@/shared/motion";
import { useItemBounds } from "./hooks/use-item-bounds";
import { ITEM_APPEAR, MenubarItem } from "./menubar-item";

export function StatusItems() {
  const [items, setItems] = useState<StatusItemView[]>([]);

  useEffect(() => {
    commands.getItems().then(setItems)
    const off = events.itemsChanged.listen((e) => setItems(e.payload))
    return () => { off.then((stop) => stop()) }
  }, []);

  // Menus open under their icon, so Rust needs to know where each one is drawn.
  useItemBounds(items.map((item) => `${item.id}:${item.label ?? ""}`).join("|"));

  // An app's item fades in with a touch of scale and blur, and leaves a little faster; the pills
  // around it glide into the gap (the layout spring in MenubarItem). The first list does not animate.
  return (
    <AnimatePresence initial={false} mode="popLayout">
      {items.map((item) => (
        <MenubarItem className="gap-1.5 tabular-nums"
          key={item.id}
          {...ITEM_APPEAR}
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
