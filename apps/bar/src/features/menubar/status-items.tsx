import { commands, events, type StatusItemView } from "@/shared/bindings";
import { Icon } from "@barista/ui/icons";
import { useEffect, useState } from "react";
import { MenubarItem } from "./menubar-item";

export function StatusItems() {
  const [items, setItems] = useState<StatusItemView[]>([]);

  useEffect(() => {
    commands.getItems().then(setItems)
    const off = events.itemsChanged.listen((e) => setItems(e.payload))
    return () => { off.then((stop) => stop()) }
  }, []);
  return items.map((item) => (
    <MenubarItem className="gap-1.5"
      key={item.id}
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
