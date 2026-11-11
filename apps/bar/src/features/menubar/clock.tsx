import { useEffect, useState } from "react";
import { MenubarItem } from "./menubar-item";

export function Clock() {
  const [time, setTime] = useState(new Date());

  useEffect(() => {
    // Only minutes are shown, so wake up once at the start of each minute instead of every second.
    let timerId: ReturnType<typeof setTimeout>
    const tick = () => {
      const now = new Date()
      setTime(now)
      timerId = setTimeout(tick, 60_000 - (now.getSeconds() * 1000 + now.getMilliseconds()))
    }
    tick()

    return () => clearTimeout(timerId)
  }, []);

  return (
    <MenubarItem>
      {time.toLocaleTimeString([], {hour: '2-digit', minute: '2-digit'})}
    </MenubarItem>
  )
}
