import { useAppMenus } from "@/shared/hooks/use-app-menus";
import { MenubarItem } from "./menubar-item";

/** The name of the app in front next to the logo, bold like macOS. "Barista" on the desktop. */
export function AppName({ open }: { open?: boolean }) {
  const name = useAppMenus()?.appName;
  return (
    <MenubarItem variant="app" open={open} className="font-bar-strong">
      {name || "Barista"}
    </MenubarItem>
  );
}
