import { cn } from "cn";
import { type HTMLAttributes } from "react";

type MenubarItemProps = HTMLAttributes<HTMLDivElement> & {
  /** "app" = left side (wider padding, pills touch); "status" = right side (icons, clock). */
  variant?: "app" | "status";
  /** Shows the pill as pressed while its menu or panel is open. */
  open?: boolean;
};

/**
 * One item of the bar, drawn as a pill like macOS: 4px shorter than the bar, 4px radius, and a
 * tint on hover or while open. No transition: macOS switches instantly.
 * Sizes come from the --menubar-* tokens (styles/tokens.css).
 */
export function MenubarItem({
  children,
  className,
  variant = "status",
  open = false,
  ...props
}: MenubarItemProps) {
  return (
    <div
      className={cn(
        "flex h-menubar-pill shrink-0 items-center justify-center rounded-menubar-pill transition-none hover:bg-bar-hover",
        variant === "app" ? "px-menubar-app-x" : "px-menubar-status-x",
        open && "bg-bar-hover",
        className
      )}
      {...props}
    >
      {children}
    </div>
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
