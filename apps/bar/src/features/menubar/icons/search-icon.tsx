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
      <circle cx="9.6" cy="9.6" r="5.1" />
      <path d="M13.4 13.4L17.4 17.4" />
    </Glyph>
  );
}
