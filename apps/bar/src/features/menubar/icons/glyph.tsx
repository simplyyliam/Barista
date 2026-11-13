import { cn } from "cn";
import { type SVGProps } from "react";

/**
 * The shared frame of the bar's line icons: a 22x22 box (--menubar-icon-box) with the glyph drawn
 * at 15-16px inside it, 1.5px stroke, round caps and joins (SF Symbols weight). `currentColor`,
 * so the icon follows the adaptive tone. The viewBox scales with the token.
 */
export function Glyph({ className, children, ...props }: SVGProps<SVGSVGElement>) {
  return (
    <svg
      viewBox="0 0 22 22"
      fill="none"
      stroke="currentColor"
      strokeWidth={1.5}
      strokeLinecap="round"
      strokeLinejoin="round"
      shapeRendering="geometricPrecision"
      aria-hidden="true"
      className={cn("size-menubar-icon shrink-0", className)}
      {...props}
    >
      {children}
    </svg>
  );
}

/** An id that is safe inside `url(#...)` (useId() returns colons). */
export const safeId = (id: string) => id.replace(/[^a-zA-Z0-9_-]/g, "");
