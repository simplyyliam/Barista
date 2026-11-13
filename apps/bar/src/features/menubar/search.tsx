import { SearchIcon } from "./icons"
import { MenubarItem } from "./menubar-item"

/** Search icon. No action yet: the bar has no search to open. */
export function Search() {
  return (
    <MenubarItem className="cursor-default" aria-label="Search">
      <SearchIcon />
    </MenubarItem>
  )
}
