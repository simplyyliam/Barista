import { useLayoutEffect, useRef, type RefObject } from "react"

/** A ref that always holds the newest value, for event handlers that must not go stale. */
export function useLatest<T>(value: T): RefObject<T> {
  const ref = useRef(value)
  useLayoutEffect(() => {
    ref.current = value
  })
  return ref
}
