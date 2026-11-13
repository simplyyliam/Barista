import { motion } from "motion/react";
import { type SVGProps } from "react";
import { SLIDE } from "@/shared/motion";
import { Glyph } from "./glyph";

/**
 * Control center: two toggle pills, one knob left and one right (the macOS symbol). While its panel
 * is open the knobs trade places, a small sign that it is "switched on".
 *
 * @example
 * <ControlCenterIcon open />
 */
export function ControlCenterIcon({ open = false, ...props }: SVGProps<SVGSVGElement> & { open?: boolean }) {
  return (
    <Glyph {...props}>
      <rect x="4.25" y="4.25" width="13.5" height="5.5" rx="2.75" />
      <rect x="4.25" y="12.25" width="13.5" height="5.5" rx="2.75" />
      <motion.circle
        cy="7"
        r="1.4"
        fill="currentColor"
        stroke="none"
        initial={false}
        animate={{ cx: open ? 15 : 7 }}
        transition={SLIDE}
      />
      <motion.circle
        cy="15"
        r="1.4"
        fill="currentColor"
        stroke="none"
        initial={false}
        animate={{ cx: open ? 7 : 15 }}
        transition={SLIDE}
      />
    </Glyph>
  );
}
