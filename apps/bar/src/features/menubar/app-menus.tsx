import { commands } from "@/shared/bindings";
import { useAppMenus, useOpenMenuKey } from "@/shared/hooks/use-app-menus";
import { useItemBounds } from "./hooks/use-item-bounds";
import { MenubarItem } from "./menubar-item";

/**
 * The menus of the app in front (File, Edit, View ...), next to its name. Rust reads them from the app
 * (ADR 0036) and the bar's own menu window draws them; a click is an `itemClicked` on the title's key,
 * like any other item. Apps that have no readable menu show none.
 */
export function AppMenus() {
  const menus = useAppMenus();
  const openKey = useOpenMenuKey();
  const titles = menus?.titles ?? [];

  // Menus open under their title, so Rust needs to know where each one is drawn.
  useItemBounds(titles.map((title) => `${title.key}:${title.label}`).join("|"));

  return (
    <>
      {titles.map((title) => (
        <MenubarItem
          key={title.key}
          variant="app"
          open={openKey === title.key}
          data-item-id={title.key}
          // With a menu open, moving onto another title switches menus without a click.
          onMouseEnter={() => void commands.menuHover(title.key)}
          onClick={() => void commands.itemClicked(title.key, "left")}
        >
          {title.label}
        </MenubarItem>
      ))}
    </>
  );
}
