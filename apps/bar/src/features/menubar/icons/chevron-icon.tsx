import { type SVGProps } from "react";
import { Glyph } from "./glyph";

/**
 * A chevron for show/hide controls. `direction` is where the point faces.
 *
 * @example
 * <ChevronIcon direction="left" />
 */
const PATHS = {
  left: "M12.5 6.5L8 11l4.5 4.5",
  right: "M9.5 6.5L14 11l-4.5 4.5",
  down: "M6.5 9L11 13.5 15.5 9",
} as const;

export function ChevronIcon({ direction = "left", ...props }: SVGProps<SVGSVGElement> & { direction?: keyof typeof PATHS }) {
  return (
    <Glyph {...props}>
      <path d={PATHS[direction]} />
    </Glyph>
  );
}
