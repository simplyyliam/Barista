import { Coffee03Icon } from "@barista/ui/icons";
import { motion } from "motion/react";
import { type SVGProps } from "react";

/**
 * The Barista mark, top left: Hugeicons' coffee mug (Coffee03) in a 14x17 box, with the cup body
 * filled. Scaled 0.7 and centred, so it sits optically next to the 13px bold name.
 *
 * The three wisps of steam lift and fade once, one after the other, when a parent motion element
 * (the pill) is hovered (variant "steam"). One short gesture, no loop.
 *
 * @example
 * <BrandLogo />
 */
export function BrandLogo(props: SVGProps<SVGSVGElement>) {
  return (
    <svg
      width="14"
      height="17"
      viewBox="0 0 14 17"
      fill="none"
      stroke="currentColor"
      strokeLinecap="round"
      strokeLinejoin="round"
      shapeRendering="geometricPrecision"
      aria-hidden="true"
      {...props}
    >
      {/* Stroke 2 in the scaled group is about 1.4px on screen. */}
      <g transform="translate(-1.4 0.1) scale(0.7)" strokeWidth={2}>
        {Coffee03Icon.map(([, attrs], i) =>
          // Elements 0 (handle) and 1 (cup body, filled) are still; 2 to 4 are the steam.
          i < 2 ? (
            <path key={i} d={String(attrs.d)} fill={i === 1 ? "currentColor" : "none"} />
          ) : (
            <motion.path
              key={i}
              d={String(attrs.d)}
              variants={{
                steam: {
                  y: [0, -1.6, 0],
                  opacity: [1, 0.35, 1],
                  transition: { duration: 0.55, ease: "easeInOut", delay: (i - 2) * 0.07 },
                },
              }}
            />
          )
        )}
      </g>
    </svg>
  );
}
