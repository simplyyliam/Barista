import { useEffect, useState } from "react";

export function Clock() {
  const [time, setTime] = useState(new Date());

  useEffect(() => {
    // update the state every second
    const timerId = setInterval(() => {
      setTime(new Date())
    }, 1000)

    return () => clearInterval(timerId)
  }, []);

  return (
    <div className="">
      {time.toLocaleTimeString([], {hour: '2-digit', minute: '2-digit'})}
    </div>
  )
}
