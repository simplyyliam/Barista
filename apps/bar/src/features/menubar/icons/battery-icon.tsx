import { type SVGProps } from "react";

type BatteryIconProps = Omit<SVGProps<SVGSVGElement>, "children"> & {
  /** 0 to 100. `null` = unknown: only the outline is drawn. */
  percent: number | null;
  charging?: boolean;
  pluggedIn?: boolean;
};

// The battery drawing of the former examples/battery item, kept as it was: a rounded outline with
// the terminal in one path (24 x 24 grid), a charge fill inset from the outline and a bolt while
// charging. Only the colours changed: the outline and fill are `currentColor`, so the icon follows
// the bar's light / dark tone (the example was white for a dark bar).
const BATTERY_PATH =
  "M8 18H16.5C16.9644 18 17.1966 18 17.3916 17.9743C18.7378 17.7971 19.7971 16.7378 19.9743 15.3916C20 15.1966 20 14.9644 20 14.5C21.1046 14.5 22 13.6046 22 12.5V11.5C22 10.3954 21.1046 9.5 20 9.5C20 9.03558 20 8.80337 19.9743 8.60842C19.7971 7.26222 18.7378 6.2029 17.3916 6.02567C17.1966 6 16.9644 6 16.5 6H8C5.17157 6 3.75736 6 2.87868 6.87868C2 7.75736 2 9.17157 2 12C2 14.8284 2 16.2426 2.87868 17.1213C3.75736 18 5.17157 18 8 18Z";

// Fill: inset from the outline, stopping before the terminal.
const FILL = { x: 3.6, y: 7.6, maxWidth: 14.8, height: 8.8, radius: 1.5 };
const CENTER_X = 11;
const LOW = "#ef4444"; // red at 15 % and below
const MARK_OUTLINE = "#1c1c1e";

// The drawing's bounds with its 1.5 stroke (what fitToGlyph cropped to): 21.5 x 13.5 around the
// path. Drawn 12px high, like WIDE_ICON_HEIGHT, so the glyph is about 19px wide.
const VIEW_BOX = "1.25 5.25 21.5 13.5";

/**
 * The battery: outline, a fill that follows the level (red at 15 % or less) and a bolt while
 * charging.
 *
 * @example
 * <BatteryIcon percent={72} charging />
 */
export function BatteryIcon({ percent, charging = false, pluggedIn = false, ...props }: BatteryIconProps) {
  void pluggedIn; // the example colours plugged-in fills white; with currentColor that is the default
  const level = percent === null ? 0 : Math.min(100, Math.max(0, Math.round(percent)));
  const fillColor = level <= 15 && percent !== null ? LOW : "currentColor";
  // Keep a visible rounded dot at very low non-zero levels.
  const fillWidth = level > 0 ? Math.max(FILL.radius * 2, (FILL.maxWidth * level) / 100) : 0;
  const c = CENTER_X;

  return (
    <svg
      viewBox={VIEW_BOX}
      height="12"
      shapeRendering="geometricPrecision"
      role="img"
      aria-label={percent === null ? "Battery" : `Battery ${level}%${charging ? ", charging" : ""}`}
      {...props}
    >
      <path
        d={BATTERY_PATH}
        fill="none"
        stroke="currentColor"
        strokeWidth="1.5"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
      {fillWidth > 0 && (
        <rect
          x={FILL.x}
          y={FILL.y}
          width={fillWidth.toFixed(2)}
          height={FILL.height}
          rx={FILL.radius}
          fill={fillColor}
        />
      )}
      {charging && (
        <path
          d={`M${c + 1.8} 7.2 L${c - 4.2} 12 H${c - 0.5} L${c - 1.8} 16.6 L${c + 4.6} 11 H${c + 0.7} Z`}
          fill="#ffffff"
          stroke={MARK_OUTLINE}
          strokeWidth="1"
          strokeLinecap="round"
          strokeLinejoin="round"
        />
      )}
    </svg>
  );
}
