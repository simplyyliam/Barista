import { type SVGProps } from "react";
import { Glyph } from "./glyph";

/**
 * Magnifying glass.
 *
 * @example
 * <SearchIcon />
 */
export function SearchIcon(props: SVGProps<SVGSVGElement>) {
  return (
    <Glyph {...props}>
      <circle cx="9.5" cy="9.5" r="5.5" />
      <path d="M13.6 13.6L18 18" />
    </Glyph>
  );
}
