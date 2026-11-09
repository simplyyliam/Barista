import { commands, events, type StatusItemView } from "@/shared/bindings";
import { Button } from "@barista/ui/components/ui/button";
import { Icon } from "@barista/ui/icons";
import { useEffect, useState } from "react";

export function StatusItems() {
  const [items, setItems] = useState<StatusItemView[]>([]);

  useEffect(() => {
    commands.getItems().then(setItems)
    const off = events.itemsChanged.listen((e) => setItems(e.payload))
    return () => { off.then((stop) => stop()) }
  }, []);
  return items.map((item) => (
    <Button key={item.id} onClick={() => commands.itemClicked(item.id, "middle")}>
      {item.icon && <Icon icon={item.icon} />}
      {item.label}
    </Button>
  ))
}
