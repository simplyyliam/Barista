import { type SVGProps } from "react";
import { Glyph } from "./glyph";

/**
 * A chevron for show/hide controls. `direction` is where the point faces.
 *
 * @example
 * <ChevronIcon direction="left" />
 */
export function ChevronIcon({ direction = "left", ...props }: SVGProps<SVGSVGElement> & { direction?: "left" | "right" }) {
  return (
    <Glyph {...props}>
      <path d={direction === "left" ? "M12.5 6.5L8 11l4.5 4.5" : "M9.5 6.5L14 11l-4.5 4.5"} />
    </Glyph>
  );
}
