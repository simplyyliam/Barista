import { motion } from "motion/react";
import { useId, type SVGProps } from "react";
import { EASE_OUT } from "@/shared/motion";
import { Glyph, safeId } from "./glyph";

type WifiIconProps = SVGProps<SVGSVGElement> & {
  /** 0 to 3 arcs lit; the rest are dimmed. */
  strength?: 0 | 1 | 2 | 3;
  /** The radio is off: everything dims and a slash cuts through. */
  off?: boolean;
};

// Three arcs around (11, 16), opening 90 degrees upwards: inner, middle, outer.
const ARCS = [
  "M8.53 13.53A3.5 3.5 0 0 1 13.47 13.53",
  "M6.05 11.05A7 7 0 0 1 15.95 11.05",
  "M3.58 8.58A10.5 10.5 0 0 1 18.42 8.58",
];
const SLASH = "M5 4.5L17 17.5";
const FADE = { duration: 0.18, ease: EASE_OUT };
const DRAW = { duration: 0.2, ease: EASE_OUT };

/**
 * Wi-Fi in the bar: dot plus three arcs. The arcs light and dim with the signal and the slash
 * draws across when the radio goes off (the cut around it follows).
 *
 * @example
 * <WifiIcon strength={2} />
 */
export function WifiIcon({ strength = 3, off = false, ...props }: WifiIconProps) {
  const maskId = `wifi-off-${safeId(useId())}`;
  return (
    <Glyph {...props}>
      {/* Cuts a gap around the slash so it reads clearly over the arcs. */}
      <mask id={maskId} maskUnits="userSpaceOnUse" x="0" y="0" width="22" height="22">
        <rect width="22" height="22" fill="white" />
        <motion.path
          d={SLASH}
          stroke="black"
          strokeWidth={4}
          initial={false}
          animate={{ pathLength: off ? 1 : 0, opacity: off ? 1 : 0 }}
          transition={DRAW}
        />
      </mask>
      <g mask={`url(#${maskId})`}>
        {ARCS.map((d, i) => (
          <motion.path
            key={d}
            d={d}
            initial={false}
            animate={{ opacity: !off && i < strength ? 1 : 0.3 }}
            transition={FADE}
          />
        ))}
        <motion.circle
          cx="11"
          cy="16"
          r="1.1"
          fill="currentColor"
          stroke="none"
          initial={false}
          animate={{ opacity: !off && strength > 0 ? 1 : 0.3 }}
          transition={FADE}
        />
      </g>
      <motion.path
        d={SLASH}
        initial={false}
        animate={{ pathLength: off ? 1 : 0, opacity: off ? 1 : 0 }}
        transition={DRAW}
      />
    </Glyph>
  );
}
