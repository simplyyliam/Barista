import { cn } from "cn";
import { motion, type HTMLMotionProps } from "motion/react";
import { useEffect, useState, type HTMLAttributes, type ReactNode } from "react";
import { ITEM_ENTER, ITEM_HIDDEN, ITEM_SHOWN, PRESS, PRESS_TRANSITION, SLIDE } from "@/shared/motion";
import { useHoverEpoch } from "@/shared/stores/hover";

/** What a pill does when it first appears (the battery, Wi-Fi and volume arrive once their data does). */
export const ITEM_APPEAR = {
  initial: ITEM_HIDDEN,
  animate: { ...ITEM_SHOWN, transition: ITEM_ENTER },
};

type MenubarItemProps = HTMLMotionProps<"div"> & {
  /** "app" = left side (wider padding, pills touch); "status" = right side with text (clock, labels); "icon" = icon only. */
  variant?: "app" | "status" | "icon";
  /** Shows the pill as pressed while its menu or panel is open. */
  open?: boolean;
};

/**
 * One item of the bar, drawn as a pill like macOS: 4px shorter than the bar, 4px radius.
 *
 * Feel (ADR 0023): the tint comes in at once on hover (macOS does, and the bar is used hundreds of
 * times a day) and fades out in 150ms; a press dips the pill to 0.96 and springs back; when a
 * neighbour appears or leaves, the pills glide instead of jumping. Hover comes from pointer events
 * (not `:hover`) so it can be cleared when the pointer left for another window (shared/stores/hover.ts). The hit area reaches the top and
 * bottom edge of the bar, so slamming the mouse to the top of the screen still hits the item.
 * Sizes come from the --menubar-* tokens (styles/tokens.css).
 */
export function MenubarItem({
  children,
  className,
  variant = "status",
  open = false,
  transition,
  onPointerEnter,
  onPointerMove,
  onPointerLeave,
  ...props
}: MenubarItemProps) {
  const [hovered, setHovered] = useState(false);
  const epoch = useHoverEpoch();
  // A window opened or closed: the pointer may be elsewhere now, so forget the hover.
  useEffect(() => setHovered(false), [epoch]);

  return (
    <motion.div
      layout="position"
      whileTap={PRESS}
      transition={{ ...PRESS_TRANSITION, layout: SLIDE, ...transition }}
      className={cn(
        "group relative isolate flex h-menubar-pill shrink-0 items-center justify-center rounded-menubar-pill",
        // Hit area: the 2px above and below the pill belong to it (the bar is 4px taller).
        "before:absolute before:inset-x-0 before:-inset-y-0.5",
        variant === "app" ? "px-menubar-app-x" : variant === "icon" ? "px-menubar-icon-x" : "px-menubar-status-x",
        className
      )}
      data-hover={hovered || undefined}
      onPointerEnter={(e) => {
        setHovered(true);
        onPointerEnter?.(e);
      }}
      onPointerMove={(e) => {
        if (!hovered) setHovered(true);
        onPointerMove?.(e);
      }}
      onPointerLeave={(e) => {
        setHovered(false);
        onPointerLeave?.(e);
      }}
      {...props}
    >
      <span
        aria-hidden="true"
        className={cn(
          "pointer-events-none absolute inset-0 -z-10 rounded-menubar-pill bg-bar-hover opacity-0 transition-opacity duration-150 ease-out",
          "group-data-[hover=true]:opacity-100 group-data-[hover=true]:duration-0",
          open && "opacity-100 duration-0"
        )}
      />
      {children as ReactNode}
    </motion.div>
  );
}

export function MenubarItemGroup({
  children,
  className,
  ...props
}: HTMLAttributes<HTMLDivElement>) {
  return (
    <div className={cn("flex items-center justify-center", className)} {...props}>
      {children}
    </div>
  );
}
