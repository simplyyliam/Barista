import { type SVGProps } from "react";
import { Glyph } from "./glyph";

/**
 * Control center: two toggle pills, one knob left and one right (the macOS symbol).
 *
 * @example
 * <ControlCenterIcon />
 */
export function ControlCenterIcon(props: SVGProps<SVGSVGElement>) {
  return (
    <Glyph {...props}>
      <rect x="4.75" y="4.75" width="12.5" height="5" rx="2.5" />
      <rect x="4.75" y="12.25" width="12.5" height="5" rx="2.5" />
      <circle cx="7.5" cy="7.25" r="1.4" fill="currentColor" stroke="none" />
      <circle cx="14.5" cy="14.75" r="1.4" fill="currentColor" stroke="none" />
    </Glyph>
  );
}
