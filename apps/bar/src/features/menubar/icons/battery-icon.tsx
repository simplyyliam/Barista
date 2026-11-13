import { useId, type SVGProps } from "react";
import { safeId } from "./glyph";

type BatteryIconProps = Omit<SVGProps<SVGSVGElement>, "children"> & {
  /** 0 to 100. `null` = unknown: only the outline is drawn. */
  percent: number | null;
  /** Charging: an 8x10 bolt cuts through the fill. */
  charging?: boolean;
  /** Not charging: a bold 9px percentage sits inside the shell. */
  showPercent?: boolean;
};

// All numbers are px in the 27x12 viewBox. Body 25x12 (outer edge), 1.25 stroke, nub 1.5x4.
const STROKE = 1.25;
const FILL_X = 2; // stroke + 0.75 gap
const FILL_W = 21;
const BOLT = "M5.5 0L0 6H3.5L2.5 10L8 3.5H4.5Z"; // 8x10
const BOLT_AT = "translate(8.5 1)";

/**
 * The battery: outline, nub and a fill that follows the level. Charging or the percentage is
 * knocked out of the fill with an SVG mask; over the empty part it is drawn in the icon colour,
 * so it never disappears.
 *
 * @example
 * <BatteryIcon percent={72} charging />
 */
export function BatteryIcon({ percent, charging = false, showPercent = false, ...props }: BatteryIconProps) {
  const id = safeId(useId());
  const level = percent === null ? 0 : Math.min(100, Math.max(0, percent));
  const fillW = (FILL_W * level) / 100;
  const fillEnd = FILL_X + fillW;
  const text = !charging && showPercent && percent !== null ? String(Math.round(level)) : null;

  // The same shape twice: black in the fill's mask, `currentColor` over the empty part.
  const mark = (paint: string) =>
    charging ? (
      <path d={BOLT} transform={BOLT_AT} fill={paint} stroke={paint} strokeWidth={0.6} strokeLinejoin="round" />
    ) : text ? (
      <text
        x="12.5"
        y="6.4"
        textAnchor="middle"
        dominantBaseline="central"
        fontSize="9"
        fontWeight="700"
        fill={paint}
        stroke={paint}
        strokeWidth={0.4}
        style={{ fontVariantNumeric: "tabular-nums", letterSpacing: 0 }}
      >
        {text}
      </text>
    ) : null;

  return (
    <svg
      width="27"
      height="12"
      viewBox="0 0 27 12"
      shapeRendering="geometricPrecision"
      role="img"
      aria-label={percent === null ? "Battery" : `Battery ${Math.round(level)}%${charging ? ", charging" : ""}`}
      {...props}
    >
      <mask id={`${id}-fill`} maskUnits="userSpaceOnUse" x="0" y="0" width="27" height="12">
        <rect width="27" height="12" fill="white" />
        {mark("black")}
      </mask>
      <clipPath id={`${id}-empty`}>
        <rect x={fillEnd} y="0" width={27 - fillEnd} height="12" />
      </clipPath>

      <rect
        x={STROKE / 2}
        y={STROKE / 2}
        width={25 - STROKE}
        height={12 - STROKE}
        rx={3 - STROKE / 2}
        fill="none"
        stroke="currentColor"
        strokeWidth={STROKE}
        opacity={0.5}
      />
      <rect x="25.5" y="4" width="1.5" height="4" rx="0.75" fill="currentColor" opacity={0.5} />
      {fillW > 0 && (
        <rect
          x={FILL_X}
          y={FILL_X}
          width={fillW}
          height={12 - FILL_X * 2}
          rx="1.5"
          fill="currentColor"
          mask={`url(#${id}-fill)`}
        />
      )}
      <g clipPath={`url(#${id}-empty)`}>{mark("currentColor")}</g>
    </svg>
  );
}
