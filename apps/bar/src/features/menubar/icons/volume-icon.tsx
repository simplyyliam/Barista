import { motion } from "motion/react";
import { type SVGProps } from "react";
import { EASE_OUT } from "@/shared/motion";
import { waveCount } from "../lib/levels";
import { Glyph } from "./glyph";

type VolumeIconProps = SVGProps<SVGSVGElement> & {
  /** 0 to 1, as the volume service reports it. */
  level?: number;
  muted?: boolean;
};

const SPEAKER = "M4 8.6H6.6L10.6 5.2V16.8L6.6 13.4H4Z";
// Waves around (10.4, 11), opening 90 degrees to the right: small, medium, large.
const WAVES = [
  "M12.38 9.02A2.8 2.8 0 0 1 12.38 12.98",
  "M14.08 7.32A5.2 5.2 0 0 1 14.08 14.68",
  "M15.77 5.63A7.6 7.6 0 0 1 15.77 16.37",
];
const CROSS = "M14 8.8L18.4 13.2M18.4 8.8L14 13.2";

/** A stroke that draws on (and off) along its length. Opacity too: a round cap would leave a dot at length 0. */
const draw = (on: boolean, delay = 0) => ({
  animate: { opacity: on ? 1 : 0, pathLength: on ? 1 : 0 },
  transition: { duration: 0.18, ease: EASE_OUT, delay: on ? delay : 0 },
});

/**
 * Speaker with 0 to 3 waves (level), or crossed out when muted. The waves draw on and off one
 * after the other as the level moves, and the cross draws in over a mute.
 *
 * @example
 * <VolumeIcon level={0.5} muted={false} />
 */
export function VolumeIcon({ level = 1, muted = false, ...props }: VolumeIconProps) {
  const waves = muted ? 0 : waveCount(level);
  return (
    <Glyph {...props}>
      <path d={SPEAKER} />
      {WAVES.map((d, i) => (
        <motion.path key={d} d={d} initial={false} {...draw(i < waves, i * 0.03)} />
      ))}
      <motion.path d={CROSS} initial={false} {...draw(muted)} />
    </Glyph>
  );
}
