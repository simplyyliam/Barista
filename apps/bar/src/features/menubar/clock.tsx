
import { useEffect, useState } from "react";
import { MenubarItem } from "./menubar-item";

export function Clock() {
  const [time, setTime] = useState(() => new Date());

  useEffect(() => {
    let timerId: ReturnType<typeof setTimeout>;

    const tick = () => {
      const now = new Date();
      setTime(now);

      // Synchronize with the beginning of the next minute.
      const delay =
        60_000 - (now.getSeconds() * 1000 + now.getMilliseconds());

      timerId = setTimeout(tick, delay);
    };

    tick();

    return () => clearTimeout(timerId);
  }, []);

  return (
    <MenubarItem className="gap-2">
      <p>
        {time.toLocaleDateString("en-US", {
          weekday: "short",
          month: "short",
          day: "numeric",
        })}
      </p>
      <p>
        {time.toLocaleTimeString([], {
          hour: "2-digit",
          minute: "2-digit",
        })}
      </p>
    </MenubarItem>
  );
}
