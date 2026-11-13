import { Coffee03Icon } from "@barista/ui/icons";
import { type SVGProps } from "react";

/**
 * The Barista mark, top left: Hugeicons' coffee mug (Coffee03) in a 14x17 box, with the cup body
 * filled. Scaled 0.7 and centred, so it sits optically next to the 13px bold name.
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
        {Coffee03Icon.map(([, attrs], i) => (
          // Element 1 is the closed cup body: fill it. The rest (handle, steam) stay lines.
          <path key={i} d={String(attrs.d)} fill={i === 1 ? "currentColor" : "none"} />
        ))}
      </g>
    </svg>
  );
}
