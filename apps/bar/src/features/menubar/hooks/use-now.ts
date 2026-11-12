import { useEffect, useState } from "react"

/** The current time, updated at the start of every minute (so the date also rolls over at midnight). */
export function useNow() {
  const [now, setNow] = useState(() => new Date())

  useEffect(() => {
    let timerId: ReturnType<typeof setTimeout>
    const tick = () => {
      const current = new Date()
      setNow(current)
      timerId = setTimeout(tick, 60_000 - (current.getSeconds() * 1000 + current.getMilliseconds()))
    }
    tick()
    return () => clearTimeout(timerId)
  }, [])

  return now
}
