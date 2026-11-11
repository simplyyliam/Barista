import { commands, events, type StatusItemView } from "@/shared/bindings";
import { Icon } from "@barista/ui/icons";
import { useEffect, useState } from "react";
import { useItemBounds } from "./hooks/use-item-bounds";
import { MenubarItem } from "./menubar-item";

export function StatusItems() {
  const [items, setItems] = useState<StatusItemView[]>([]);

  useEffect(() => {
    commands.getItems().then(setItems)
    const off = events.itemsChanged.listen((e) => setItems(e.payload))
    return () => { off.then((stop) => stop()) }
  }, []);

  // Menus open under their icon, so Rust needs to know where each one is drawn.
  useItemBounds(items.map((item) => `${item.id}:${item.label ?? ""}`).join("|"));

  return items.map((item) => (
    <MenubarItem className="gap-1.5"
      key={item.id}
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
  ))
}
