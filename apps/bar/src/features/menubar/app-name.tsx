import { MenubarItem } from "./menubar-item";

/** The app name next to the logo, bold like macOS. Later: the name of the app in front. */
export function AppName() {
  return (
    <MenubarItem variant="app" className="font-bar-strong">
      Barista
    </MenubarItem>
  );
}
