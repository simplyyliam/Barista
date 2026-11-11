
import { cn } from "cn";
import { type HTMLAttributes } from "react";

export function MenubarItem({
  children,
  className,
  ...props
}: HTMLAttributes<HTMLDivElement>) {
  return (
    <div
      className={cn(
        "flex items-center justify-center h-bar-item-h px-bar-item-x",
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
    <div
      className={cn(
        "flex items-center justify-center",
        className
      )}
      {...props}
    >
      {children}
    </div>
  );
}