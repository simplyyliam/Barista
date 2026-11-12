import { MenubarItem } from "./menubar-item";
import { useNow } from "./hooks/use-now";

/** The time. The date lives in the middle of the bar (`DateItem`), like the design. */
export function Clock() {
  const time = useNow();

  return (
    <MenubarItem>
      <p>
        {time.toLocaleTimeString([], {
          hour: "2-digit",
          minute: "2-digit",
        })}
      </p>
    </MenubarItem>
  );
}
