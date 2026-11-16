import { commands } from "@/shared/bindings";
import { BrandLogo } from "./icons";
import { MenubarItem } from "./menubar-item";

/** The Barista logo, first item on the left. Click opens the settings window (a real menu comes later). */
export function BrandItem({ open }: { open?: boolean }) {
  return (
    <MenubarItem
      variant="app"
      open={open}
      role="button"
      tabIndex={0}
      aria-label="Barista settings"
      whileHover="steam"
      onClick={() => void commands.openSettingsWindow()}
      onKeyDown={(e) => {
        if (e.key === "Enter" || e.key === " ") {
          e.preventDefault();
          void commands.openSettingsWindow();
        }
      }}
      className="cursor-default outline-none focus-visible:bg-bar-hover"
    >
      <BrandLogo />
    </MenubarItem>
  );
}
