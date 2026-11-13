import { MenubarItem } from "./menubar-item";

/** The app name next to the logo, bold like macOS. Later: the name of the app in front. */
export function AppName({ open }: { open?: boolean }) {
  return (
    <MenubarItem variant="app" open={open} className="font-bar-strong">
      Barista
    </MenubarItem>
  );
}
