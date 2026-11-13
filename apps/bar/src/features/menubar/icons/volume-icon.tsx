import { type SVGProps } from "react";
import { Glyph } from "./glyph";

type VolumeIconProps = SVGProps<SVGSVGElement> & {
  /** 0 to 1, as the volume service reports it. */
  level?: number;
  muted?: boolean;
};

// Waves around (10.4, 11), opening 90 degrees to the right: small, medium, large.
const WAVES = [
  "M12.38 9.02A2.8 2.8 0 0 1 12.38 12.98",
  "M14.08 7.32A5.2 5.2 0 0 1 14.08 14.68",
  "M15.77 5.63A7.6 7.6 0 0 1 15.77 16.37",
];

/** How many of the three waves a level lights: none at 0, then a third each. */
export function waveCount(level: number) {
  if (!(level > 0)) return 0;
  return Math.min(3, Math.ceil(level * 3));
}

/**
 * Speaker with 0 to 3 waves (level), or crossed out when muted.
 *
 * @example
 * <VolumeIcon level={0.5} muted={false} />
 */
export function VolumeIcon({ level = 1, muted = false, ...props }: VolumeIconProps) {
  const waves = muted ? 0 : waveCount(level);
  return (
    <Glyph {...props}>
      <path d="M4 8.6H6.6L10.6 5.2V16.8L6.6 13.4H4Z" />
      {WAVES.slice(0, waves).map((d) => (
        <path key={d} d={d} />
      ))}
      {muted && <path d="M14 8.8L18.4 13.2M18.4 8.8L14 13.2" />}
    </Glyph>
  );
}
