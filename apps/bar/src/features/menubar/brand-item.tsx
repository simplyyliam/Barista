import { BrandLogo } from "./icons";
import { MenubarItem } from "./menubar-item";

/** The Barista logo, first item on the left. */
export function BrandItem({ open }: { open?: boolean }) {
  return (
    <MenubarItem variant="app" open={open} aria-label="Barista" whileHover="steam">
      <BrandLogo />
    </MenubarItem>
  );
}
