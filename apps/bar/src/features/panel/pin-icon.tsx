import { type SVGProps } from "react"

/**
 * A small pin. Filled when the item is pinned (stays in the bar), outlined when it is not.
 *
 * @example
 * <PinIcon pinned />
 */
export function PinIcon({ pinned, ...props }: SVGProps<SVGSVGElement> & { pinned: boolean }) {
  return (
    <svg viewBox="0 0 12 12" width="10" height="10" aria-hidden="true" {...props}>
      <path
        d="M7.2 1.2 10.8 4.8 9.3 5.2 7.4 7.4 7.6 9.2 6.8 9.2 4.6 7 1.6 10 1 10.4 1.4 9.4 4.4 6.4 2.2 4.2 2.2 3.4 4 3.6 6.2 1.7z"
        fill={pinned ? "currentColor" : "none"}
        stroke="currentColor"
        strokeWidth="1"
        strokeLinejoin="round"
      />
    </svg>
  )
}
