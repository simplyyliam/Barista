import { BrandLogo } from "./icons";
import { MenubarItem } from "./menubar-item";

/** The Barista logo, first item on the left. */
export function BrandItem() {
  return (
    <MenubarItem variant="app" aria-label="Barista">
      <BrandLogo />
    </MenubarItem>
  );
}
